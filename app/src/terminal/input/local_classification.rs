//! Runs the bundled input classifier on the terminal input independently of AI availability, so
//! its prediction can be shown in the input and recorded as training data.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use ai_terminal_completer::ParsedTokensSnapshot;
use ai_terminal_ui::r#async::SpawnedFutureHandle;
use ai_terminal_ui::{Entity, ModelContext, ModelHandle, SingletonEntity};
use anyhow::Context as _;
use async_channel::Sender;
use chrono::Local;
use input_classifier::{InputClassifier, InputClassifierDecisionSource, InputType};
use serde::Serialize;

use super::buffer_model::InputBufferModel;
use crate::debounce;
use crate::input_classifier::InputClassifierModel;
use crate::settings::{InputSettings, InputSettingsChangedEvent};

const CLASSIFICATION_DEBOUNCE_PERIOD: Duration = Duration::from_millis(150);

const SAMPLES_FILE_NAME: &str = "input_classifier_samples.jsonl";

/// Shown in the terminal input message bar in place of a prediction while the input is empty.
pub const PLACEHOLDER_HINT_TEXT: &str = "Local classifier: waiting for input";

/// Returns the path of the JSON Lines file that submitted inputs are appended to.
pub fn samples_file_path() -> PathBuf {
    ai_terminal_core::paths::secure_state_dir()
        .unwrap_or_else(ai_terminal_core::paths::state_dir)
        .join(SAMPLES_FILE_NAME)
}

/// The input classifier's prediction for a specific input buffer.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalInputClassification {
    buffer_text: String,
    p_ai: f32,
    p_shell: f32,
    source: InputClassifierDecisionSource,
}

impl LocalInputClassification {
    fn predicted_input_type(&self) -> InputType {
        if self.p_shell > self.p_ai {
            InputType::Shell
        } else {
            InputType::AI
        }
    }

    pub fn is_natural_language(&self) -> bool {
        self.predicted_input_type().is_ai()
    }

    fn is_model_prediction(&self) -> bool {
        match self.source {
            InputClassifierDecisionSource::InputClassifier => true,
            InputClassifierDecisionSource::InputClassifierFallbackHeuristic
            | InputClassifierDecisionSource::InputClassifierFallbackCurrentInput
            | InputClassifierDecisionSource::NaturalLanguageOneOffAllowlist
            | InputClassifierDecisionSource::ShellCommandAllowList
            | InputClassifierDecisionSource::ShellHeuristic => false,
        }
    }

    /// A short description of the prediction for display in the terminal input message bar.
    pub fn hint_text(&self) -> String {
        let classifier = if self.is_model_prediction() {
            "Local model"
        } else {
            "Local heuristics"
        };
        let (label, probability) = match self.predicted_input_type() {
            InputType::AI => ("natural language", self.p_ai),
            InputType::Shell => ("shell command", self.p_shell),
        };
        format!("{classifier}: {label} ({:.0}%)", probability * 100.)
    }
}

/// Classifies the terminal input buffer with the bundled input classifier and records submitted
/// inputs, as configured in [`InputSettings`].
pub struct LocalInputClassificationModel {
    buffer_model: ModelHandle<InputBufferModel>,
    latest_classification: Option<LocalInputClassification>,
    classification_handle: Option<SpawnedFutureHandle>,
    debounce_tx: Sender<()>,
}

impl LocalInputClassificationModel {
    pub fn new(buffer_model: ModelHandle<InputBufferModel>, ctx: &mut ModelContext<Self>) -> Self {
        let (debounce_tx, debounce_rx) = async_channel::unbounded();
        let _ = ctx.spawn_stream_local(
            debounce(CLASSIFICATION_DEBOUNCE_PERIOD, debounce_rx),
            |me, _, ctx| me.classify_buffer(ctx),
            |_, _| {},
        );

        ctx.subscribe_to_model(&buffer_model, |me, _, _, ctx| {
            if !InputSettings::as_ref(ctx).is_local_input_classification_hint_enabled() {
                return;
            }
            // Clearing is immediate so a submitted input's prediction doesn't linger under the
            // emptied input, while edits are debounced and keep showing the previous prediction.
            if me
                .buffer_model
                .as_ref(ctx)
                .current_value()
                .trim()
                .is_empty()
            {
                me.classify_buffer(ctx);
            } else {
                let _ = me.debounce_tx.try_send(());
            }
        });
        ctx.subscribe_to_model(&InputSettings::handle(ctx), |me, _, event, ctx| {
            if matches!(
                event,
                InputSettingsChangedEvent::ShowLocalInputClassification { .. }
            ) {
                me.classify_buffer(ctx);
            }
        });

        Self {
            buffer_model,
            latest_classification: None,
            classification_handle: None,
            debounce_tx,
        }
    }

    /// Returns the most recent classification, which may lag behind the buffer while the input is
    /// being edited.
    pub fn latest_classification(&self) -> Option<&LocalInputClassification> {
        self.latest_classification.as_ref()
    }

    fn classify_buffer(&mut self, ctx: &mut ModelContext<Self>) {
        if let Some(handle) = self.classification_handle.take() {
            handle.abort();
        }

        let buffer_text = self.buffer_model.as_ref(ctx).current_value().to_owned();
        if buffer_text.trim().is_empty()
            || !InputSettings::as_ref(ctx).is_local_input_classification_hint_enabled()
        {
            self.set_latest_classification(None, ctx);
            return;
        }

        let classifier = InputClassifierModel::as_ref(ctx).classifier();
        self.classification_handle = Some(ctx.spawn(
            classify(classifier, buffer_text),
            |me, classification, ctx| match classification {
                Ok(classification) => me.set_latest_classification(Some(classification), ctx),
                Err(err) => log::debug!("Local input classification failed: {err:#}"),
            },
        ));
    }

    fn set_latest_classification(
        &mut self,
        classification: Option<LocalInputClassification>,
        ctx: &mut ModelContext<Self>,
    ) {
        if self.latest_classification != classification {
            self.latest_classification = classification;
            ctx.emit(());
        }
    }

    /// Appends `buffer_text` and the classifier's prediction for it to [`samples_file_path`], if
    /// recording is enabled.
    pub fn record_submission(
        &self,
        buffer_text: String,
        submitted_as: InputType,
        ctx: &mut ModelContext<Self>,
    ) {
        if buffer_text.trim().is_empty()
            || !InputSettings::as_ref(ctx).is_local_input_classification_log_enabled()
        {
            return;
        }

        let classifier = InputClassifierModel::as_ref(ctx).classifier();
        let timestamp = Local::now().to_rfc3339();
        let _ = ctx.spawn(
            async move {
                let classification = match classify(classifier, buffer_text).await {
                    Ok(classification) => classification,
                    Err(err) => {
                        log::warn!("Skipped recording an input classifier sample: {err:#}");
                        return Ok(());
                    }
                };
                append_sample(
                    &samples_file_path(),
                    &classification,
                    submitted_as,
                    timestamp,
                )
            },
            |_, result, _| {
                if let Err(err) = result {
                    log::error!("Failed to record local input classification sample: {err:#}");
                }
            },
        );
    }
}

impl Entity for LocalInputClassificationModel {
    type Event = ();
}

async fn classify(
    classifier: Arc<dyn InputClassifier>,
    buffer_text: String,
) -> anyhow::Result<LocalInputClassification> {
    // The model only reads the raw text; parsed tokens would only refine the heuristic fallback.
    let input = ParsedTokensSnapshot {
        buffer_text: buffer_text.clone(),
        parsed_tokens: Vec::new(),
    };
    let context = input_classifier::Context {
        current_input_type: InputType::Shell,
        is_agent_follow_up: false,
    };
    let result = classifier.classify_input(input, &context).await?;
    Ok(LocalInputClassification {
        buffer_text,
        p_ai: result.p_ai(),
        p_shell: result.p_shell(),
        source: result.source,
    })
}

#[derive(Serialize)]
struct Sample<'a> {
    timestamp: String,
    text: &'a str,
    submitted_as: InputType,
    predicted: InputType,
    p_ai: f32,
    p_shell: f32,
    source: InputClassifierDecisionSource,
}

fn sample_json_line(
    classification: &LocalInputClassification,
    submitted_as: InputType,
    timestamp: String,
) -> serde_json::Result<String> {
    let mut line = serde_json::to_string(&Sample {
        timestamp,
        text: &classification.buffer_text,
        submitted_as,
        predicted: classification.predicted_input_type(),
        p_ai: classification.p_ai,
        p_shell: classification.p_shell,
        source: classification.source,
    })?;
    line.push('\n');
    Ok(line)
}

fn append_sample(
    path: &Path,
    classification: &LocalInputClassification,
    submitted_as: InputType,
    timestamp: String,
) -> anyhow::Result<()> {
    let line = sample_json_line(classification, submitted_as, timestamp)
        .context("Failed to serialize input classifier sample")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create input classifier samples directory")?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .context("Failed to open input classifier samples file")?;
    file.write_all(line.as_bytes())
        .context("Failed to write input classifier sample")
}

#[cfg(test)]
#[path = "local_classification_tests.rs"]
mod tests;

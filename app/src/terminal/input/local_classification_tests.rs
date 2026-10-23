use ai_terminal_ui::{App, ModelHandle, SingletonEntity as _, ViewHandle};
use input_classifier::{InputClassifierDecisionSource, InputType};
use settings::Setting as _;

use super::{
    LocalInputClassification, LocalInputClassificationModel, append_sample, sample_json_line,
};
use crate::input_classifier::InputClassifierModel;
use crate::settings::InputSettings;
use crate::terminal::input::Input;
use crate::terminal::input::tests::{add_window_with_bootstrapped_terminal, initialize_app};
use crate::test_util::assert_eventually;

fn set_hint_enabled(enabled: bool, app: &mut App) {
    InputSettings::handle(app).update(app, |settings, ctx| {
        settings
            .show_local_input_classification
            .set_value(enabled, ctx)
            .unwrap();
    });
}

async fn add_input_with_classification_model(
    app: &mut App,
) -> (
    ViewHandle<Input>,
    ModelHandle<LocalInputClassificationModel>,
) {
    initialize_app(app);
    app.add_singleton_model(InputClassifierModel::new);

    let terminal =
        add_window_with_bootstrapped_terminal(app, None /* history_file_commands */, None).await;
    let input = terminal.read(app, |terminal, _| terminal.input().clone());
    let model = input.read(app, |input, _| input.local_classification_model.clone());
    (input, model)
}

fn latest_buffer_text(
    model: &ModelHandle<LocalInputClassificationModel>,
    app: &App,
) -> Option<String> {
    model.read(app, |model, _| {
        model
            .latest_classification()
            .map(|classification| classification.buffer_text.clone())
    })
}

#[test]
fn hint_classifies_input_and_clears_immediately_when_input_is_emptied() {
    App::test((), |mut app| async move {
        let (input, model) = add_input_with_classification_model(&mut app).await;
        set_hint_enabled(true, &mut app);

        input.update(&mut app, |input, ctx| {
            input.user_insert("how do I list all files in this directory", ctx);
        });
        assert_eventually!(
            200 => latest_buffer_text(&model, &app).as_deref()
                == Some("how do I list all files in this directory"),
            "Expected the input buffer to be classified"
        );

        input.update(&mut app, |input, ctx| {
            input.clear_buffer_and_reset_undo_stack(ctx);
        });
        assert_eq!(latest_buffer_text(&model, &app), None);
    });
}

#[test]
fn toggling_hint_classifies_existing_input_and_clears_on_disable() {
    App::test((), |mut app| async move {
        let (input, model) = add_input_with_classification_model(&mut app).await;

        input.update(&mut app, |input, ctx| {
            input.user_insert("git status", ctx);
        });
        assert_eq!(latest_buffer_text(&model, &app), None);

        set_hint_enabled(true, &mut app);
        assert_eventually!(
            200 => latest_buffer_text(&model, &app).as_deref() == Some("git status"),
            "Expected enabling the hint to classify the existing input"
        );

        set_hint_enabled(false, &mut app);
        assert_eq!(latest_buffer_text(&model, &app), None);
    });
}

#[test]
fn hint_text_reports_natural_language_with_model_probability() {
    let classification = LocalInputClassification {
        buffer_text: "how do I undo my last commit".to_owned(),
        p_ai: 0.75,
        p_shell: 0.25,
        source: InputClassifierDecisionSource::InputClassifier,
    };

    assert_eq!(
        classification.hint_text(),
        "Local model: natural language (75%)"
    );
}

#[test]
fn hint_text_reports_shell_command_with_model_probability() {
    let classification = LocalInputClassification {
        buffer_text: "git reset --soft HEAD~1".to_owned(),
        p_ai: 0.25,
        p_shell: 0.75,
        source: InputClassifierDecisionSource::InputClassifier,
    };

    assert_eq!(
        classification.hint_text(),
        "Local model: shell command (75%)"
    );
}

#[test]
fn hint_text_marks_heuristic_fallback_prediction() {
    let classification = LocalInputClassification {
        buffer_text: "what is my ip address".to_owned(),
        p_ai: 1.0,
        p_shell: 0.0,
        source: InputClassifierDecisionSource::InputClassifierFallbackHeuristic,
    };

    assert_eq!(
        classification.hint_text(),
        "Local heuristics: natural language (100%)"
    );
}

#[test]
fn tied_probabilities_predict_natural_language() {
    let classification = LocalInputClassification {
        buffer_text: "status".to_owned(),
        p_ai: 0.5,
        p_shell: 0.5,
        source: InputClassifierDecisionSource::InputClassifier,
    };

    assert!(classification.is_natural_language());
}

#[test]
fn sample_json_line_serializes_one_json_object_per_line() {
    let classification = LocalInputClassification {
        buffer_text: "列出 \"当前\" 目录".to_owned(),
        p_ai: 0.75,
        p_shell: 0.25,
        source: InputClassifierDecisionSource::InputClassifier,
    };

    let line = sample_json_line(
        &classification,
        InputType::Shell,
        "2026-10-23T12:00:00+08:00".to_owned(),
    )
    .unwrap();

    assert_eq!(
        line,
        "{\"timestamp\":\"2026-10-23T12:00:00+08:00\",\"text\":\"列出 \\\"当前\\\" 目录\",\
         \"submitted_as\":\"Shell\",\"predicted\":\"AI\",\"p_ai\":0.75,\"p_shell\":0.25,\
         \"source\":\"InputClassifier\"}\n"
    );
}

#[test]
fn append_sample_creates_missing_directories_and_appends_lines() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("state").join("samples.jsonl");
    let shell_sample = LocalInputClassification {
        buffer_text: "ls -la".to_owned(),
        p_ai: 0.25,
        p_shell: 0.75,
        source: InputClassifierDecisionSource::InputClassifier,
    };
    let ai_sample = LocalInputClassification {
        buffer_text: "list all files".to_owned(),
        p_ai: 0.75,
        p_shell: 0.25,
        source: InputClassifierDecisionSource::InputClassifier,
    };

    append_sample(&path, &shell_sample, InputType::Shell, "t1".to_owned()).unwrap();
    append_sample(&path, &ai_sample, InputType::AI, "t2".to_owned()).unwrap();

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "{\"timestamp\":\"t1\",\"text\":\"ls -la\",\"submitted_as\":\"Shell\",\
         \"predicted\":\"Shell\",\"p_ai\":0.25,\"p_shell\":0.75,\"source\":\"InputClassifier\"}\n\
         {\"timestamp\":\"t2\",\"text\":\"list all files\",\"submitted_as\":\"AI\",\
         \"predicted\":\"AI\",\"p_ai\":0.75,\"p_shell\":0.25,\"source\":\"InputClassifier\"}\n"
    );
}

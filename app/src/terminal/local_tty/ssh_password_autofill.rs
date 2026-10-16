//! Connects SSH password autofill to a local PTY by polling its terminal mode while an SSH
//! command logs in.

use std::borrow::Cow;
use std::os::fd::RawFd;
use std::sync::Arc;
use std::time::Duration;

use ai_terminal_core::SessionId;
use ai_terminal_ui::r#async::Timer;
use ai_terminal_ui::{AppContext, Entity, ModelContext, ModelHandle, SingletonEntity, ViewHandle};
use instant::Instant;
use nix::sys::termios::{self, LocalFlags, SpecialCharacterIndices, Termios};
use parking_lot::FairMutex;
use settings::Setting as _;

use super::mio_channel;
use crate::settings::SshSettings;
use crate::terminal::TerminalModel;
use crate::terminal::event::BlockType;
use crate::terminal::model::block::BlockId;
use crate::terminal::model::session::Sessions;
use crate::terminal::model::terminal_model::BlockIndex;
use crate::terminal::model_events::{ModelEvent, ModelEventDispatcher};
use crate::terminal::ssh::password_autofill::{
    LineEditingChars, LoginObservation, SavedSshPasswords, SshLoginPasswordTracker, SshLoginTarget,
    SshPasswordAutofillEvent, TrackerAction,
};
use crate::terminal::view::command_first_word_and_suffix;
use crate::terminal::writeable_pty::{Message, PtyIntent, PtyIntentEvent, TerminalSurface};

const POLL_INTERVAL: Duration = Duration::from_millis(100);
/// How long to keep polling without a password prompt, measured from when the command started or
/// last showed a hidden-input prompt.
const PROMPT_TIMEOUT: Duration = Duration::from_secs(180);
/// The number of rows read before the cursor, so that a prompt wrapped onto a second row is read
/// whole.
const PROMPT_ROWS: usize = 2;
/// The number of output rows searched for SSH's authentication failure when the command ends.
const FINAL_OUTPUT_ROWS: usize = 100;

struct TrackedLogin {
    block_id: BlockId,
    tracker: SshLoginPasswordTracker,
    stop_polling_at: Instant,
}

/// Enters saved passwords at the SSH password prompts of a local PTY, and saves the passwords
/// typed there once they log in.
pub struct SshPasswordAutofill {
    pty_fd: RawFd,
    model: Arc<FairMutex<TerminalModel>>,
    sessions: ModelHandle<Sessions>,
    event_loop_tx: mio_channel::Sender<Message>,
    login: Option<TrackedLogin>,
    /// Incremented to cancel the scheduled poll.
    poll_epoch: usize,
}

impl Entity for SshPasswordAutofill {
    type Event = SshPasswordAutofillEvent;
}

impl SshPasswordAutofill {
    fn new(
        pty_fd: RawFd,
        model_events: &ModelHandle<ModelEventDispatcher>,
        event_loop_tx: mio_channel::Sender<Message>,
        model: Arc<FairMutex<TerminalModel>>,
        sessions: ModelHandle<Sessions>,
        ctx: &mut ModelContext<Self>,
    ) -> Self {
        ctx.subscribe_to_model(model_events, |me, dispatcher, event, ctx| match event {
            ModelEvent::AfterBlockStarted {
                block_id,
                command,
                is_for_in_band_command: false,
            } => {
                let session_id = dispatcher.as_ref(ctx).active_session_id();
                me.on_block_started(block_id, command, session_id, ctx);
            }
            ModelEvent::AfterBlockCompleted(completed) => {
                if let BlockType::User(user) = &completed.block_type {
                    me.on_user_block_completed(user.index, user.was_part_of_agent_interaction, ctx);
                }
            }
            _ => {}
        });
        Self {
            pty_fd,
            model,
            sessions,
            event_loop_tx,
            login: None,
            poll_epoch: 0,
        }
    }

    fn is_tracking(&self) -> bool {
        self.login.is_some()
    }

    fn on_block_started(
        &mut self,
        block_id: &BlockId,
        command: &str,
        session_id: Option<SessionId>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.stop_tracking();
        let session = session_id.and_then(|session_id| self.sessions.as_ref(ctx).get(session_id));
        // Inside a remote session, the local terminal mode belongs to the outer SSH client.
        if session.as_ref().is_some_and(|session| !session.is_local()) {
            return;
        }
        let expanded_command = session.as_ref().and_then(|session| {
            let (first_word, rest) = command_first_word_and_suffix(command)?;
            let alias_value = session.alias_value(first_word)?;
            Some(format!("{alias_value}{rest}"))
        });
        let Some(target) = SshLoginTarget::parse(expanded_command.as_deref().unwrap_or(command))
        else {
            return;
        };
        if !*SshSettings::as_ref(ctx).remember_ssh_passwords.value() {
            return;
        }

        self.login = Some(TrackedLogin {
            block_id: block_id.clone(),
            tracker: SshLoginPasswordTracker::new(target),
            stop_polling_at: Instant::now() + PROMPT_TIMEOUT,
        });
        self.schedule_poll(self.poll_epoch, ctx);
    }

    fn on_user_block_completed(
        &mut self,
        index: BlockIndex,
        was_part_of_agent_interaction: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        let Some(mut login) = self.login.take() else {
            return;
        };
        self.stop_tracking();
        let (exit_code, output) = {
            let model = self.model.lock();
            let Some(block) = model
                .block_list()
                .block_at(index)
                .filter(|block| block.id() == &login.block_id)
            else {
                return;
            };
            if was_part_of_agent_interaction || model.shared_session_status().is_sharer_or_viewer()
            {
                return;
            }
            (
                block.exit_code(),
                block
                    .output_grid()
                    .contents_to_string_with_secrets_unobfuscated(false, Some(FINAL_OUTPUT_ROWS)),
            )
        };
        let action = login.tracker.on_command_finished(exit_code, &output);
        self.perform(action, ctx);
    }

    fn on_pty_intent(&mut self, intent: &PtyIntent) {
        let Some(login) = &mut self.login else {
            return;
        };
        match intent {
            PtyIntent::WriteBytes(bytes) => login.tracker.on_user_input(bytes),
            PtyIntent::WriteAgentInput { .. } | PtyIntent::CtrlD | PtyIntent::Interrupt => {
                login.tracker.on_untracked_input();
            }
            PtyIntent::ShutdownPty
            | PtyIntent::Resize(_)
            | PtyIntent::ExecuteCommand(_)
            | PtyIntent::RunNativeShellCompletions { .. } => {}
        }
    }

    fn stop_tracking(&mut self) {
        self.login = None;
        self.poll_epoch += 1;
    }

    fn schedule_poll(&self, epoch: usize, ctx: &mut ModelContext<Self>) {
        let pty_fd = self.pty_fd;
        ctx.spawn(
            async move {
                Timer::after(POLL_INTERVAL).await;
                termios::tcgetattr(pty_fd)
            },
            move |me, termios, ctx| me.on_poll(epoch, termios, ctx),
        );
    }

    fn on_poll(
        &mut self,
        epoch: usize,
        termios: nix::Result<Termios>,
        ctx: &mut ModelContext<Self>,
    ) {
        if epoch != self.poll_epoch {
            return;
        }
        let Some(observation) = termios.ok().and_then(|termios| self.observe(&termios)) else {
            self.stop_tracking();
            return;
        };
        let Some(login) = &mut self.login else {
            return;
        };
        if matches!(observation, LoginObservation::HiddenInput { .. }) {
            login.stop_polling_at = Instant::now() + PROMPT_TIMEOUT;
        }
        let actions = login.tracker.on_observation(observation, |key| {
            SavedSshPasswords::as_ref(ctx)
                .password(key, ctx)
                .map(str::to_owned)
        });
        let finished = login.tracker.is_finished();
        let keep_polling = Instant::now() < login.stop_polling_at;
        self.perform(actions, ctx);

        if finished {
            self.stop_tracking();
        } else if keep_polling {
            self.schedule_poll(epoch, ctx);
        }
    }

    /// Reads the state of the login from the terminal, or returns `None` if the login shouldn't be
    /// followed anymore.
    fn observe(&self, termios: &Termios) -> Option<LoginObservation> {
        if !termios.local_flags.contains(LocalFlags::ICANON) {
            return Some(LoginObservation::RawMode);
        }
        if termios.local_flags.contains(LocalFlags::ECHO) {
            return Some(LoginObservation::EchoedInput);
        }

        let login = self.login.as_ref()?;
        let model = self.model.lock();
        let block = model.block_list().active_block();
        // Others may type into a shared session, and an agent may drive the command, neither of
        // whom should be able to log in with the user's saved password.
        if block.id() != &login.block_id
            || block.is_agent_driving_command()
            || block.is_agent_in_control_or_tagged_in()
            || model.shared_session_status().is_sharer_or_viewer()
        {
            return None;
        }
        let grid = block.output_grid();
        let cursor = grid.grid_handler().cursor_point();
        let line = if cursor.col > 0 && cursor.row + 1 == grid.len() {
            grid.contents_to_string_with_secrets_unobfuscated(false, Some(PROMPT_ROWS))
                .rsplit('\n')
                .next()
                .unwrap_or_default()
                .to_owned()
        } else {
            String::new()
        };
        Some(LoginObservation::HiddenInput {
            line,
            row: grid.total_row_count(),
            editing: line_editing_chars(termios),
        })
    }

    fn perform(
        &mut self,
        actions: impl IntoIterator<Item = TrackerAction>,
        ctx: &mut ModelContext<Self>,
    ) {
        for action in actions {
            match action {
                TrackerAction::FillPassword { key, password } => {
                    let mut bytes = password.into_bytes();
                    bytes.push(b'\r');
                    if self
                        .event_loop_tx
                        .send(Message::Input(Cow::Owned(bytes)))
                        .is_ok()
                    {
                        ctx.emit(SshPasswordAutofillEvent::PasswordFilled(key));
                    }
                }
                TrackerAction::SavePassword { key, password } => {
                    let saved = SavedSshPasswords::handle(ctx).update(ctx, |passwords, ctx| {
                        passwords.save(key.clone(), password, ctx)
                    });
                    if saved {
                        ctx.emit(SshPasswordAutofillEvent::PasswordSaved(key));
                    }
                }
                TrackerAction::ForgetPassword { key } => {
                    let forgotten = SavedSshPasswords::handle(ctx)
                        .update(ctx, |passwords, ctx| passwords.forget(&key, ctx));
                    if forgotten {
                        ctx.emit(SshPasswordAutofillEvent::SavedPasswordRejected(key));
                    }
                }
            }
        }
    }
}

fn line_editing_chars(termios: &Termios) -> LineEditingChars {
    let control_char = |index: SpecialCharacterIndices| {
        let value = termios.control_chars[index as usize];
        // Linux disables a control character with 0, and macOS with 0xff.
        (value != 0 && value != 0xff).then_some(value)
    };
    LineEditingChars {
        erase: control_char(SpecialCharacterIndices::VERASE),
        kill: control_char(SpecialCharacterIndices::VKILL),
    }
}

/// Starts SSH password autofill for the local PTY behind `surface`.
///
/// The subscriptions hold weak handles, since strong ones would create reference cycles.
pub(super) fn wire_up_ssh_password_autofill<S: TerminalSurface>(
    pty_fd: RawFd,
    surface: &ViewHandle<S>,
    model_events: &ModelHandle<ModelEventDispatcher>,
    event_loop_tx: mio_channel::Sender<Message>,
    model: Arc<FairMutex<TerminalModel>>,
    sessions: ModelHandle<Sessions>,
    ctx: &mut AppContext,
) -> ModelHandle<SshPasswordAutofill>
where
    <S as Entity>::Event: PtyIntentEvent,
{
    let autofill = ctx.add_model(|ctx| {
        SshPasswordAutofill::new(pty_fd, model_events, event_loop_tx, model, sessions, ctx)
    });

    let autofill_for_input = autofill.downgrade();
    ctx.subscribe_to_view(surface, move |_surface, event, ctx| {
        let Some(autofill) = autofill_for_input.upgrade(ctx) else {
            return;
        };
        if !autofill.as_ref(ctx).is_tracking() {
            return;
        }
        let Some(intent) = event.pty_intent() else {
            return;
        };
        autofill.update(ctx, |autofill, _| autofill.on_pty_intent(&intent));
    });

    let surface_for_events = surface.downgrade();
    ctx.subscribe_to_model(&autofill, move |_autofill, event, ctx| {
        let Some(surface) = surface_for_events.upgrade(ctx) else {
            return;
        };
        surface.update(ctx, |surface, ctx| {
            surface.on_ssh_password_autofill_event(event, ctx);
        });
    });

    autofill
}

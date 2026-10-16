use std::collections::HashSet;
use std::mem;

use ai_terminal_core::command::ExitCode;

use super::SshCredentialKey;
use super::command::SshLoginTarget;
use super::prompt::PasswordPrompt;

/// The start of OpenSSH's message when it gives up on authentication, e.g.
/// `root@host: Permission denied (publickey,password).`
const AUTHENTICATION_FAILED_MESSAGE: &str = "Permission denied (";

/// A state of the terminal observed while an SSH command logs in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoginObservation {
    /// The terminal reads input line by line without echoing it, as when asking for a password.
    HiddenInput {
        /// The text before the cursor.
        line: String,
        /// Identifies the cursor's row, so that each prompt is handled once.
        row: u64,
        editing: LineEditingChars,
    },
    /// The terminal reads input line by line and echoes it.
    EchoedInput,
    /// The terminal passes input through unprocessed, as an interactive SSH session does.
    RawMode,
}

/// The terminal's line editing characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineEditingChars {
    /// Deletes the previous character.
    pub erase: Option<u8>,
    /// Deletes the whole line.
    pub kill: Option<u8>,
}

/// A change to saved passwords or to the terminal requested by the tracker.
// Debug output would expose passwords, so it's only derived for tests.
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum TrackerAction {
    /// Enter the saved password at the current prompt.
    FillPassword {
        key: SshCredentialKey,
        password: String,
    },
    /// Save the password that logged in to the account.
    SavePassword {
        key: SshCredentialKey,
        password: String,
    },
    /// Remove the saved password, which the server rejected.
    ForgetPassword { key: SshCredentialKey },
}

enum Phase {
    WaitingForPrompt,
    /// The saved password was entered at the current prompt.
    Filled {
        key: SshCredentialKey,
        /// Whether the user also typed after the prompt appeared, which would have changed the
        /// password the server received.
        user_typed: bool,
    },
    Typing(PromptInput),
    /// The user submitted a password, and the server hasn't accepted it yet.
    Submitted {
        key: SshCredentialKey,
        password: String,
        named_account: bool,
    },
    /// The login finished, so later prompts belong to the remote session.
    Finished,
}

/// What the user typed at a password prompt.
struct PromptInput {
    key: SshCredentialKey,
    /// Whether the prompt named the account it asked about.
    named_account: bool,
    editing: LineEditingChars,
    bytes: Vec<u8>,
    /// Whether `bytes` is known to match what the prompt received.
    trusted: bool,
}

impl PromptInput {
    fn edit(&mut self, byte: u8) {
        if Some(byte) == self.editing.erase {
            // The terminal erases either a byte or a whole UTF-8 character, depending on its
            // settings.
            if self.bytes.pop().is_some_and(|byte| !byte.is_ascii()) {
                self.trusted = false;
            }
        } else if Some(byte) == self.editing.kill {
            self.bytes.clear();
        } else if byte.is_ascii_control() {
            // Other control characters may edit the line in ways that aren't mirrored.
            self.trusted = false;
        } else {
            self.bytes.push(byte);
        }
    }

    fn submit(self) -> Phase {
        match String::from_utf8(self.bytes) {
            Ok(password) if self.trusted && !password.is_empty() => Phase::Submitted {
                key: self.key,
                password,
                named_account: self.named_account,
            },
            Ok(_) | Err(_) => Phase::WaitingForPrompt,
        }
    }
}

/// Follows the login of one SSH command to fill saved passwords and learn new ones.
pub struct SshLoginPasswordTracker {
    target: SshLoginTarget,
    phase: Phase,
    /// The accounts whose saved password was entered, so that a rejected password isn't entered
    /// again.
    filled_accounts: HashSet<SshCredentialKey>,
    /// The row and text of the last prompt handled.
    last_prompt: Option<(u64, String)>,
    /// Whether the user typed outside of a followed prompt since last submitting a line. Such
    /// input may reach the next prompt.
    typed_ahead: bool,
}

impl SshLoginPasswordTracker {
    pub fn new(target: SshLoginTarget) -> Self {
        Self {
            target,
            phase: Phase::WaitingForPrompt,
            filled_accounts: HashSet::new(),
            last_prompt: None,
            typed_ahead: false,
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.phase, Phase::Finished)
    }

    /// Updates the login from the terminal's state. `saved_password` returns the saved password
    /// of an account.
    pub fn on_observation(
        &mut self,
        observation: LoginObservation,
        saved_password: impl FnOnce(&SshCredentialKey) -> Option<String>,
    ) -> Vec<TrackerAction> {
        match observation {
            LoginObservation::HiddenInput { line, row, editing } => {
                self.on_hidden_input(&line, row, editing, saved_password)
            }
            LoginObservation::EchoedInput => {
                // Password prompts discard pending input when they hide it, so input typed while
                // the terminal echoes it can't reach the next prompt.
                self.typed_ahead = false;
                if matches!(self.phase, Phase::Typing(_)) {
                    self.phase = Phase::WaitingForPrompt;
                }
                Vec::new()
            }
            LoginObservation::RawMode => match mem::replace(&mut self.phase, Phase::Finished) {
                Phase::Submitted { key, password, .. } => {
                    vec![TrackerAction::SavePassword { key, password }]
                }
                Phase::WaitingForPrompt
                | Phase::Filled { .. }
                | Phase::Typing(_)
                | Phase::Finished => Vec::new(),
            },
        }
    }

    fn on_hidden_input(
        &mut self,
        line: &str,
        row: u64,
        editing: LineEditingChars,
        saved_password: impl FnOnce(&SshCredentialKey) -> Option<String>,
    ) -> Vec<TrackerAction> {
        if self.is_finished() {
            return Vec::new();
        }
        let Some(prompt) = PasswordPrompt::parse(line) else {
            return Vec::new();
        };
        let instance = (row, line.trim().to_owned());
        if self.last_prompt.as_ref() == Some(&instance) {
            return Vec::new();
        }
        self.last_prompt = Some(instance);

        let typed_ahead = mem::take(&mut self.typed_ahead);
        let key = self.target.credential_key(&prompt);
        let mut actions = Vec::new();
        match mem::replace(&mut self.phase, Phase::WaitingForPrompt) {
            // Being asked again for the same account means the server rejected the password.
            Phase::Filled {
                key: filled_key,
                user_typed,
            } => {
                if !user_typed && key.as_ref() == Some(&filled_key) {
                    actions.push(TrackerAction::ForgetPassword { key: filled_key });
                }
            }
            // Being asked for another account means the previous one logged in, e.g. to a jump
            // host. Only prompts that name their account tell the accounts apart.
            Phase::Submitted {
                key: submitted_key,
                password,
                named_account,
            } => {
                if named_account && prompt.account.is_some() && key.as_ref() != Some(&submitted_key)
                {
                    actions.push(TrackerAction::SavePassword {
                        key: submitted_key,
                        password,
                    });
                }
            }
            Phase::WaitingForPrompt | Phase::Typing(_) | Phase::Finished => {}
        }

        let Some(key) = key else {
            return actions;
        };
        if !typed_ahead
            && !self.filled_accounts.contains(&key)
            && let Some(password) = saved_password(&key)
        {
            self.filled_accounts.insert(key.clone());
            actions.push(TrackerAction::FillPassword {
                key: key.clone(),
                password,
            });
            self.phase = Phase::Filled {
                key,
                user_typed: false,
            };
        } else {
            self.phase = Phase::Typing(PromptInput {
                key,
                named_account: prompt.account.is_some(),
                editing,
                bytes: Vec::new(),
                trusted: !typed_ahead,
            });
        }
        actions
    }

    /// Mirrors input the user typed into the terminal.
    pub fn on_user_input(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            let line_end = byte == b'\r' || byte == b'\n';
            self.phase = match mem::replace(&mut self.phase, Phase::Finished) {
                Phase::Typing(input) if line_end => input.submit(),
                Phase::Typing(mut input) => {
                    input.edit(byte);
                    Phase::Typing(input)
                }
                Phase::Filled { key, .. } => {
                    self.typed_ahead = !line_end;
                    Phase::Filled {
                        key,
                        user_typed: true,
                    }
                }
                phase @ (Phase::WaitingForPrompt | Phase::Submitted { .. }) => {
                    self.typed_ahead = !line_end;
                    phase
                }
                Phase::Finished => Phase::Finished,
            };
        }
    }

    /// Notes input that reached the terminal without being mirrored, e.g. from an agent.
    pub fn on_untracked_input(&mut self) {
        match &mut self.phase {
            Phase::Typing(input) => input.trusted = false,
            Phase::Filled { user_typed, .. } => {
                *user_typed = true;
                self.typed_ahead = true;
            }
            Phase::WaitingForPrompt | Phase::Submitted { .. } => self.typed_ahead = true,
            Phase::Finished => {}
        }
    }

    /// Ends the login when its command finishes with `exit_code` and `output`.
    pub fn on_command_finished(
        &mut self,
        exit_code: ExitCode,
        output: &str,
    ) -> Option<TrackerAction> {
        let logged_in = self.target.exit_code_proves_login(exit_code);
        match mem::replace(&mut self.phase, Phase::Finished) {
            Phase::Submitted { key, password, .. } if logged_in => {
                Some(TrackerAction::SavePassword { key, password })
            }
            Phase::Filled {
                key,
                user_typed: false,
            } if !logged_in && self.output_shows_rejection(output) => {
                Some(TrackerAction::ForgetPassword { key })
            }
            Phase::WaitingForPrompt
            | Phase::Filled { .. }
            | Phase::Typing(_)
            | Phase::Submitted { .. }
            | Phase::Finished => None,
        }
    }

    /// Whether SSH gave up on authentication after the last prompt.
    fn output_shows_rejection(&self, output: &str) -> bool {
        let after_last_prompt = self
            .last_prompt
            .as_ref()
            .and_then(|(_, prompt)| output.rfind(prompt.as_str()))
            .map_or(output, |index| &output[index..]);
        after_last_prompt.contains(AUTHENTICATION_FAILED_MESSAGE)
    }
}

#[cfg(test)]
#[path = "tracker_tests.rs"]
mod tests;

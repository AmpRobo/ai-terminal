//! Remembers passwords typed at SSH login prompts and enters them on later logins to the same
//! account.

mod command;
mod prompt;
mod saved_passwords;
mod tracker;

use std::fmt;

pub use command::SshLoginTarget;
pub use saved_passwords::{SavedSshPasswords, SavedSshPasswordsEvent};
use serde::{Deserialize, Serialize};
pub use tracker::{LineEditingChars, LoginObservation, SshLoginPasswordTracker, TrackerAction};

const DEFAULT_SSH_PORT: u16 = 22;

/// The account a saved SSH password belongs to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SshCredentialKey {
    /// `None` when neither the command nor the prompt names the user, in which case SSH logs in
    /// as the user from the SSH config or the local user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    host: String,
    /// `None` for the default SSH port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
}

impl SshCredentialKey {
    pub fn new(user: Option<String>, host: &str, port: Option<u16>) -> Self {
        let host = host
            .strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
            .unwrap_or(host)
            .to_ascii_lowercase();
        Self {
            user: user.filter(|user| !user.is_empty()),
            host,
            port: port.filter(|port| *port != DEFAULT_SSH_PORT),
        }
    }
}

impl fmt::Display for SshCredentialKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(user) = &self.user {
            write!(f, "{user}@")?;
        }
        match self.port {
            Some(port) if self.host.contains(':') => write!(f, "[{}]:{port}", self.host),
            Some(port) => write!(f, "{}:{port}", self.host),
            None => write!(f, "{}", self.host),
        }
    }
}

/// A user-visible outcome of SSH password autofill in a terminal session.
#[derive(Clone, Debug)]
pub enum SshPasswordAutofillEvent {
    /// The saved password was entered at the login prompt.
    PasswordFilled(SshCredentialKey),
    /// The password typed at the login prompt was accepted and saved.
    PasswordSaved(SshCredentialKey),
    /// The server rejected the saved password, so it was removed.
    SavedPasswordRejected(SshCredentialKey),
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

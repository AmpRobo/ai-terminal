use lazy_static::lazy_static;
use regex::Regex;

/// A password prompt printed while an SSH client logs in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PasswordPrompt {
    /// The account the prompt asks about, which OpenSSH names in its own prompts.
    pub(super) account: Option<PromptAccount>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PromptAccount {
    pub(super) user: String,
    pub(super) host: String,
}

lazy_static! {
    /// OpenSSH's prompt for the `password` authentication method. The host is the resolved host
    /// name rather than the alias typed in the command.
    static ref PASSWORD_AUTHENTICATION_PROMPT: Regex =
        Regex::new(r"^(?P<account>\S+@\S+)'s password:$").expect("prompt regex is valid");
    /// A `keyboard-interactive` password prompt, which OpenSSH 8.4 and later prefix with the
    /// account.
    static ref KEYBOARD_INTERACTIVE_PROMPT: Regex = Regex::new(
        r"^(?:\((?P<account>\S+@\S+)\) )?(?:[Pp]assword|PASSWORD|密码) ?[:：]$"
    )
    .expect("prompt regex is valid");
}

impl PasswordPrompt {
    /// Parses the text before the cursor while the terminal hides input, or returns `None` if it
    /// doesn't ask for a password.
    pub(super) fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        let captures = PASSWORD_AUTHENTICATION_PROMPT
            .captures(line)
            .or_else(|| KEYBOARD_INTERACTIVE_PROMPT.captures(line))?;
        let account = captures.name("account").and_then(|account| {
            let (user, host) = account.as_str().rsplit_once('@')?;
            Some(PromptAccount {
                user: user.to_owned(),
                host: host.to_owned(),
            })
        });
        Some(Self { account })
    }
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;

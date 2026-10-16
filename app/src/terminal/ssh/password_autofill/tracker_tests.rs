use ai_terminal_core::command::ExitCode;

use super::{LineEditingChars, LoginObservation, SshLoginPasswordTracker, TrackerAction};
use crate::terminal::ssh::password_autofill::{SshCredentialKey, SshLoginTarget};

const PROMPT: &str = "root@10.0.0.1's password: ";

const EDITING: LineEditingChars = LineEditingChars {
    erase: Some(0x7f),
    kill: Some(0x15),
};

fn tracker(command: &str) -> SshLoginPasswordTracker {
    SshLoginPasswordTracker::new(SshLoginTarget::parse(command).expect("is an SSH command"))
}

fn key(user: &str, host: &str) -> SshCredentialKey {
    SshCredentialKey::new(Some(user.to_owned()), host, None)
}

fn root_key() -> SshCredentialKey {
    key("root", "10.0.0.1")
}

fn hidden(line: &str, row: u64) -> LoginObservation {
    LoginObservation::HiddenInput {
        line: line.to_owned(),
        row,
        editing: EDITING,
    }
}

fn saved(password: &str) -> impl FnOnce(&SshCredentialKey) -> Option<String> {
    let password = password.to_owned();
    move |_| Some(password)
}

fn no_saved_password(_: &SshCredentialKey) -> Option<String> {
    None
}

fn fill(key: SshCredentialKey, password: &str) -> TrackerAction {
    TrackerAction::FillPassword {
        key,
        password: password.to_owned(),
    }
}

fn save(key: SshCredentialKey, password: &str) -> TrackerAction {
    TrackerAction::SavePassword {
        key,
        password: password.to_owned(),
    }
}

/// Types `password` at a prompt that has no saved password.
fn type_at_prompt(tracker: &mut SshLoginPasswordTracker, row: u64, password: &[u8]) {
    assert_eq!(
        tracker.on_observation(hidden(PROMPT, row), no_saved_password),
        []
    );
    tracker.on_user_input(password);
}

#[test]
fn fills_saved_password_once_per_prompt() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    assert_eq!(
        tracker.on_observation(hidden(PROMPT, 0), saved("secret")),
        [fill(root_key(), "secret")]
    );
    assert_eq!(
        tracker.on_observation(hidden(PROMPT, 0), saved("secret")),
        []
    );
    assert_eq!(
        tracker.on_observation(LoginObservation::EchoedInput, no_saved_password),
        []
    );
    assert_eq!(
        tracker.on_observation(LoginObservation::RawMode, no_saved_password),
        []
    );
    assert!(tracker.is_finished());
}

#[test]
fn saves_typed_password_once_the_session_starts() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    type_at_prompt(&mut tracker, 0, b"hunter2\r");
    assert_eq!(
        tracker.on_observation(LoginObservation::EchoedInput, no_saved_password),
        []
    );
    assert_eq!(
        tracker.on_observation(LoginObservation::RawMode, no_saved_password),
        [save(root_key(), "hunter2")]
    );
    assert!(tracker.is_finished());
    assert_eq!(tracker.on_command_finished(ExitCode::from(0), ""), None);
}

#[test]
fn saves_typed_password_when_a_remote_command_succeeds() {
    let mut tracker = tracker("ssh root@10.0.0.1 uptime");
    type_at_prompt(&mut tracker, 0, b"hunter2\r");
    assert_eq!(
        tracker.on_command_finished(ExitCode::from(0), " 10:00:00 up 1 day"),
        Some(save(root_key(), "hunter2"))
    );
}

#[test]
fn doesnt_save_password_of_failed_login() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    type_at_prompt(&mut tracker, 0, b"hunter2\r");
    assert_eq!(
        tracker.on_command_finished(
            ExitCode::from(255),
            "root@10.0.0.1: Permission denied (publickey,password)."
        ),
        None
    );
}

#[test]
fn saves_the_password_that_logs_in_after_a_rejected_one() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    type_at_prompt(&mut tracker, 0, b"wrong\r");
    type_at_prompt(&mut tracker, 1, b"right\r");
    assert_eq!(
        tracker.on_observation(LoginObservation::RawMode, no_saved_password),
        [save(root_key(), "right")]
    );
}

#[test]
fn forgets_rejected_saved_password_and_saves_the_retyped_one() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    assert_eq!(
        tracker.on_observation(hidden(PROMPT, 0), saved("old")),
        [fill(root_key(), "old")]
    );
    assert_eq!(
        tracker.on_observation(hidden(PROMPT, 1), saved("old")),
        [TrackerAction::ForgetPassword { key: root_key() }]
    );
    tracker.on_user_input(b"new\r");
    assert_eq!(
        tracker.on_observation(LoginObservation::RawMode, no_saved_password),
        [save(root_key(), "new")]
    );
}

#[test]
fn forgets_saved_password_when_ssh_gives_up() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    tracker.on_observation(hidden(PROMPT, 0), saved("old"));
    assert_eq!(
        tracker.on_command_finished(
            ExitCode::from(255),
            "root@10.0.0.1's password: \nroot@10.0.0.1: Permission denied (publickey,password).\n"
        ),
        Some(TrackerAction::ForgetPassword { key: root_key() })
    );
}

#[test]
fn keeps_saved_password_when_the_failure_isnt_a_rejection() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    tracker.on_observation(hidden(PROMPT, 0), saved("old"));
    assert_eq!(
        tracker.on_command_finished(
            ExitCode::from(255),
            "Permission denied (publickey).\nroot@10.0.0.1's password: \nConnection closed by \
             10.0.0.1 port 22\n"
        ),
        None
    );
}

#[test]
fn keeps_saved_password_the_user_typed_over() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    tracker.on_observation(hidden(PROMPT, 0), saved("old"));
    tracker.on_user_input(b"x");
    assert_eq!(tracker.on_observation(hidden(PROMPT, 1), saved("old")), []);
    assert_eq!(
        tracker.on_command_finished(
            ExitCode::from(255),
            "root@10.0.0.1: Permission denied (publickey,password)."
        ),
        None
    );
}

#[test]
fn doesnt_fill_or_save_after_type_ahead() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    tracker.on_user_input(b"hunter");
    assert_eq!(
        tracker.on_observation(hidden(PROMPT, 0), saved("secret")),
        []
    );
    tracker.on_user_input(b"2\r");
    assert_eq!(
        tracker.on_observation(LoginObservation::RawMode, no_saved_password),
        []
    );
}

#[test]
fn input_typed_while_echoing_doesnt_reach_the_prompt() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    tracker.on_user_input(b"yes");
    assert_eq!(
        tracker.on_observation(LoginObservation::EchoedInput, no_saved_password),
        []
    );
    assert_eq!(
        tracker.on_observation(hidden(PROMPT, 1), saved("secret")),
        [fill(root_key(), "secret")]
    );
}

#[test]
fn mirrors_line_editing() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    type_at_prompt(&mut tracker, 0, b"garbage\x15hunterX\x7f2\r");
    assert_eq!(
        tracker.on_observation(LoginObservation::RawMode, no_saved_password),
        [save(root_key(), "hunter2")]
    );
}

#[test]
fn doesnt_save_password_edited_in_ways_that_arent_mirrored() {
    for input in [
        &b"hunter\x1b[D2\r"[..],
        b"hunter\x172\r",
        "pässwö\x7f\r".as_bytes(),
    ] {
        let mut tracker = tracker("ssh root@10.0.0.1");
        type_at_prompt(&mut tracker, 0, input);
        assert_eq!(
            tracker.on_observation(LoginObservation::RawMode, no_saved_password),
            [],
            "{input:?}"
        );
    }

    let mut tracker = tracker("ssh root@10.0.0.1");
    type_at_prompt(&mut tracker, 0, b"hunter");
    tracker.on_untracked_input();
    tracker.on_user_input(b"2\r");
    assert_eq!(
        tracker.on_observation(LoginObservation::RawMode, no_saved_password),
        []
    );
}

#[test]
fn saves_jump_host_password_when_the_destination_prompts() {
    let mut tracker = tracker("ssh -J admin@jump root@10.0.0.1");
    assert_eq!(
        tracker.on_observation(hidden("admin@jump's password: ", 0), no_saved_password),
        []
    );
    tracker.on_user_input(b"jump-password\r");
    assert_eq!(
        tracker.on_observation(hidden(PROMPT, 1), no_saved_password),
        [save(key("admin", "jump"), "jump-password")]
    );
    tracker.on_user_input(b"root-password\r");
    assert_eq!(
        tracker.on_observation(LoginObservation::RawMode, no_saved_password),
        [save(root_key(), "root-password")]
    );
}

#[test]
fn ignores_relayed_prompts_that_dont_name_their_account() {
    let mut tracker = tracker("ssh -J admin@jump root@10.0.0.1");
    assert_eq!(
        tracker.on_observation(hidden("Password: ", 0), saved("secret")),
        []
    );
    tracker.on_user_input(b"hunter2\r");
    assert_eq!(
        tracker.on_observation(LoginObservation::RawMode, no_saved_password),
        []
    );
}

#[test]
fn ignores_other_hidden_input() {
    let mut tracker = tracker("ssh root@10.0.0.1");
    assert_eq!(
        tracker.on_observation(
            hidden("Enter passphrase for key '/root/.ssh/id_rsa': ", 0),
            saved("secret")
        ),
        []
    );
    tracker.on_user_input(b"passphrase\r");
    assert_eq!(
        tracker.on_observation(LoginObservation::RawMode, no_saved_password),
        []
    );
}

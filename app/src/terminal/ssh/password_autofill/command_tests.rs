use ai_terminal_core::command::ExitCode;

use super::{SshLoginTarget, SshProgram};
use crate::terminal::ssh::password_autofill::SshCredentialKey;
use crate::terminal::ssh::password_autofill::prompt::PasswordPrompt;

fn target(
    program: SshProgram,
    user: Option<&str>,
    host: Option<&str>,
    port: Option<u16>,
    may_relay: bool,
) -> SshLoginTarget {
    SshLoginTarget {
        program,
        user: user.map(str::to_owned),
        host: host.map(str::to_owned),
        port,
        may_relay,
    }
}

fn parse(command: &str) -> SshLoginTarget {
    SshLoginTarget::parse(command).unwrap_or_else(|| panic!("`{command}` should parse"))
}

fn key(user: Option<&str>, host: &str, port: Option<u16>) -> SshCredentialKey {
    SshCredentialKey::new(user.map(str::to_owned), host, port)
}

fn prompt(line: &str) -> PasswordPrompt {
    PasswordPrompt::parse(line).unwrap_or_else(|| panic!("`{line}` should be a password prompt"))
}

#[test]
fn parses_ssh_destination() {
    use SshProgram::Ssh;

    assert_eq!(
        parse("ssh root@10.0.0.1"),
        target(Ssh, Some("root"), Some("10.0.0.1"), None, false)
    );
    assert_eq!(
        parse("ssh -v -i ~/.ssh/id_ed25519 example.com ls -la"),
        target(Ssh, None, Some("example.com"), None, false)
    );
    assert_eq!(
        parse("ssh -46Cp2222 admin@example.com"),
        target(Ssh, Some("admin"), Some("example.com"), Some(2222), false)
    );
    assert_eq!(
        parse("ssh ssh://admin;fingerprint=x@example.com:2200"),
        target(Ssh, Some("admin"), Some("example.com"), Some(2200), false)
    );
    assert_eq!(
        parse("ssh admin@[fe80::1]"),
        target(Ssh, Some("admin"), Some("[fe80::1]"), None, false)
    );
    assert_eq!(parse("ssh"), target(Ssh, None, None, None, false));
}

#[test]
fn parses_ssh_options_after_the_destination() {
    use SshProgram::Ssh;

    assert_eq!(
        parse("ssh example.com -p 2222 -l admin"),
        target(Ssh, Some("admin"), Some("example.com"), Some(2222), false)
    );
    // After `--`, everything past the destination is the remote command.
    assert_eq!(
        parse("ssh -- example.com -p 2222"),
        target(Ssh, None, Some("example.com"), None, false)
    );
    // The remote command's options aren't ssh's.
    assert_eq!(
        parse("ssh example.com ls -p 2222"),
        target(Ssh, None, Some("example.com"), None, false)
    );
}

#[test]
fn first_ssh_user_and_port_win() {
    use SshProgram::Ssh;

    assert_eq!(
        parse("ssh -l alice bob@example.com"),
        target(Ssh, Some("alice"), Some("example.com"), None, false)
    );
    assert_eq!(
        parse("ssh bob@example.com -l alice"),
        target(Ssh, Some("bob"), Some("example.com"), None, false)
    );
    assert_eq!(
        parse("ssh -o User=alice -l bob -o Port=2200 -p 2222 example.com"),
        target(Ssh, Some("alice"), Some("example.com"), Some(2200), false)
    );
    assert_eq!(
        parse("ssh -o 'port 2200' ssh://example.com:2222"),
        target(Ssh, None, Some("example.com"), Some(2200), false)
    );
}

#[test]
fn parses_ssh_relays() {
    use SshProgram::Ssh;

    assert_eq!(
        parse("ssh -J jump.example.com root@10.0.0.1"),
        target(Ssh, Some("root"), Some("10.0.0.1"), None, true)
    );
    assert_eq!(
        parse("ssh -o 'ProxyCommand=nc -X 5 %h %p' 10.0.0.1"),
        target(Ssh, None, Some("10.0.0.1"), None, true)
    );
    assert_eq!(
        parse("ssh -o ProxyJump=none 10.0.0.1"),
        target(Ssh, None, Some("10.0.0.1"), None, false)
    );
}

#[test]
fn parses_wrapped_ssh_commands() {
    use SshProgram::Ssh;

    assert_eq!(
        parse("TERM=xterm-256color command /usr/bin/ssh root@10.0.0.1"),
        target(Ssh, Some("root"), Some("10.0.0.1"), None, false)
    );
}

#[test]
fn ignores_other_commands() {
    for command in [
        "",
        "echo ssh root@10.0.0.1",
        "sshpass -p secret ssh root@10.0.0.1",
        "ssh-copy-id root@10.0.0.1",
        "ssh 'unterminated",
    ] {
        assert_eq!(SshLoginTarget::parse(command), None, "{command}");
    }
}

#[test]
fn parses_scp_destination() {
    use SshProgram::Scp;

    assert_eq!(
        parse("scp -r ./site root@10.0.0.1:/var/www"),
        target(Scp, Some("root"), Some("10.0.0.1"), None, false)
    );
    assert_eq!(
        parse("scp -P 2222 admin@[fe80::1]:notes.txt ./"),
        target(Scp, Some("admin"), Some("[fe80::1]"), Some(2222), false)
    );
    assert_eq!(
        parse("scp -P 2222 notes.txt scp://admin@example.com:2200/tmp"),
        target(Scp, Some("admin"), Some("example.com"), Some(2200), false)
    );
    // scp passes `-o` options to ssh ahead of the destination's user.
    assert_eq!(
        parse("scp -o User=alice notes.txt bob@example.com:"),
        target(Scp, Some("alice"), Some("example.com"), None, false)
    );
    assert_eq!(
        parse("scp example.com:a.txt example.com:b.txt"),
        target(Scp, None, Some("example.com"), None, false)
    );
}

#[test]
fn parses_scp_without_a_single_remote_as_relayed() {
    use SshProgram::Scp;

    assert_eq!(
        parse("scp first.example.com:a.txt second.example.com:"),
        target(Scp, None, None, None, true)
    );
    assert_eq!(
        parse("scp ./a.txt ./dir/b:c.txt"),
        target(Scp, None, None, None, true)
    );
}

#[test]
fn parses_sftp_and_rsync() {
    assert_eq!(
        parse("sftp -P 2222 admin@example.com"),
        target(
            SshProgram::Sftp,
            Some("admin"),
            Some("example.com"),
            Some(2222),
            false
        )
    );
    assert_eq!(
        parse("sftp sftp://admin@example.com:2200/upload"),
        target(
            SshProgram::Sftp,
            Some("admin"),
            Some("example.com"),
            Some(2200),
            false
        )
    );
    assert_eq!(
        parse("rsync -av -e 'ssh -p 2222' ./ root@10.0.0.1:/backup"),
        target(SshProgram::Rsync, None, None, None, true)
    );
}

#[test]
fn credential_key_prefers_the_account_named_by_the_prompt() {
    let alias = parse("ssh -p 2222 myserver");
    assert_eq!(
        alias.credential_key(&prompt("root@10.0.0.1's password:")),
        Some(key(Some("root"), "10.0.0.1", Some(2222)))
    );
    assert_eq!(
        alias.credential_key(&prompt("Password:")),
        Some(key(None, "myserver", Some(2222)))
    );
}

#[test]
fn credential_key_attributes_relayed_logins_by_prompt() {
    let relayed = parse("ssh -J admin@jump.example.com -p 2222 root@10.0.0.1");
    assert_eq!(
        relayed.credential_key(&prompt("admin@jump.example.com's password:")),
        Some(key(Some("admin"), "jump.example.com", None))
    );
    assert_eq!(
        relayed.credential_key(&prompt("(root@10.0.0.1) Password:")),
        Some(key(Some("root"), "10.0.0.1", Some(2222)))
    );
    assert_eq!(relayed.credential_key(&prompt("Password:")), None);
    assert_eq!(
        parse("ssh").credential_key(&prompt("Password:")),
        None,
        "a command without a destination can't name the account"
    );
}

#[test]
fn exit_code_proves_login() {
    let ssh = parse("ssh example.com");
    for code in [0, 1, 127] {
        assert!(ssh.exit_code_proves_login(ExitCode::from(code)), "{code}");
    }
    for code in [130, 255] {
        assert!(!ssh.exit_code_proves_login(ExitCode::from(code)), "{code}");
    }

    let scp = parse("scp a.txt example.com:");
    assert!(scp.exit_code_proves_login(ExitCode::from(0)));
    assert!(!scp.exit_code_proves_login(ExitCode::from(1)));
}

use super::{PasswordPrompt, PromptAccount};

fn account(user: &str, host: &str) -> Option<PromptAccount> {
    Some(PromptAccount {
        user: user.to_owned(),
        host: host.to_owned(),
    })
}

#[test]
fn parses_password_authentication_prompt() {
    assert_eq!(
        PasswordPrompt::parse("root@10.0.0.1's password: "),
        Some(PasswordPrompt {
            account: account("root", "10.0.0.1"),
        })
    );
    assert_eq!(
        PasswordPrompt::parse("first.last@corp.com@build-01's password:"),
        Some(PasswordPrompt {
            account: account("first.last@corp.com", "build-01"),
        })
    );
}

#[test]
fn parses_keyboard_interactive_prompts() {
    assert_eq!(
        PasswordPrompt::parse("(admin@example.com) Password: "),
        Some(PasswordPrompt {
            account: account("admin", "example.com"),
        })
    );
    for prompt in ["Password:", "password: ", "PASSWORD:", "密码：", "密码:"] {
        assert_eq!(
            PasswordPrompt::parse(prompt),
            Some(PasswordPrompt { account: None }),
            "{prompt}"
        );
    }
}

#[test]
fn ignores_other_hidden_input_prompts() {
    for prompt in [
        "",
        "Enter passphrase for key '/home/me/.ssh/id_ed25519': ",
        "Verification code: ",
        "(admin@example.com) Verification code: ",
        "[sudo] password for me: ",
        "Old password:",
        "root@10.0.0.1's password: oops",
    ] {
        assert_eq!(PasswordPrompt::parse(prompt), None, "{prompt}");
    }
}

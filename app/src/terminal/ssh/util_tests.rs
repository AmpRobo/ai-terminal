use super::*;

#[test]
fn ssh_command_starting_in_directory_appends_a_login_shell() {
    assert_eq!(
        ssh_command_starting_in_directory("ssh user@host", "/repo").as_deref(),
        Some("ssh user@host -t 'cd /repo && exec \"$SHELL\" -l'")
    );
    assert_eq!(
        ssh_command_starting_in_directory("ssh -p 2222 user@host", "/repo").as_deref(),
        Some("ssh -p 2222 user@host -t 'cd /repo && exec \"$SHELL\" -l'")
    );
    assert_eq!(
        ssh_command_starting_in_directory("command ssh user@host", "/repo").as_deref(),
        Some("ssh user@host -t 'cd /repo && exec \"$SHELL\" -l'")
    );
    assert_eq!(
        ssh_command_starting_in_directory("ssh user@host", "/my repo").as_deref(),
        Some("ssh user@host -t 'cd '\\''/my repo'\\'' && exec \"$SHELL\" -l'")
    );
}

#[test]
fn ssh_command_starting_in_directory_rejects_non_plain_ssh() {
    assert!(ssh_command_starting_in_directory("gcloud compute ssh instance", "/repo").is_none());
    assert!(ssh_command_starting_in_directory("ssh user@host uname", "/repo").is_none());
    assert!(ssh_command_starting_in_directory("ssh", "/repo").is_none());
}

#[test]
fn ssh_command_starting_in_directory_replaces_hostname_with_ip() {
    let directory = "/home/michael/log/vehicle/2026-08-24_20-20-58_61895";
    assert_eq!(
        ssh_command_starting_in_directory_with_host(
            "ssh michael@ThinkPad-X1",
            directory,
            Some("192.168.1.5"),
        )
        .as_deref(),
        Some(
            "ssh -o 'HostKeyAlias=ThinkPad-X1' michael@192.168.1.5 -t 'cd /home/michael/log/vehicle/2026-08-24_20-20-58_61895 && exec \"$SHELL\" -l'"
        )
    );
    assert_eq!(
        ssh_command_starting_in_directory_with_host(
            "ssh -p 2222 -i ~/.ssh/id_ed25519 michael@ThinkPad-X1",
            "/repo",
            Some("192.168.1.5"),
        )
        .as_deref(),
        Some(
            "ssh -p 2222 -i ~/.ssh/id_ed25519 -o 'HostKeyAlias=ThinkPad-X1' michael@192.168.1.5 -t 'cd /repo && exec \"$SHELL\" -l'"
        )
    );
    assert_eq!(
        ssh_command_starting_in_directory_with_host(
            "ssh michael@ThinkPad-X1",
            "/repo",
            Some("2001:db8::1"),
        )
        .as_deref(),
        Some(
            "ssh -o 'HostKeyAlias=ThinkPad-X1' 'michael@[2001:db8::1]' -t 'cd /repo && exec \"$SHELL\" -l'"
        )
    );
    assert_eq!(
        ssh_command_starting_in_directory_with_host(
            "ssh -o HostKeyAlias=alias michael@ThinkPad-X1",
            "/repo",
            Some("192.168.1.5"),
        )
        .as_deref(),
        Some("ssh -o 'HostKeyAlias=alias' michael@192.168.1.5 -t 'cd /repo && exec \"$SHELL\" -l'")
    );
}

#[test]
fn ssh_command_starting_in_directory_keeps_jump_host_and_literal_ip() {
    assert_eq!(
        ssh_command_starting_in_directory_with_host(
            "ssh -J bastion michael@ThinkPad-X1",
            "/repo",
            Some("10.0.0.1"),
        )
        .as_deref(),
        Some("ssh -J bastion michael@ThinkPad-X1 -t 'cd /repo && exec \"$SHELL\" -l'")
    );
    assert_eq!(
        ssh_command_starting_in_directory_with_host(
            "ssh -o ProxyJump=bastion michael@ThinkPad-X1",
            "/repo",
            Some("10.0.0.1"),
        )
        .as_deref(),
        Some("ssh -o ProxyJump=bastion michael@ThinkPad-X1 -t 'cd /repo && exec \"$SHELL\" -l'")
    );
    assert_eq!(
        ssh_command_starting_in_directory_with_host(
            "ssh michael@192.168.1.5",
            "/repo",
            Some("10.0.0.1"),
        )
        .as_deref(),
        Some("ssh michael@192.168.1.5 -t 'cd /repo && exec \"$SHELL\" -l'")
    );
}

#[test]
fn ssh_command_for_endpoint_includes_user_and_port() {
    assert_eq!(
        ssh_command_for_endpoint("user", "host", Some("2222"), "/repo"),
        "ssh -p 2222 user@host -t 'cd /repo && exec \"$SHELL\" -l'"
    );
    assert_eq!(
        ssh_command_for_endpoint("", "host", None, "/repo"),
        "ssh host -t 'cd /repo && exec \"$SHELL\" -l'"
    );
    assert_eq!(
        ssh_command_for_endpoint("user", "2001:db8::1", None, "/repo"),
        "ssh 'user@[2001:db8::1]' -t 'cd /repo && exec \"$SHELL\" -l'"
    );
}

#[test]
fn ssh_gcloud_ssh_parsing() {
    assert!(parse_interactive_ssh_command("gcloud").is_none());
    assert!(parse_interactive_ssh_command("gcloud compute").is_none());
    assert!(parse_interactive_ssh_command("gcloud compute ss").is_none());
    assert!(parse_interactive_ssh_command("gcloud compute ssh").is_none());
    assert!(parse_interactive_ssh_command("command gcloud compute ssh").is_none());

    assert!(
        parse_interactive_ssh_command("command gcloud compute ssh --zone us-west1-a").is_some()
    );
    assert!(parse_interactive_ssh_command("gcloud compute ssh --zone us-west1-a").is_some());
    assert!(
        parse_interactive_ssh_command("gcloud compute ssh --zone us-west1-a my-instance").is_some()
    );
    assert!(
        parse_interactive_ssh_command(
            "gcloud compute ssh --zone us-west1-a my-instance --project my-project"
        )
        .is_some()
    );
}

#[test]
fn ssh_elastic_beanstalk_parsing() {
    assert!(parse_interactive_ssh_command("eb").is_none());
    assert!(parse_interactive_ssh_command("eb ss").is_none());
    assert!(parse_interactive_ssh_command("eb ssh").is_none());
    assert!(parse_interactive_ssh_command("command eb ssh").is_none());

    assert!(parse_interactive_ssh_command("command eb ssh --profile my-profile").is_some());
    assert!(parse_interactive_ssh_command("eb ssh --profile my-profile").is_some());
    assert!(parse_interactive_ssh_command("eb ssh --profile my-profile my-env").is_some());
}

#[test]
fn ssh_digital_ocean_droplet_parsing() {
    assert!(parse_interactive_ssh_command("doctl").is_none());
    assert!(parse_interactive_ssh_command("doctl compute").is_none());
    assert!(parse_interactive_ssh_command("doctl compute ss").is_none());
    assert!(parse_interactive_ssh_command("doctl compute ssh").is_none());
    assert!(parse_interactive_ssh_command("command doctl compute ssh").is_none());

    assert!(parse_interactive_ssh_command("command doctl compute ssh --region nyc1").is_some());
    assert!(parse_interactive_ssh_command("doctl compute ssh --region nyc1").is_some());
    assert!(parse_interactive_ssh_command("doctl compute ssh --region nyc1 my-droplet").is_some());
}

/// Verifies that commands resulting from shell alias expansion are correctly
/// detected as interactive SSH commands. When a user types an alias (e.g.
/// `myssh`), the terminal view expands it to the alias value before passing
/// it to `parse_interactive_ssh_command`. These tests cover representative
/// expanded forms.
#[test]
fn ssh_alias_expanded_commands() {
    // Simple alias: alias myssh='ssh user@host'
    assert_eq!(
        parse_interactive_ssh_command("ssh user@host").unwrap().host,
        Some("user@host".to_string())
    );

    // Alias with key and user: alias company1='ssh -i /path/to/key user@server'
    assert_eq!(
        parse_interactive_ssh_command("ssh -i /path/to/key user@server")
            .unwrap()
            .host,
        Some("user@server".to_string())
    );

    // Alias with extra args appended by the user: alias myssh='ssh -i key'
    // then the user types `myssh user@host` which expands to `ssh -i key user@host`
    assert_eq!(
        parse_interactive_ssh_command("ssh -i key user@host")
            .unwrap()
            .host,
        Some("user@host".to_string())
    );

    // Alias that isn't SSH should not match
    assert!(parse_interactive_ssh_command("ls -la").is_none());
}

#[test]
fn ssh_interactive_shell_parsing() {
    assert!(parse_interactive_ssh_command("").is_none());
    assert!(parse_interactive_ssh_command("ls").is_none());
    assert!(parse_interactive_ssh_command("ssh-add-key").is_none());

    // Basic interactive command
    assert!(
        parse_interactive_ssh_command("ssh localhost").unwrap().host
            == Some("localhost".to_string())
    );
    assert!(
        parse_interactive_ssh_command("command ssh localhost")
            .unwrap()
            .host
            == Some("localhost".to_string())
    );
    assert!(
        parse_interactive_ssh_command("ssh root@127.14.80.1 -p 2222")
            .unwrap()
            .host
            == Some("root@127.14.80.1".to_string())
    );
    assert!(
        parse_interactive_ssh_command("ssh -4vw root@127.14.80.1 -p 2222")
            .unwrap()
            .host
            == Some("root@127.14.80.1".to_string())
    );

    // Commands with -T or -W, which are non-interactive
    assert!(parse_interactive_ssh_command("ssh -T user@host").is_none());
    assert!(parse_interactive_ssh_command("ssh -v user@host -W localhost:22").is_none());
    assert!(parse_interactive_ssh_command("ssh -o IdentityFile=/etc/file -T user@host").is_none());

    // Commands with multiple positional arguments, implying non-interactive
    assert!(parse_interactive_ssh_command("ssh user@host ls").is_none());
    assert!(parse_interactive_ssh_command("ssh user@host echo 'Hello, World!'").is_none());

    // Weird spacing and shell characters shouldn't matter
    assert!(
        parse_interactive_ssh_command("ssh     user@host")
            .unwrap()
            .host
            == Some("user@host".to_string())
    );
    assert!(
        parse_interactive_ssh_command("ssh -4 -- localhost")
            .unwrap()
            .host
            == Some("localhost".to_string())
    );
}

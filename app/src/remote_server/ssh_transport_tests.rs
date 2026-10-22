use ai_terminal_ui::r#async::BoxFuture;

use super::*;

fn static_auth_context() -> Arc<RemoteServerAuthContext> {
    Arc::new(RemoteServerAuthContext::new(
        || -> BoxFuture<'static, Option<String>> { Box::pin(async { None }) },
        || "user id/with spaces".to_string(),
        String::new(),
        String::new(),
        true,
    ))
}

#[test]
fn remote_proxy_command_quotes_identity_key() {
    let transport = SshTransport::new(
        PathBuf::from("/tmp/control-master.sock"),
        static_auth_context(),
        true,
    );

    let command = transport.remote_proxy_command();

    assert!(command.contains("remote-server-proxy --identity-key"));
    assert!(command.contains("'user id/with spaces'"));
}

#[test]
fn command_without_fallback_binary_runs_the_binary() {
    assert_eq!(
        command_with_fallback_binary("~/remote-server/oz", None, "--version"),
        "~/remote-server/oz --version"
    );
}

#[cfg(unix)]
#[test]
fn command_with_fallback_binary_runs_the_fallback_only_when_the_binary_is_missing() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().expect("create temp dir");
    let write_fake_binary = |name: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, format!("#!/bin/sh\necho {name} \"$@\"\n")).expect("write binary");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("make binary executable");
        path
    };
    let binary = write_fake_binary("build");
    let fallback = write_fake_binary("fallback");
    let command = command_with_fallback_binary(
        binary.to_str().unwrap(),
        Some(fallback.to_str().unwrap()),
        "remote-server-proxy --identity-key key",
    );
    let run = || {
        let output = command::blocking::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .expect("run command");
        String::from_utf8(output.stdout).expect("utf-8 output")
    };

    assert_eq!(run(), "build remote-server-proxy --identity-key key\n");
    std::fs::remove_file(&binary).expect("remove binary");
    assert_eq!(run(), "fallback remote-server-proxy --identity-key key\n");
}

use super::super::protocol::ToolKind;
use super::{
    EXCLUDED_NETWORK_TOOLS, LocalToolPermission, fallback_auth_env, permission_for_tool_kind,
    qwen_arguments, start_local_terminal,
};

#[test]
fn qwen_launch_stays_on_acp_and_drops_network_tools() {
    let args = qwen_arguments();
    assert_eq!(args.first().map(String::as_str), Some("--acp"));
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--approval-mode" && pair[1] == "yolo")
    );
    let excluded = args
        .windows(2)
        .find(|pair| pair[0] == "--exclude-tools")
        .expect("exclude-tools");
    for tool in EXCLUDED_NETWORK_TOOLS {
        assert!(
            excluded[1].split(',').any(|name| name == *tool),
            "missing excluded tool {tool}"
        );
    }
    assert!(args.iter().all(|arg| !arg.starts_with("http")));
}

#[test]
fn missing_api_key_selects_local_openai_auth_without_replacing_a_real_key() {
    let vars = fallback_auth_env(None).expect("fallback auth");
    assert!(
        vars.iter()
            .any(|(key, value)| *key == "OPENAI_API_KEY" && *value == "local")
    );
    assert!(vars.iter().any(|(key, value)| {
        *key == "OPENAI_BASE_URL" && value.starts_with("http://127.0.0.1:")
    }));
    assert!(fallback_auth_env(Some(std::ffi::OsStr::new("real-key"))).is_none());
}

#[test]
fn network_tool_kinds_are_refused_and_local_tools_are_allowed() {
    assert!(matches!(
        permission_for_tool_kind(Some(ToolKind::Fetch)),
        LocalToolPermission::Deny { .. }
    ));
    for kind in [
        ToolKind::Read,
        ToolKind::Edit,
        ToolKind::Execute,
        ToolKind::Search,
        ToolKind::Other,
    ] {
        assert!(matches!(
            permission_for_tool_kind(Some(kind)),
            LocalToolPermission::Allow
        ));
    }
    assert!(matches!(
        permission_for_tool_kind(None),
        LocalToolPermission::Allow
    ));
}

#[cfg(unix)]
#[test]
fn local_terminal_runs_a_command_without_a_shell() {
    let terminal = start_local_terminal(
        "echo",
        &["hello; touch /tmp/no-qwen-shell".to_owned()],
        None,
        &[],
        4096,
    )
    .expect("echo");
    let exit = terminal.wait_for_exit();
    let (output, truncated, _) = terminal.snapshot();
    assert_eq!(exit, Some(0));
    assert!(!truncated);
    assert!(output.contains("hello; touch /tmp/no-qwen-shell"));
}

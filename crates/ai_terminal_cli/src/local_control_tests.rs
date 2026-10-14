use std::collections::HashSet;

use clap_complete::aot::Shell;
use local_control::protocol::{ActionKind, ControlError, ErrorCode};
use serde_json::json;

use super::*;

#[test]
fn parses_typed_create_and_setting_list_params() {
    let args = ControlArgs::try_parse_from([
        "ai_terminal_ctrl",
        "tab",
        "create",
        "--type",
        "agent",
        "--session",
        "session_1",
    ])
    .expect("tab create parses");
    let ControlCommand::Tab(TabCommand::Create(args)) = args.command else {
        panic!("expected tab create command");
    };
    assert_eq!(args.tab_type, Some(CliTabType::Agent));
    assert_eq!(args.target.session.as_deref(), Some("session_1"));

    let err = ControlArgs::try_parse_from(["ai_terminal_ctrl", "tab", "create", "--shell", "zsh"])
        .expect_err("shell is not an accepted tab create flag");
    assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);

    let args = ControlArgs::try_parse_from([
        "ai_terminal_ctrl",
        "setting",
        "list",
        "--namespace",
        "editor",
    ])
    .expect("setting list parses");
    let ControlCommand::Setting(SettingCommand::List(args)) = args.command else {
        panic!("expected setting list command");
    };
    assert_eq!(args.namespace.as_deref(), Some("editor"));
}

#[test]
fn rejects_conflicting_instance_selectors() {
    let err = ControlArgs::try_parse_from([
        "ai_terminal_ctrl",
        "tab",
        "create",
        "--instance",
        "inst_123",
        "--pid",
        "123",
    ])
    .expect_err("instance and pid conflict");
    assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
}

#[test]
fn parses_instance_and_pid_selectors() {
    let args = ControlArgs::try_parse_from([
        "ai_terminal_ctrl",
        "tab",
        "create",
        "--instance",
        "inst_123",
    ])
    .expect("instance selector parses");
    let ControlCommand::Tab(TabCommand::Create(create)) = args.command else {
        panic!("expected tab create command");
    };
    assert_eq!(create.target.instance.as_deref(), Some("inst_123"));

    let args = ControlArgs::try_parse_from(["ai_terminal_ctrl", "app", "ping", "--pid", "123"])
        .expect("pid selector parses");
    let ControlCommand::App(AppCommand::Ping(target)) = args.command else {
        panic!("expected app ping command");
    };
    assert_eq!(target.pid, Some(123));
}

#[test]
fn surface_list_accepts_instance_selection() {
    let args = ControlArgs::try_parse_from([
        "ai_terminal_ctrl",
        "surface",
        "list",
        "--instance",
        "inst_123",
    ])
    .expect("surface list instance selector parses");
    let ControlCommand::Surface(SurfaceCommand::List(target)) = args.command else {
        panic!("expected surface list command");
    };
    assert_eq!(target.instance.as_deref(), Some("inst_123"));
}

#[test]
fn rejects_excluded_command_routes() {
    for args in [
        vec!["ai_terminal_ctrl", "history", "list"],
        vec!["ai_terminal_ctrl", "block", "list"],
        vec!["ai_terminal_ctrl", "block", "inspect", "block_1"],
        vec!["ai_terminal_ctrl", "block", "output", "block_1"],
        vec!["ai_terminal_ctrl", "input", "get"],
        vec!["ai_terminal_ctrl", "input", "clear"],
        vec!["ai_terminal_ctrl", "input", "mode", "set", "agent"],
        vec!["ai_terminal_ctrl", "input", "run", "pwd"],
        vec!["ai_terminal_ctrl", "file", "list"],
        vec!["ai_terminal_ctrl", "drive", "list"],
        vec!["ai_terminal_ctrl", "auth", "status"],
    ] {
        assert!(ControlArgs::try_parse_from(args).is_err());
    }
}

#[test]
fn parses_first_slice_instance_list() {
    let args = ControlArgs::try_parse_from(["ai_terminal_ctrl", "instance", "list"])
        .expect("instance list parses");
    assert!(matches!(
        args.command,
        ControlCommand::Instance(InstanceCommand::List)
    ));
}

#[test]
fn parses_first_slice_app_smoke_metadata_commands() {
    assert!(ControlArgs::try_parse_from(["ai_terminal_ctrl", "app", "ping"]).is_ok());
    assert!(ControlArgs::try_parse_from(["ai_terminal_ctrl", "app", "version"]).is_ok());
    assert!(ControlArgs::try_parse_from(["ai_terminal_ctrl", "app", "active"]).is_ok());
    assert!(ControlArgs::try_parse_from(["ai_terminal_ctrl", "app", "focus"]).is_ok());
}

#[test]
fn parses_catalog_metadata_commands() {
    let args = ControlArgs::try_parse_from([
        "ai_terminal_ctrl",
        "action",
        "inspect",
        "surface.settings.open",
    ])
    .expect("action inspect parses");
    let ControlCommand::Action(ActionCatalogCommand::Inspect { action }) = args.command else {
        panic!("expected action inspect command");
    };
    assert_eq!(action, "surface.settings.open");
    assert!(ControlArgs::try_parse_from(["ai_terminal_ctrl", "action", "list"]).is_ok());
    assert!(ControlArgs::try_parse_from(["ai_terminal_ctrl", "capability", "list"]).is_ok());
    assert!(
        ControlArgs::try_parse_from(["ai_terminal_ctrl", "capability", "inspect", "tab.create"])
            .is_ok()
    );
}

#[test]
fn parses_control_mode_args_after_hidden_flag() {
    let args = ControlArgs::try_parse_control_mode_from([
        "ai-terminal",
        "--ai_terminal_ctrl",
        "tab",
        "create",
    ])
    .expect("control mode flag is present")
    .expect("control mode args parse");
    assert!(matches!(
        args.command,
        ControlCommand::Tab(TabCommand::Create(_))
    ));
}

#[test]
fn ignores_args_without_control_mode_flag() {
    assert!(ControlArgs::try_parse_control_mode_from(["ai-terminal", "tab", "create"]).is_none());
}

#[test]
fn parses_completion_generation_command() {
    let args = ControlArgs::try_parse_from(["ai_terminal_ctrl", "completions", "bash"])
        .expect("completions parses");
    assert!(matches!(
        args.command,
        ControlCommand::Completions {
            shell: Some(Shell::Bash)
        }
    ));
}

#[test]
fn parses_exact_window_tab_pane_and_session_selectors() {
    let args = ControlArgs::try_parse_from([
        "ai_terminal_ctrl",
        "session",
        "inspect",
        "--window-title",
        "docs",
        "--tab-index",
        "2",
        "--pane",
        "pane_1",
        "--session",
        "session_1",
    ])
    .expect("exact target selectors parse");
    let ControlCommand::Session(SessionCommand::Inspect(target)) = args.command else {
        panic!("expected session inspect command");
    };
    assert_eq!(target.window_title.as_deref(), Some("docs"));
    assert_eq!(target.tab_index, Some(2));
    assert_eq!(target.pane.as_deref(), Some("pane_1"));
    assert_eq!(target.session.as_deref(), Some("session_1"));
}

#[test]
fn instance_list_output_serializes_empty_and_populated_lists() {
    let empty = serde_json::to_value(commands::instance_list_output(Vec::new()))
        .expect("empty list serializes");
    assert_eq!(empty, json!({ "instances": [] }));

    let record = local_control::discovery::InstanceRecord::for_current_process(
        None,
        "dev",
        "dev.aiterminal.AiTerminal",
        Some("v0.1.0".to_owned()),
        Vec::new(),
    );
    let instance_id = record.instance_id.0.clone();
    let populated = serde_json::to_value(commands::instance_list_output(vec![record]))
        .expect("populated list serializes");
    assert_eq!(populated["instances"][0]["instance_id"], json!(instance_id));
    assert_eq!(populated["instances"][0]["channel"], json!("dev"));
    assert_eq!(
        populated["instances"][0]["app_id"],
        json!("dev.aiterminal.AiTerminal")
    );
    assert_eq!(populated["instances"][0]["app_version"], json!("v0.1.0"));
}

#[test]
fn excluded_actions_are_not_allowlisted_catalog_entries() {
    for excluded in ["auth.api_key.set", "file.write", "block.list"] {
        assert!(
            ActionKind::ALL
                .iter()
                .all(|action| action.as_str() != excluded)
        );
    }
}

#[test]
fn generated_bash_completions_include_readonly_commands() {
    let completions =
        generate_completion_string(Shell::Bash).expect("bash completions render to UTF-8");
    assert!(completions.contains("instance"));
    assert!(completions.contains("action"));
    assert!(completions.contains("capability"));
    assert!(!completions.contains("stubs-only"));
    assert!(completions.contains("window"));
    assert!(completions.contains("input"));
    assert!(completions.contains("completions"));
    assert!(!completions.contains("block"));
}

#[test]
fn every_retained_catalog_action_has_a_parseable_cli_example() {
    let mut covered = HashSet::new();
    for (kind, argv) in retained_action_examples() {
        let args = ControlArgs::try_parse_from(argv)
            .unwrap_or_else(|err| panic!("{} parses: {err}", kind.as_str()));
        assert_eq!(parsed_action_kind(&args.command), Some(kind));
        covered.insert(kind);
    }
    let expected = ActionKind::ALL.iter().copied().collect::<HashSet<_>>();
    let missing = expected
        .difference(&covered)
        .map(|kind| kind.as_str())
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "retained catalog actions missing parser examples: {missing:?}"
    );
}

#[test]
fn generated_bash_completions_include_mutating_command_groups() {
    let completions =
        generate_completion_string(Shell::Bash).expect("bash completions render to UTF-8");
    assert!(completions.contains("surface"));
    assert!(completions.contains("command-palette"));
    assert!(completions.contains("ai-terminal-drive"));
    assert!(completions.contains("resource-center"));
    assert!(completions.contains("activate"));
    assert!(completions.contains("split"));
    assert!(!completions.contains("history"));
    assert!(!completions.contains("share-to-team"));
}

#[test]
fn structured_error_output_uses_stable_error_code() {
    let error = ControlError::new(
        ErrorCode::NoInstance,
        "no local AI Terminal control instances",
    );
    let value = serde_json::to_value(ErrorSummary {
        ok: false,
        error: &error,
    })
    .expect("error summary serializes");
    assert_eq!(value["ok"], json!(false));
    assert_eq!(value["error"]["code"], json!("no_instance"));
    assert_eq!(
        value["error"]["message"],
        json!("no local AI Terminal control instances")
    );
}

#[test]
fn renders_human_readable_tab_create_output() {
    let rendered = render_human_readable_for_test(
        local_control::protocol::ActionKind::TabCreate,
        &json!({
            "tab": {
                "id": "tab_123",
                "active_index": 2,
                "count": 3
            },
            "window": {
                "id": "window_123"
            }
        }),
    );
    assert_eq!(
        rendered,
        "Created tab tab_123 in window window_123 (active index 2, tab count 3)"
    );
}

fn retained_action_examples() -> Vec<(ActionKind, Vec<&'static str>)> {
    vec![
        (
            ActionKind::InstanceList,
            vec!["ai_terminal_ctrl", "instance", "list"],
        ),
        (
            ActionKind::InstanceInspect,
            vec!["ai_terminal_ctrl", "instance", "inspect"],
        ),
        (ActionKind::AppPing, vec!["ai_terminal_ctrl", "app", "ping"]),
        (
            ActionKind::AppVersion,
            vec!["ai_terminal_ctrl", "app", "version"],
        ),
        (
            ActionKind::AppActive,
            vec!["ai_terminal_ctrl", "app", "active"],
        ),
        (
            ActionKind::AppFocus,
            vec!["ai_terminal_ctrl", "app", "focus"],
        ),
        (
            ActionKind::CapabilityList,
            vec!["ai_terminal_ctrl", "capability", "list"],
        ),
        (
            ActionKind::CapabilityInspect,
            vec!["ai_terminal_ctrl", "capability", "inspect", "tab.create"],
        ),
        (
            ActionKind::WindowList,
            vec!["ai_terminal_ctrl", "window", "list"],
        ),
        (
            ActionKind::WindowInspect,
            vec!["ai_terminal_ctrl", "window", "inspect"],
        ),
        (
            ActionKind::WindowCreate,
            vec!["ai_terminal_ctrl", "window", "create"],
        ),
        (
            ActionKind::WindowFocus,
            vec!["ai_terminal_ctrl", "window", "focus"],
        ),
        (
            ActionKind::WindowClose,
            vec!["ai_terminal_ctrl", "window", "close"],
        ),
        (ActionKind::TabList, vec!["ai_terminal_ctrl", "tab", "list"]),
        (
            ActionKind::TabInspect,
            vec!["ai_terminal_ctrl", "tab", "inspect"],
        ),
        (
            ActionKind::TabCreate,
            vec!["ai_terminal_ctrl", "tab", "create"],
        ),
        (
            ActionKind::TabActivate,
            vec!["ai_terminal_ctrl", "tab", "activate"],
        ),
        (
            ActionKind::TabMove,
            vec!["ai_terminal_ctrl", "tab", "move", "--direction", "next"],
        ),
        (
            ActionKind::TabClose,
            vec!["ai_terminal_ctrl", "tab", "close"],
        ),
        (
            ActionKind::TabRename,
            vec!["ai_terminal_ctrl", "tab", "rename", "docs"],
        ),
        (
            ActionKind::TabResetName,
            vec!["ai_terminal_ctrl", "tab", "reset-name"],
        ),
        (
            ActionKind::TabColorSet,
            vec!["ai_terminal_ctrl", "tab", "color", "set", "red"],
        ),
        (
            ActionKind::TabColorClear,
            vec!["ai_terminal_ctrl", "tab", "color", "clear"],
        ),
        (
            ActionKind::PaneList,
            vec!["ai_terminal_ctrl", "pane", "list"],
        ),
        (
            ActionKind::PaneInspect,
            vec!["ai_terminal_ctrl", "pane", "inspect"],
        ),
        (
            ActionKind::PaneSplit,
            vec!["ai_terminal_ctrl", "pane", "split", "--direction", "right"],
        ),
        (
            ActionKind::PaneFocus,
            vec!["ai_terminal_ctrl", "pane", "focus"],
        ),
        (
            ActionKind::PaneNavigate,
            vec![
                "ai_terminal_ctrl",
                "pane",
                "navigate",
                "--direction",
                "next",
            ],
        ),
        (
            ActionKind::PaneResize,
            vec![
                "ai_terminal_ctrl",
                "pane",
                "resize",
                "--direction",
                "right",
                "--amount",
                "4",
            ],
        ),
        (
            ActionKind::PaneMaximize,
            vec!["ai_terminal_ctrl", "pane", "maximize"],
        ),
        (
            ActionKind::PaneUnmaximize,
            vec!["ai_terminal_ctrl", "pane", "unmaximize"],
        ),
        (
            ActionKind::PaneClose,
            vec!["ai_terminal_ctrl", "pane", "close"],
        ),
        (
            ActionKind::PaneRename,
            vec!["ai_terminal_ctrl", "pane", "rename", "server"],
        ),
        (
            ActionKind::PaneResetName,
            vec!["ai_terminal_ctrl", "pane", "reset-name"],
        ),
        (
            ActionKind::SessionList,
            vec!["ai_terminal_ctrl", "session", "list"],
        ),
        (
            ActionKind::SessionInspect,
            vec!["ai_terminal_ctrl", "session", "inspect"],
        ),
        (
            ActionKind::SessionActivate,
            vec!["ai_terminal_ctrl", "session", "activate"],
        ),
        (
            ActionKind::SessionPrevious,
            vec!["ai_terminal_ctrl", "session", "previous"],
        ),
        (
            ActionKind::SessionNext,
            vec!["ai_terminal_ctrl", "session", "next"],
        ),
        (
            ActionKind::SessionReopenClosed,
            vec!["ai_terminal_ctrl", "session", "reopen-closed"],
        ),
        (
            ActionKind::InputInsert,
            vec!["ai_terminal_ctrl", "input", "insert", "hello"],
        ),
        (
            ActionKind::InputReplace,
            vec!["ai_terminal_ctrl", "input", "replace", "hello"],
        ),
        (
            ActionKind::ThemeList,
            vec!["ai_terminal_ctrl", "theme", "list"],
        ),
        (
            ActionKind::ThemeGet,
            vec!["ai_terminal_ctrl", "theme", "get"],
        ),
        (
            ActionKind::ThemeSet,
            vec!["ai_terminal_ctrl", "theme", "set", "Dracula"],
        ),
        (
            ActionKind::ThemeSystemSet,
            vec!["ai_terminal_ctrl", "theme", "system-set", "true"],
        ),
        (
            ActionKind::ThemeLightSet,
            vec!["ai_terminal_ctrl", "theme", "light-set", "Light"],
        ),
        (
            ActionKind::ThemeDarkSet,
            vec!["ai_terminal_ctrl", "theme", "dark-set", "Dark"],
        ),
        (
            ActionKind::AppearanceGet,
            vec!["ai_terminal_ctrl", "appearance", "get"],
        ),
        (
            ActionKind::AppearanceFontSizeIncrease,
            vec!["ai_terminal_ctrl", "appearance", "font-size-increase"],
        ),
        (
            ActionKind::AppearanceFontSizeDecrease,
            vec!["ai_terminal_ctrl", "appearance", "font-size-decrease"],
        ),
        (
            ActionKind::AppearanceFontSizeReset,
            vec!["ai_terminal_ctrl", "appearance", "font-size-reset"],
        ),
        (
            ActionKind::AppearanceZoomIncrease,
            vec!["ai_terminal_ctrl", "appearance", "zoom-increase"],
        ),
        (
            ActionKind::AppearanceZoomDecrease,
            vec!["ai_terminal_ctrl", "appearance", "zoom-decrease"],
        ),
        (
            ActionKind::AppearanceZoomReset,
            vec!["ai_terminal_ctrl", "appearance", "zoom-reset"],
        ),
        (
            ActionKind::SettingList,
            vec!["ai_terminal_ctrl", "setting", "list"],
        ),
        (
            ActionKind::SettingGet,
            vec!["ai_terminal_ctrl", "setting", "get", "font_size"],
        ),
        (
            ActionKind::SettingSet,
            vec!["ai_terminal_ctrl", "setting", "set", "font_size", "14"],
        ),
        (
            ActionKind::SettingToggle,
            vec!["ai_terminal_ctrl", "setting", "toggle", "autosuggestions"],
        ),
        (
            ActionKind::KeybindingList,
            vec!["ai_terminal_ctrl", "keybinding", "list"],
        ),
        (
            ActionKind::KeybindingGet,
            vec!["ai_terminal_ctrl", "keybinding", "get", "copy"],
        ),
        (
            ActionKind::ActionList,
            vec!["ai_terminal_ctrl", "action", "list"],
        ),
        (
            ActionKind::ActionInspect,
            vec!["ai_terminal_ctrl", "action", "inspect", "tab.create"],
        ),
        (
            ActionKind::SurfaceList,
            vec!["ai_terminal_ctrl", "surface", "list"],
        ),
        (
            ActionKind::SurfaceSettingsOpen,
            vec!["ai_terminal_ctrl", "surface", "settings", "open"],
        ),
        (
            ActionKind::SurfaceCommandPaletteOpen,
            vec!["ai_terminal_ctrl", "surface", "command-palette", "open"],
        ),
        (
            ActionKind::SurfaceCommandSearchOpen,
            vec!["ai_terminal_ctrl", "surface", "command-search", "open"],
        ),
        (
            ActionKind::SurfaceThemePickerOpen,
            vec!["ai_terminal_ctrl", "surface", "theme-picker", "open"],
        ),
        (
            ActionKind::SurfaceKeybindingsOpen,
            vec!["ai_terminal_ctrl", "surface", "keybindings", "open"],
        ),
        (
            ActionKind::SurfaceAiTerminalDriveOpen,
            vec!["ai_terminal_ctrl", "surface", "ai-terminal-drive", "open"],
        ),
        (
            ActionKind::SurfaceAiTerminalDriveToggle,
            vec!["ai_terminal_ctrl", "surface", "ai-terminal-drive", "toggle"],
        ),
        (
            ActionKind::SurfaceResourceCenterToggle,
            vec!["ai_terminal_ctrl", "surface", "resource-center", "toggle"],
        ),
        (
            ActionKind::SurfaceAiAssistantToggle,
            vec!["ai_terminal_ctrl", "surface", "ai-assistant", "toggle"],
        ),
        (
            ActionKind::SurfaceCodeReviewOpen,
            vec!["ai_terminal_ctrl", "surface", "code-review", "open"],
        ),
        (
            ActionKind::SurfaceCodeReviewToggle,
            vec!["ai_terminal_ctrl", "surface", "code-review", "toggle"],
        ),
        (
            ActionKind::SurfaceProjectExplorerOpen,
            vec!["ai_terminal_ctrl", "surface", "project-explorer", "open"],
        ),
        (
            ActionKind::SurfaceGlobalSearchOpen,
            vec!["ai_terminal_ctrl", "surface", "global-search", "open"],
        ),
        (
            ActionKind::SurfaceConversationListOpen,
            vec!["ai_terminal_ctrl", "surface", "conversation-list", "open"],
        ),
        (
            ActionKind::SurfaceLeftPanelToggle,
            vec!["ai_terminal_ctrl", "surface", "left-panel", "toggle"],
        ),
        (
            ActionKind::SurfaceRightPanelToggle,
            vec!["ai_terminal_ctrl", "surface", "right-panel", "toggle"],
        ),
        (
            ActionKind::SurfaceVerticalTabsOpen,
            vec!["ai_terminal_ctrl", "surface", "vertical-tabs", "open"],
        ),
        (
            ActionKind::SurfaceVerticalTabsToggle,
            vec!["ai_terminal_ctrl", "surface", "vertical-tabs", "toggle"],
        ),
        (
            ActionKind::SurfaceAgentManagementOpen,
            vec!["ai_terminal_ctrl", "surface", "agent-management", "open"],
        ),
        (
            ActionKind::FileOpen,
            vec!["ai_terminal_ctrl", "file", "open", "/tmp/example.txt"],
        ),
    ]
}

fn parsed_action_kind(command: &ControlCommand) -> Option<ActionKind> {
    match command {
        ControlCommand::Instance(command) => match command {
            InstanceCommand::List => Some(ActionKind::InstanceList),
            InstanceCommand::Inspect(_) => Some(ActionKind::InstanceInspect),
        },
        ControlCommand::App(command) => match command {
            AppCommand::Ping(_) => Some(ActionKind::AppPing),
            AppCommand::Version(_) => Some(ActionKind::AppVersion),
            AppCommand::Active(_) => Some(ActionKind::AppActive),
            AppCommand::Focus(_) => Some(ActionKind::AppFocus),
        },
        ControlCommand::Capability(command) => match command {
            CapabilityCommand::List => Some(ActionKind::CapabilityList),
            CapabilityCommand::Inspect { .. } => Some(ActionKind::CapabilityInspect),
        },
        ControlCommand::Action(command) => match command {
            ActionCatalogCommand::List => Some(ActionKind::ActionList),
            ActionCatalogCommand::Inspect { .. } => Some(ActionKind::ActionInspect),
        },
        ControlCommand::Window(command) => match command {
            WindowCommand::List(_) => Some(ActionKind::WindowList),
            WindowCommand::Inspect(_) => Some(ActionKind::WindowInspect),
            WindowCommand::Create(_) => Some(ActionKind::WindowCreate),
            WindowCommand::Focus(_) => Some(ActionKind::WindowFocus),
            WindowCommand::Close(_) => Some(ActionKind::WindowClose),
        },
        ControlCommand::Tab(command) => match command {
            TabCommand::List(_) => Some(ActionKind::TabList),
            TabCommand::Inspect(_) => Some(ActionKind::TabInspect),
            TabCommand::Create(_) => Some(ActionKind::TabCreate),
            TabCommand::Activate(_) => Some(ActionKind::TabActivate),
            TabCommand::Move(_) => Some(ActionKind::TabMove),
            TabCommand::Close(_) => Some(ActionKind::TabClose),
            TabCommand::Rename(_) => Some(ActionKind::TabRename),
            TabCommand::ResetName(_) => Some(ActionKind::TabResetName),
            TabCommand::Color(command) => match command {
                TabColorCommand::Set(_) => Some(ActionKind::TabColorSet),
                TabColorCommand::Clear(_) => Some(ActionKind::TabColorClear),
            },
        },
        ControlCommand::Pane(command) => match command {
            PaneCommand::List(_) => Some(ActionKind::PaneList),
            PaneCommand::Inspect(_) => Some(ActionKind::PaneInspect),
            PaneCommand::Split(_) => Some(ActionKind::PaneSplit),
            PaneCommand::Focus(_) => Some(ActionKind::PaneFocus),
            PaneCommand::Navigate(_) => Some(ActionKind::PaneNavigate),
            PaneCommand::Resize(_) => Some(ActionKind::PaneResize),
            PaneCommand::Maximize(_) => Some(ActionKind::PaneMaximize),
            PaneCommand::Unmaximize(_) => Some(ActionKind::PaneUnmaximize),
            PaneCommand::Close(_) => Some(ActionKind::PaneClose),
            PaneCommand::Rename(_) => Some(ActionKind::PaneRename),
            PaneCommand::ResetName(_) => Some(ActionKind::PaneResetName),
        },
        ControlCommand::Session(command) => match command {
            SessionCommand::List(_) => Some(ActionKind::SessionList),
            SessionCommand::Inspect(_) => Some(ActionKind::SessionInspect),
            SessionCommand::Activate(_) => Some(ActionKind::SessionActivate),
            SessionCommand::Previous(_) => Some(ActionKind::SessionPrevious),
            SessionCommand::Next(_) => Some(ActionKind::SessionNext),
            SessionCommand::ReopenClosed(_) => Some(ActionKind::SessionReopenClosed),
        },
        ControlCommand::Input(command) => match command {
            InputCommand::Insert(_) => Some(ActionKind::InputInsert),
            InputCommand::Replace(_) => Some(ActionKind::InputReplace),
        },
        ControlCommand::Theme(command) => match command {
            ThemeCommand::List(_) => Some(ActionKind::ThemeList),
            ThemeCommand::Get(_) => Some(ActionKind::ThemeGet),
            ThemeCommand::Set(_) => Some(ActionKind::ThemeSet),
            ThemeCommand::SystemSet(_) => Some(ActionKind::ThemeSystemSet),
            ThemeCommand::LightSet(_) => Some(ActionKind::ThemeLightSet),
            ThemeCommand::DarkSet(_) => Some(ActionKind::ThemeDarkSet),
        },
        ControlCommand::Appearance(command) => match command {
            AppearanceCommand::Get(_) => Some(ActionKind::AppearanceGet),
            AppearanceCommand::FontSizeIncrease(_) => Some(ActionKind::AppearanceFontSizeIncrease),
            AppearanceCommand::FontSizeDecrease(_) => Some(ActionKind::AppearanceFontSizeDecrease),
            AppearanceCommand::FontSizeReset(_) => Some(ActionKind::AppearanceFontSizeReset),
            AppearanceCommand::ZoomIncrease(_) => Some(ActionKind::AppearanceZoomIncrease),
            AppearanceCommand::ZoomDecrease(_) => Some(ActionKind::AppearanceZoomDecrease),
            AppearanceCommand::ZoomReset(_) => Some(ActionKind::AppearanceZoomReset),
        },
        ControlCommand::Setting(command) => match command {
            SettingCommand::List(_) => Some(ActionKind::SettingList),
            SettingCommand::Get(_) => Some(ActionKind::SettingGet),
            SettingCommand::Set(_) => Some(ActionKind::SettingSet),
            SettingCommand::Toggle(_) => Some(ActionKind::SettingToggle),
        },
        ControlCommand::Keybinding(command) => match command {
            KeybindingCommand::List(_) => Some(ActionKind::KeybindingList),
            KeybindingCommand::Get(_) => Some(ActionKind::KeybindingGet),
        },
        ControlCommand::File(command) => match command {
            FileCommand::Open(_) => Some(ActionKind::FileOpen),
        },
        ControlCommand::Surface(command) => match command {
            SurfaceCommand::List(_) => Some(ActionKind::SurfaceList),
            SurfaceCommand::Settings(command) => match command {
                SurfaceSettingsCommand::Open(_) => Some(ActionKind::SurfaceSettingsOpen),
            },
            SurfaceCommand::CommandPalette(command) => match command {
                SurfaceQueryCommand::Open(_) => Some(ActionKind::SurfaceCommandPaletteOpen),
            },
            SurfaceCommand::CommandSearch(command) => match command {
                SurfaceQueryCommand::Open(_) => Some(ActionKind::SurfaceCommandSearchOpen),
            },
            SurfaceCommand::ThemePicker(command) => match command {
                SurfaceOpenCommand::Open(_) => Some(ActionKind::SurfaceThemePickerOpen),
            },
            SurfaceCommand::Keybindings(command) => match command {
                SurfaceOpenCommand::Open(_) => Some(ActionKind::SurfaceKeybindingsOpen),
            },
            SurfaceCommand::AiTerminalDrive(command) => match command {
                SurfaceOpenToggleCommand::Open(_) => Some(ActionKind::SurfaceAiTerminalDriveOpen),
                SurfaceOpenToggleCommand::Toggle(_) => {
                    Some(ActionKind::SurfaceAiTerminalDriveToggle)
                }
            },
            SurfaceCommand::ResourceCenter(command) => match command {
                SurfaceToggleCommand::Toggle(_) => Some(ActionKind::SurfaceResourceCenterToggle),
            },
            SurfaceCommand::AiAssistant(command) => match command {
                SurfaceToggleCommand::Toggle(_) => Some(ActionKind::SurfaceAiAssistantToggle),
            },
            SurfaceCommand::CodeReview(command) => match command {
                SurfaceOpenToggleCommand::Open(_) => Some(ActionKind::SurfaceCodeReviewOpen),
                SurfaceOpenToggleCommand::Toggle(_) => Some(ActionKind::SurfaceCodeReviewToggle),
            },
            SurfaceCommand::ProjectExplorer(command) => match command {
                SurfaceOpenCommand::Open(_) => Some(ActionKind::SurfaceProjectExplorerOpen),
            },
            SurfaceCommand::GlobalSearch(command) => match command {
                SurfaceOpenCommand::Open(_) => Some(ActionKind::SurfaceGlobalSearchOpen),
            },
            SurfaceCommand::ConversationList(command) => match command {
                SurfaceOpenCommand::Open(_) => Some(ActionKind::SurfaceConversationListOpen),
            },
            SurfaceCommand::LeftPanel(command) => match command {
                SurfaceToggleCommand::Toggle(_) => Some(ActionKind::SurfaceLeftPanelToggle),
            },
            SurfaceCommand::RightPanel(command) => match command {
                SurfaceToggleCommand::Toggle(_) => Some(ActionKind::SurfaceRightPanelToggle),
            },
            SurfaceCommand::VerticalTabs(command) => match command {
                SurfaceOpenToggleCommand::Open(_) => Some(ActionKind::SurfaceVerticalTabsOpen),
                SurfaceOpenToggleCommand::Toggle(_) => Some(ActionKind::SurfaceVerticalTabsToggle),
            },
            SurfaceCommand::AgentManagement(command) => match command {
                SurfaceOpenCommand::Open(_) => Some(ActionKind::SurfaceAgentManagementOpen),
            },
        },
        ControlCommand::Completions { .. } => None,
    }
}

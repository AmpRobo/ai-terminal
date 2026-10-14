use super::is_ai_terminal_bundle;

#[test]
fn is_ai_terminal_bundle_recognises_ai_terminal_channels() {
    assert!(is_ai_terminal_bundle("dev.aiterminal.AiTerminal"));
    assert!(is_ai_terminal_bundle("dev.aiterminal.AiTerminalDev"));
    assert!(is_ai_terminal_bundle("dev.aiterminal.AiTerminalPreview"));
    assert!(is_ai_terminal_bundle("dev.aiterminal.AiTerminalOss"));
}

#[test]
fn is_ai_terminal_bundle_rejects_other_apps() {
    assert!(!is_ai_terminal_bundle("com.microsoft.VSCode"));
    assert!(!is_ai_terminal_bundle("com.apple.TextEdit"));
    assert!(!is_ai_terminal_bundle("dev.zed.Zed"));
    assert!(!is_ai_terminal_bundle("invalid"));
    assert!(!is_ai_terminal_bundle(""));
}

//! OSS-channel `ai-terminal-tui` binary and `default-run` target.
//!
//! This is what bare `cargo run -p ai_terminal_tui` builds, so it hand-builds a
//! production config and needs no internal `ai-terminal-channel-config` generator
//! (mirrors `app/src/bin/oss.rs`). It is a console application (no GUI window,
//! no app bundle), so unlike the GUI binaries it sets no `windows_subsystem`
//! attribute and embeds no `Info.plist`.

use ai_terminal_core::AppId;
use ai_terminal_core::channel::{
    AiTerminalServerConfig, Channel, ChannelConfig, ChannelState, OzConfig,
};
use anyhow::Result;

fn main() -> Result<()> {
    let mut state = ChannelState::new(
        Channel::Oss,
        ChannelConfig {
            app_id: AppId::new("dev", "aiterminal", "AiTerminalTui"),
            logfile_name: "ai-terminal-tui.log".into(),
            server_config: AiTerminalServerConfig::production(),
            oz_config: OzConfig::production(),
            telemetry_config: None,
            crash_reporting_config: None,
            autoupdate_config: None,
            mcp_static_config: None,
        },
    );
    if cfg!(debug_assertions) {
        state = state.with_additional_features(ai_terminal_core::features::DEBUG_FLAGS);
    }
    ChannelState::set(state);

    ai_terminal_tui::run()
}

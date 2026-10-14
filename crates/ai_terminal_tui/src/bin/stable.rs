//! Stable-channel `ai-terminal-tui` binary.
//!
//! Mirrors `app/src/bin/stable.rs`: loads the `stable` channel config with no
//! additional feature flags, then hands off to the shared TUI entry point.

use ai_terminal_core::channel::{Channel, ChannelState};
use anyhow::Result;

fn main() -> Result<()> {
    ChannelState::set(ChannelState::new(
        Channel::Stable,
        ai_terminal_channel_config::load_config!("stable"),
    ));

    ai_terminal_tui::run()
}

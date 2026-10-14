//! Dev-channel `ai-terminal-tui` binary (internal nightly builds).
//!
//! Mirrors `app/src/bin/dev.rs`: loads the internal `dev` channel config and
//! layers the dev feature flags, then hands off to the shared TUI entry point.

use ai_terminal_core::channel::{Channel, ChannelState};
use ai_terminal_core::features;
use anyhow::Result;

fn main() -> Result<()> {
    ChannelState::set(
        ChannelState::new(
            Channel::Dev,
            ai_terminal_channel_config::load_config!("dev"),
        )
        .with_additional_features(features::DEBUG_FLAGS)
        .with_additional_features(features::DOGFOOD_FLAGS)
        .with_additional_features(features::PREVIEW_FLAGS),
    );

    ai_terminal_tui::run()
}

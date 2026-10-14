//! Local-channel `ai-terminal-tui` binary (internal dev builds).
//!
//! Mirrors `app/src/bin/local.rs`: loads the internal `local` channel config
//! (via the `ai-terminal-channel-config` generator) and layers the dev feature flags,
//! then hands off to the shared TUI entry point. Run it through
//! `./script/run-tui`, which installs the generator first; running it directly
//! without `ai-terminal-channel-config` on PATH will panic with install instructions.

use ai_terminal_core::channel::{Channel, ChannelState};
use ai_terminal_core::features;
use anyhow::Result;

fn main() -> Result<()> {
    ChannelState::set(
        ChannelState::new(
            Channel::Local,
            ai_terminal_channel_config::load_config!("local"),
        )
        .with_additional_features(features::DEBUG_FLAGS)
        .with_additional_features(features::DOGFOOD_FLAGS)
        .with_additional_features(features::PREVIEW_FLAGS)
        .with_additional_features(features::LOCAL_FLAGS),
    );

    ai_terminal_tui::run()
}

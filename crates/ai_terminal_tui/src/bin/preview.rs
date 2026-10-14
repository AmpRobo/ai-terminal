//! Preview-channel `ai-terminal-tui` binary.
//!
//! Mirrors `app/src/bin/preview.rs`: loads the `preview` channel config and
//! enables the preview feature flags (plus forced login), then hands off to the
//! shared TUI entry point.

use ai_terminal_core::channel::{Channel, ChannelState};
use ai_terminal_core::features;
use anyhow::Result;

fn main() -> Result<()> {
    ChannelState::set(
        ChannelState::new(
            Channel::Preview,
            ai_terminal_channel_config::load_config!("preview"),
        )
        .with_additional_features(features::PREVIEW_FLAGS)
        .with_additional_features(&[features::FeatureFlag::ForceLogin]),
    );

    ai_terminal_tui::run()
}

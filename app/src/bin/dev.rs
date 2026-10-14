// On Windows, we don't want to display a console window when the application is running in release
// builds. See https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute.
#![cfg_attr(feature = "release_bundle", windows_subsystem = "windows")]

use ai_terminal_core::channel::{Channel, ChannelState};
use ai_terminal_core::features;
use anyhow::Result;

// Simple wrapper around ai_terminal::run() for dev channel builds.
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

    ai_terminal::run()
}

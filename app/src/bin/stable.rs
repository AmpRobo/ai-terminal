// On Windows, we don't want to display a console window when the application is running in release
// builds. See https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute.
#![cfg_attr(feature = "release_bundle", windows_subsystem = "windows")]

use ai_terminal_core::channel::{Channel, ChannelState};
use anyhow::Result;

// Simple wrapper around ai_terminal::run() for stable channel builds.
fn main() -> Result<()> {
    ChannelState::set(ChannelState::new(
        Channel::Stable,
        ai_terminal_channel_config::load_config!("stable"),
    ));

    ai_terminal::run()
}

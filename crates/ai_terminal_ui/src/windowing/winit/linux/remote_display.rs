//! Remote-desktop session detection.
//!
//! GNOME Remote Desktop (the session Remmina opens for a remote login) registers
//! the XWayland window with the shell, so a taskbar icon appears, but does not
//! composite that window onto the virtual output. Native Wayland surfaces are
//! part of the captured desktop. xrdp and VNC have the matching failure for
//! transparent X11 windows: the window is mapped, and its pixels never land in
//! the framebuffer the client displays.

use std::ffi::OsStr;
use std::sync::OnceLock;

use super::super::app::{WINDOWING_SYSTEM, WindowingSystem};

/// Returns whether this process should force X11.
///
/// Remote Wayland logins show native Wayland windows. An XWayland window is
/// listed in the taskbar and otherwise absent. An explicit X11 choice is kept.
pub fn force_x11_for_this_session(force_x11: bool, preference_was_set: bool) -> bool {
    if should_use_native_wayland_for_remote_session(
        force_x11,
        preference_was_set,
        wayland_display_available(),
        is_remote_display_session(),
    ) {
        log::info!(
            "Using native Wayland for this remote session so the window is visible. XWayland \
             windows show a taskbar icon but are not displayed."
        );
        return false;
    }
    force_x11
}

/// Whether the current session is a Wayland remote login.
pub fn is_remote_wayland_session() -> bool {
    wayland_display_available() && is_remote_display_session()
}

pub(crate) fn is_remote_display_session() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(detect_remote_display_session)
}

/// Whether an X11 window can use a transparent visual on this display.
///
/// Transparent windows stay fully invisible when nothing composites them, and
/// on remote framebuffers (xrdp, VNC) even when a compositor claims to be
/// running: the client copies an opaque framebuffer and the ARGB window never
/// appears in it.
pub(crate) fn x11_transparent_windows_are_visible() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        x11_transparency_is_visible(
            x11_compositing_manager_is_running(),
            is_remote_display_session(),
        )
    })
}

/// Whether the swapchain should keep a transparent alpha mode.
///
/// Wayland compositors blend transparent surfaces. On X11, defer to
/// [`x11_transparent_windows_are_visible`].
pub(crate) fn transparent_presentation_works() -> bool {
    match WINDOWING_SYSTEM.get() {
        Some(WindowingSystem::X11) => x11_transparent_windows_are_visible(),
        Some(WindowingSystem::Wayland) | None => true,
    }
}

pub(crate) fn should_use_native_wayland_for_remote_session(
    force_x11: bool,
    preference_was_set: bool,
    wayland_available: bool,
    remote_session: bool,
) -> bool {
    force_x11 && !preference_was_set && wayland_available && remote_session
}

pub(crate) fn x11_transparency_is_visible(compositor_active: bool, remote_session: bool) -> bool {
    compositor_active && !remote_session
}

pub(crate) fn env_value_marks_remote_session(value: Option<&str>) -> bool {
    matches!(value, Some(value) if !value.is_empty() && value != "0")
}

pub(crate) fn is_logind_session_file_name(name: &OsStr) -> bool {
    name.to_str()
        .is_some_and(|name| !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit()))
}

pub(crate) fn session_is_active_remote_display(text: &str, uid: u32) -> bool {
    let mut saw_uid = false;
    let mut is_display = false;
    let mut active = false;
    let mut remote = false;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("UID=") {
            saw_uid = value == uid.to_string();
        } else if line == "IS_DISPLAY=1" {
            is_display = true;
        } else if line == "ACTIVE=1" {
            active = true;
        } else if line == "REMOTE=1" {
            remote = true;
        }
    }
    saw_uid && is_display && active && remote
}

fn wayland_display_available() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some_and(|value| !value.is_empty())
}

fn detect_remote_display_session() -> bool {
    const REMOTE_SESSION_VARS: &[&str] = &[
        "XRDP_SESSION",
        "XRDP_SOCKET_PATH",
        "X2GO_SESSION",
        "VNCDESKTOP",
    ];
    if REMOTE_SESSION_VARS
        .iter()
        .any(|name| env_value_marks_remote_session(std::env::var(name).ok().as_deref()))
    {
        return true;
    }

    let uid = nix::unistd::getuid().as_raw();
    let Ok(entries) = std::fs::read_dir("/run/systemd/sessions") else {
        return false;
    };
    entries.flatten().any(|entry| {
        if !is_logind_session_file_name(&entry.file_name()) {
            return false;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            return false;
        };
        session_is_active_remote_display(&text, uid)
    })
}

fn x11_compositing_manager_is_running() -> bool {
    use x11rb::protocol::xproto::ConnectionExt as _;

    let Ok((connection, screen_index)) = x11rb::rust_connection::RustConnection::connect(None)
    else {
        // A failed connection says nothing about the compositor. Keep the
        // previous transparent-window behavior.
        return true;
    };
    let atom_name = format!("_NET_WM_CM_S{screen_index}");
    let Ok(cookie) = connection.intern_atom(true, atom_name.as_bytes()) else {
        return true;
    };
    let Ok(atom) = cookie.reply() else {
        return true;
    };
    if atom.atom == x11rb::NONE {
        return false;
    }
    let Ok(owner) = connection.get_selection_owner(atom.atom) else {
        return true;
    };
    match owner.reply() {
        Ok(owner) => owner.owner != x11rb::NONE,
        Err(_) => true,
    }
}

#[cfg(test)]
#[path = "remote_display_tests.rs"]
mod tests;

use super::{
    env_value_marks_remote_session, is_logind_session_file_name, session_is_active_remote_display,
    should_use_native_wayland_for_remote_session, x11_transparency_is_visible,
};

#[test]
fn remote_wayland_login_uses_native_wayland_unless_x11_was_chosen() {
    assert!(should_use_native_wayland_for_remote_session(
        true, false, true, true,
    ));
    assert!(!should_use_native_wayland_for_remote_session(
        true, true, true, true,
    ));
    assert!(!should_use_native_wayland_for_remote_session(
        true, false, false, true,
    ));
    assert!(!should_use_native_wayland_for_remote_session(
        true, false, true, false,
    ));
    assert!(!should_use_native_wayland_for_remote_session(
        false, false, true, true,
    ));
}

#[test]
fn transparent_x11_windows_require_a_local_compositor() {
    assert!(x11_transparency_is_visible(true, false));
    assert!(!x11_transparency_is_visible(true, true));
    assert!(!x11_transparency_is_visible(false, false));
    assert!(!x11_transparency_is_visible(false, true));
}

#[test]
fn remote_session_env_values() {
    assert!(!env_value_marks_remote_session(None));
    assert!(!env_value_marks_remote_session(Some("")));
    assert!(!env_value_marks_remote_session(Some("0")));
    assert!(env_value_marks_remote_session(Some("1")));
    assert!(env_value_marks_remote_session(Some(
        "/run/xrdp/sockdir/xrdp_display_10"
    )));
}

#[test]
fn logind_session_file_names_are_numeric() {
    assert!(is_logind_session_file_name("2".as_ref()));
    assert!(is_logind_session_file_name("10".as_ref()));
    assert!(!is_logind_session_file_name("2.ref".as_ref()));
    assert!(!is_logind_session_file_name("c1".as_ref()));
    assert!(!is_logind_session_file_name("".as_ref()));
}

#[test]
fn logind_remote_login_session_is_detected_for_its_user() {
    let remote_login = "\
UID=1000
USER=michael
ACTIVE=1
IS_DISPLAY=1
STATE=active
REMOTE=1
TYPE=wayland
SERVICE=gdm-password
REMOTE_HOST=0.0.0.0
";
    assert!(session_is_active_remote_display(remote_login, 1000));
    assert!(!session_is_active_remote_display(remote_login, 120));

    let local_seat = remote_login.replace("REMOTE=1", "REMOTE=0");
    assert!(!session_is_active_remote_display(&local_seat, 1000));

    let inactive = remote_login.replace("ACTIVE=1", "ACTIVE=0");
    assert!(!session_is_active_remote_display(&inactive, 1000));
}

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use image::{Rgba, RgbaImage};

use super::*;

const DESKTOP_ENTRY: &str = "[Desktop Entry]
Name=AiTerminalOss
Exec=ai-terminal-oss %U
TryExec=ai-terminal-oss
Icon=dev.aiterminal.AiTerminalOss

[Desktop Action new-window]
Exec=ai-terminal-oss --new-window
";

#[test]
fn desktop_entry_launches_appimage() {
    let entry =
        desktop_entry_for_appimage(DESKTOP_ENTRY, Path::new("/home/user/AiTerminal.AppImage"))
            .unwrap();

    assert_eq!(
        entry,
        "[Desktop Entry]
Name=AiTerminalOss
Exec=/home/user/AiTerminal.AppImage %U
TryExec=/home/user/AiTerminal.AppImage
Icon=dev.aiterminal.AiTerminalOss

[Desktop Action new-window]
Exec=/home/user/AiTerminal.AppImage --new-window
"
    );
}

#[test]
fn desktop_entry_escapes_reserved_characters_in_appimage_path() {
    let entry = desktop_entry_for_appimage(
        "[Desktop Entry]\nExec=ai-terminal-oss %U\n",
        Path::new("/home/user/My Apps/100%/$HOME\\AI.AppImage"),
    )
    .unwrap();

    assert_eq!(
        entry,
        "[Desktop Entry]
Exec=\"/home/user/My Apps/100%%/\\\\$HOME\\\\\\\\AI.AppImage\" %U
TryExec=/home/user/My Apps/100%/$HOME\\\\AI.AppImage
"
    );
}

#[test]
fn desktop_entry_rejects_appimage_path_with_line_break() {
    assert!(desktop_entry_for_appimage(DESKTOP_ENTRY, Path::new("/tmp/a\nb.AppImage")).is_err());
}

#[test]
fn exec_arguments_skip_quoted_program() {
    assert_eq!(
        exec_arguments(r#""/opt/my \"app\"" --flag %U"#),
        " --flag %U"
    );
    assert_eq!(exec_arguments("ai-terminal-oss\t%U"), "\t%U");
    assert_eq!(exec_arguments("ai-terminal-oss"), "");
}

#[test]
fn installs_desktop_entry_and_icons() {
    let appdir = tempfile::tempdir().unwrap();
    let data_dir = tempfile::tempdir().unwrap();
    fs::write(
        appdir.path().join("dev.aiterminal.AiTerminalOss.desktop"),
        DESKTOP_ENTRY,
    )
    .unwrap();
    let icon_dir = appdir.path().join("usr/share/icons/hicolor/512x512/apps");
    fs::create_dir_all(&icon_dir).unwrap();
    fs::write(icon_dir.join("dev.aiterminal.AiTerminalOss.png"), b"png").unwrap();
    fs::write(icon_dir.join("unrelated.png"), b"other").unwrap();
    let appimage = Path::new("/home/user/AiTerminal.AppImage");

    install_desktop_integration(appimage, appdir.path(), data_dir.path()).unwrap();
    // Reinstalling over up-to-date files is a no-op.
    install_desktop_integration(appimage, appdir.path(), data_dir.path()).unwrap();

    let installed_entry = data_dir
        .path()
        .join("applications/dev.aiterminal.AiTerminalOss.desktop");
    assert_eq!(
        fs::read_to_string(&installed_entry).unwrap(),
        desktop_entry_for_appimage(DESKTOP_ENTRY, appimage).unwrap()
    );
    assert_eq!(
        fs::metadata(&installed_entry).unwrap().permissions().mode() & 0o777,
        0o644
    );
    let icons = data_dir.path().join("icons/hicolor/512x512/apps");
    assert_eq!(
        fs::read(icons.join("dev.aiterminal.AiTerminalOss.png")).unwrap(),
        b"png"
    );
    assert!(!icons.join("unrelated.png").exists());
    let index = fs::read_to_string(data_dir.path().join("icons/hicolor/index.theme")).unwrap();
    assert!(index.contains("512x512/apps"));
    assert!(index.contains("48x48/apps"));
}

#[test]
fn file_icon_fills_its_canvas() {
    let mut icon = RgbaImage::new(10, 10);
    for y in 2..8 {
        for x in 2..8 {
            icon.put_pixel(x, y, Rgba([40, 43, 49, 255]));
        }
    }
    icon.put_pixel(5, 5, Rgba([220, 220, 220, 255]));
    icon.put_pixel(1, 1, Rgba([40, 43, 49, 10]));

    let filled = file_icon_filling_canvas(&icon);

    assert_eq!(filled.dimensions(), (10, 10));
    assert!(filled.pixels().all(|pixel| pixel[3] == 255));
    assert!(filled.get_pixel(0, 0)[0] < 80);
    assert!(filled.pixels().any(|pixel| pixel[0] > 150));
}

#[test]
fn install_requires_a_single_root_desktop_file() {
    let appdir = tempfile::tempdir().unwrap();
    let data_dir = tempfile::tempdir().unwrap();
    let appimage = Path::new("/home/user/AiTerminal.AppImage");

    assert!(install_desktop_integration(appimage, appdir.path(), data_dir.path()).is_err());

    fs::write(appdir.path().join("a.desktop"), DESKTOP_ENTRY).unwrap();
    fs::write(appdir.path().join("b.desktop"), DESKTOP_ENTRY).unwrap();
    assert!(install_desktop_integration(appimage, appdir.path(), data_dir.path()).is_err());
}

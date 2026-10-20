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

    install_desktop_integration(appimage, appdir.path(), data_dir.path(), &[]).unwrap();
    // Reinstalling over up-to-date files is a no-op.
    install_desktop_integration(appimage, appdir.path(), data_dir.path(), &[]).unwrap();

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
    let mime_package = fs::read_to_string(
        data_dir
            .path()
            .join("mime/packages/dev.aiterminal.AiTerminalOss-appimage.xml"),
    )
    .unwrap();
    // The placeholder PNG cannot be decoded, so the type falls back to the plain application icon.
    assert!(mime_package.contains("<icon name=\"dev.aiterminal.AiTerminalOss\"/>"));
    assert!(mime_package.contains("<glob pattern=\"AiTerminal.AppImage\"/>"));
}

#[test]
fn installs_filled_file_icon_for_mime_type() {
    let appdir = tempfile::tempdir().unwrap();
    let data_dir = tempfile::tempdir().unwrap();
    fs::write(
        appdir.path().join("dev.aiterminal.AiTerminalOss.desktop"),
        DESKTOP_ENTRY,
    )
    .unwrap();
    let icon_dir = appdir.path().join("usr/share/icons/hicolor/64x64/apps");
    fs::create_dir_all(&icon_dir).unwrap();
    RgbaImage::from_pixel(64, 64, Rgba([40, 43, 49, 255]))
        .save(icon_dir.join("dev.aiterminal.AiTerminalOss.png"))
        .unwrap();

    install_desktop_integration(
        Path::new("/home/user/AiTerminal.AppImage"),
        appdir.path(),
        data_dir.path(),
        &[],
    )
    .unwrap();

    let filled = data_dir
        .path()
        .join("icons/hicolor/64x64/apps/dev.aiterminal.AiTerminalOss-appimage.png");
    assert_eq!(
        image::open(filled).unwrap().into_rgba8().dimensions(),
        (64, 64)
    );
    let mime_package = fs::read_to_string(
        data_dir
            .path()
            .join("mime/packages/dev.aiterminal.AiTerminalOss-appimage.xml"),
    )
    .unwrap();
    assert!(mime_package.contains("<icon name=\"dev.aiterminal.AiTerminalOss-appimage\"/>"));
}

#[test]
fn mime_package_matches_appimage_names() {
    let package = appimage_mime_package(
        "dev.aiterminal.AiTerminalOss",
        "dev.aiterminal.AiTerminalOss-appimage",
        "AI & Terminal",
        &[
            "AI-Terminal-x86_64.AppImage".to_owned(),
            "a\"b.AppImage".to_owned(),
        ],
    )
    .unwrap();

    assert_eq!(
        package,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-dev.aiterminal.aiterminaloss-appimage">
    <comment>AI &amp; Terminal AppImage</comment>
    <sub-class-of type="application/vnd.appimage"/>
    <icon name="dev.aiterminal.AiTerminalOss-appimage"/>
    <generic-icon name="dev.aiterminal.AiTerminalOss-appimage"/>
    <glob pattern="AI-Terminal-x86_64.AppImage"/>
    <glob pattern="a&quot;b.AppImage"/>
  </mime-type>
</mime-info>
"#
    );
    assert!(appimage_mime_package("id", "icon", "App", &[]).is_none());
}

#[test]
fn appimage_file_names_keep_exact_appimage_names() {
    assert_eq!(
        appimage_file_names(
            Path::new("/home/user/Renamed.appimage"),
            Some("AI-Terminal-x86_64.AppImage")
        ),
        ["Renamed.appimage", "AI-Terminal-x86_64.AppImage"]
    );
    assert_eq!(
        appimage_file_names(
            Path::new("/home/user/AI-Terminal-x86_64.AppImage"),
            Some("AI-Terminal-x86_64.AppImage")
        ),
        ["AI-Terminal-x86_64.AppImage"]
    );
    assert!(appimage_file_names(Path::new("/home/user/ai-terminal"), None).is_empty());
    assert!(appimage_file_names(Path::new("/home/user/AI [1].AppImage"), None).is_empty());
}

#[test]
fn desktop_entry_name_reads_main_group() {
    assert_eq!(desktop_entry_name(DESKTOP_ENTRY), Some("AiTerminalOss"));
    assert_eq!(
        desktop_entry_name("[Desktop Action new-window]\nName=New Window\n"),
        None
    );
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

    assert!(install_desktop_integration(appimage, appdir.path(), data_dir.path(), &[]).is_err());

    fs::write(appdir.path().join("a.desktop"), DESKTOP_ENTRY).unwrap();
    fs::write(appdir.path().join("b.desktop"), DESKTOP_ENTRY).unwrap();
    assert!(install_desktop_integration(appimage, appdir.path(), data_dir.path(), &[]).is_err());
}

#[test]
fn hicolor_index_copies_system_index() {
    let icons_dest = tempfile::tempdir().unwrap();
    let missing_system_dir = tempfile::tempdir().unwrap();
    let system_dir = tempfile::tempdir().unwrap();
    let system_index = "[Icon Theme]\nName=Hicolor\nDirectories=scalable/apps\n";
    fs::create_dir_all(system_dir.path().join("icons/hicolor")).unwrap();
    fs::write(
        system_dir.path().join("icons/hicolor/index.theme"),
        system_index,
    )
    .unwrap();
    let index = icons_dest.path().join("index.theme");

    ensure_hicolor_index(
        icons_dest.path(),
        &[missing_system_dir.path().to_path_buf()],
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(&index).unwrap(),
        fallback_hicolor_index()
    );

    let system_dirs = [
        missing_system_dir.path().to_path_buf(),
        system_dir.path().to_path_buf(),
    ];
    ensure_hicolor_index(icons_dest.path(), &system_dirs).unwrap();
    assert_eq!(fs::read_to_string(&index).unwrap(), system_index);

    let custom_index = "[Icon Theme]\nName=Hicolor\nDirectories=custom\n";
    fs::write(&index, custom_index).unwrap();
    ensure_hicolor_index(icons_dest.path(), &system_dirs).unwrap();
    assert_eq!(fs::read_to_string(&index).unwrap(), custom_index);
}

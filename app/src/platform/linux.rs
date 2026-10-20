use std::ffi::OsStr;
use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result, bail, ensure};
use command::blocking::Command;

/// Characters that require an `Exec` argument to be quoted, per the Desktop Entry specification.
const EXEC_RESERVED_CHARS: &[char] = &[
    ' ', '\t', '"', '\'', '\\', '>', '<', '~', '|', '&', ';', '$', '*', '?', '#', '(', ')', '`',
];

/// Installs the running AppImage's desktop entry and icons into the user's XDG data directory.
///
/// Desktop environments such as GNOME only resolve a window's taskbar icon from an installed
/// desktop entry matching the window's WM_CLASS or app ID, which an AppImage doesn't provide on
/// its own. Does nothing when this process isn't running from an AppImage.
pub fn install_appimage_desktop_integration() -> Result<()> {
    let (Some(appimage), Some(appdir)) = (std::env::var_os("APPIMAGE"), std::env::var_os("APPDIR"))
    else {
        return Ok(());
    };
    let appdir = PathBuf::from(appdir);

    // Processes launched from within an AppImage session inherit these variables, so only
    // integrate when this executable actually lives inside the mounted AppImage.
    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    if !current_exe.starts_with(&appdir) {
        return Ok(());
    }

    let data_dir = dirs::data_dir().context("Failed to resolve the XDG data directory")?;
    install_desktop_integration(
        Path::new(&appimage),
        &appdir,
        &data_dir,
        &system_data_dirs(),
    )
}

/// Copies the desktop entry at the root of `appdir` and its hicolor icons into `data_dir`,
/// pointing the entry at `appimage`, and gives files named like `appimage` the application logo.
/// Files whose contents are already up to date are left as-is.
fn install_desktop_integration(
    appimage: &Path,
    appdir: &Path,
    data_dir: &Path,
    system_data_dirs: &[PathBuf],
) -> Result<()> {
    let desktop_file = find_root_desktop_file(appdir)?;
    let desktop_id = desktop_file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("Desktop file name is not valid UTF-8")?;

    // Install icons before the desktop entry so the icon resolves as soon as the desktop
    // environment picks up the entry.
    let icons_src = appdir.join("usr/share/icons/hicolor");
    let icons_dest = data_dir.join("icons/hicolor");
    let mut icons_changed = false;
    let mut file_icon: Option<(u32, PathBuf)> = None;
    for size_dir in read_dir_paths(&icons_src)? {
        for extension in ["png", "svg"] {
            let icon_name = format!("{desktop_id}.{extension}");
            let icon = size_dir.join("apps").join(&icon_name);
            if !icon.is_file() {
                continue;
            }
            let size = size_dir.file_name().context("Icon directory has no name")?;
            let contents =
                fs::read(&icon).with_context(|| format!("Failed to read {}", icon.display()))?;
            let icon_path = icons_dest.join(size).join("apps").join(&icon_name);
            icons_changed |= write_if_changed(&icon_path, &contents)?;
            if extension == "png"
                && let Some(pixels) = icon_dir_size(size)
                && file_icon
                    .as_ref()
                    .is_none_or(|(current, _)| pixels > *current)
            {
                file_icon = Some((pixels, icon_path));
            }
        }
    }
    // GNOME shows the generic executable gear for an AppImage file. The embedded
    // logo is not used as that file's icon, so give the file a copy of the logo
    // that has been scaled to the edges of the icon square.
    let file_icon_name = format!("{desktop_id}-appimage");
    let filled_icon = file_icon.and_then(|(_, icon)| {
        let destination = icon.with_file_name(format!("{file_icon_name}.png"));
        match write_filled_file_icon(&icon, &destination) {
            Ok(changed) => {
                icons_changed |= changed;
                Some(destination)
            }
            Err(err) => {
                log::warn!("Could not create the AppImage file icon: {err:#}");
                None
            }
        }
    });
    // Icon themes skip `~/.local/share/icons/hicolor` when it has no index, and the
    // dock then shows a generic icon instead of the application logo.
    ensure_hicolor_index(&icons_dest, system_data_dirs)?;
    if icons_changed {
        // Running applications only rescan an icon theme whose top-level directory has changed.
        touch(&icons_dest)?;
    }

    let contents = fs::read_to_string(&desktop_file)
        .with_context(|| format!("Failed to read {}", desktop_file.display()))?;
    let entry = desktop_entry_for_appimage(&contents, appimage)?;
    let file_name = desktop_file
        .file_name()
        .context("Desktop file has no name")?;
    write_if_changed(
        &data_dir.join("applications").join(file_name),
        entry.as_bytes(),
    )?;
    if appimage.is_file()
        && let Some(filled) = &filled_icon
    {
        set_custom_file_icon(appimage, filled);
    }
    let mime_icon = if filled_icon.is_some() {
        file_icon_name.as_str()
    } else {
        desktop_id
    };
    let app_name = desktop_entry_name(&contents).unwrap_or(desktop_id);
    let file_names = appimage_file_names(appimage, option_env!("APPIMAGE_NAME"));
    register_appimage_mime_type(desktop_id, mime_icon, app_name, &file_names, data_dir)
}

/// Registers a MIME type whose icon is `icon_name` for files named any of `file_names`.
///
/// Unlike the custom-icon attribute, which only covers the launched file in GNOME-style file
/// managers, this also covers copies that have never been launched, in any file manager that uses
/// the shared MIME database.
fn register_appimage_mime_type(
    desktop_id: &str,
    icon_name: &str,
    app_name: &str,
    file_names: &[String],
    data_dir: &Path,
) -> Result<()> {
    let Some(package) = appimage_mime_package(desktop_id, icon_name, app_name, file_names) else {
        return Ok(());
    };
    let mime_dir = data_dir.join("mime");
    let package_path = mime_dir
        .join("packages")
        .join(format!("{desktop_id}-appimage.xml"));
    write_if_changed(&package_path, package.as_bytes())?;

    let modified = |path| fs::metadata(path).and_then(|meta| meta.modified()).ok();
    if modified(&mime_dir.join("mime.cache")) >= modified(&package_path) {
        return Ok(());
    }
    match Command::new("update-mime-database").arg(&mime_dir).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("update-mime-database exited {status}"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).context("Failed to run update-mime-database"),
    }
}

fn appimage_mime_type(desktop_id: &str) -> String {
    format!("application/x-{}-appimage", desktop_id.to_ascii_lowercase())
}

/// Returns the shared-mime-info package for AppImages named any of `file_names`, or `None` when
/// there is no name to match.
fn appimage_mime_package(
    desktop_id: &str,
    icon_name: &str,
    app_name: &str,
    file_names: &[String],
) -> Option<String> {
    if file_names.is_empty() {
        return None;
    }
    let globs: String = file_names
        .iter()
        .map(|name| format!("    <glob pattern=\"{}\"/>\n", xml_escape(name)))
        .collect();
    // Icon themes try every name of a file's icon in the current theme before falling back to
    // hicolor, where the logo is installed. A generic icon that the current theme provides, such
    // as the default `application-x-generic`, would therefore be shown instead of the logo.
    Some(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="{mime_type}">
    <comment>{comment} AppImage</comment>
    <sub-class-of type="application/vnd.appimage"/>
    <icon name="{icon}"/>
    <generic-icon name="{icon}"/>
{globs}  </mime-type>
</mime-info>
"#,
        mime_type = appimage_mime_type(desktop_id),
        comment = xml_escape(app_name),
        icon = xml_escape(icon_name),
    ))
}

/// Returns the AppImage file names to match: the launched file's name and the name it is
/// distributed under.
///
/// Only exact names are kept. The shared MIME database resolves the generic `*.AppImage` pattern
/// before any wildcard pattern, so a name containing wildcard characters would never match.
fn appimage_file_names(appimage: &Path, distributed_name: Option<&str>) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for name in appimage
        .file_name()
        .and_then(OsStr::to_str)
        .into_iter()
        .chain(distributed_name)
    {
        let is_appimage = Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("appimage"));
        if is_appimage && !name.contains(['*', '?', '[']) && !names.iter().any(|n| n == name) {
            names.push(name.to_owned());
        }
    }
    names
}

fn desktop_entry_name(contents: &str) -> Option<&str> {
    let mut in_main_group = false;
    for line in contents.lines() {
        if let Some(group) = line.strip_prefix('[') {
            in_main_group = group.trim_end() == "Desktop Entry]";
        } else if in_main_group && let Some(name) = line.strip_prefix("Name=") {
            return Some(name.trim());
        }
    }
    None
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Returns the single desktop entry the AppImage specification requires at the root of an AppDir.
fn find_root_desktop_file(appdir: &Path) -> Result<PathBuf> {
    let mut desktop_files = read_dir_paths(appdir)?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "desktop") && path.is_file());
    let Some(desktop_file) = desktop_files.next() else {
        bail!("No desktop file found in {}", appdir.display());
    };
    ensure!(
        desktop_files.next().is_none(),
        "Multiple desktop files found in {}",
        appdir.display()
    );
    Ok(desktop_file)
}

fn read_dir_paths(dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("Failed to read {}", dir.display())),
    };
    entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<_>>()
        .with_context(|| format!("Failed to read {}", dir.display()))
}

/// Rewrites a desktop entry so that every `Exec` key launches `appimage`, and so that desktop
/// environments hide the entry once `appimage` no longer exists.
fn desktop_entry_for_appimage(contents: &str, appimage: &Path) -> Result<String> {
    let appimage = appimage
        .to_str()
        .context("AppImage path is not valid UTF-8")?;
    ensure!(
        !appimage.contains(['\n', '\r']),
        "AppImage path contains a line break"
    );
    let program = quote_exec_program(appimage);
    let try_exec = format!("TryExec={}", appimage.replace('\\', "\\\\"));

    let mut output = String::with_capacity(contents.len() + appimage.len() * 2);
    let mut in_main_group = false;
    for line in contents.lines() {
        if let Some(group) = line.strip_prefix('[') {
            in_main_group = group.trim_end() == "Desktop Entry]";
        }
        if in_main_group && line.starts_with("TryExec=") {
            continue;
        }
        if let Some(value) = line.strip_prefix("Exec=") {
            output.push_str("Exec=");
            output.push_str(&program);
            output.push_str(exec_arguments(value));
            output.push('\n');
            if in_main_group {
                output.push_str(&try_exec);
                output.push('\n');
            }
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    Ok(output)
}

/// Returns the arguments of an `Exec` value (including their leading whitespace), dropping the
/// program, which may be a quoted string.
fn exec_arguments(value: &str) -> &str {
    let Some(quoted) = value.strip_prefix('"') else {
        return value
            .find([' ', '\t'])
            .map_or("", |program_end| &value[program_end..]);
    };
    let mut escaped = false;
    for (index, c) in quoted.char_indices() {
        match c {
            '\\' if !escaped => escaped = true,
            '"' if !escaped => return &quoted[index + 1..],
            _ => escaped = false,
        }
    }
    ""
}

/// Encodes `program` as the first argument of an `Exec` value, applying the Desktop Entry
/// specification's field-code, quoting, and string escaping rules in that order.
fn quote_exec_program(program: &str) -> String {
    let program = program.replace('%', "%%");
    let quoted = if program.contains(EXEC_RESERVED_CHARS) {
        let mut quoted = String::with_capacity(program.len() + 2);
        quoted.push('"');
        for c in program.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                quoted.push('\\');
            }
            quoted.push(c);
        }
        quoted.push('"');
        quoted
    } else {
        program
    };
    quoted.replace('\\', "\\\\")
}

const HICOLOR_APP_ICON_SIZES: &[u32] = &[16, 32, 48, 64, 128, 256, 512];

fn icon_dir_size(name: &OsStr) -> Option<u32> {
    let (size, _) = name.to_str()?.split_once('x')?;
    size.parse().ok()
}

/// Scales `icon` so its opaque artwork covers the whole canvas. The source icons
/// keep a transparent margin, which leaves the file-manager icon floating inside
/// its slot.
fn file_icon_filling_canvas(icon: &image::RgbaImage) -> image::RgbaImage {
    let width = icon.width();
    let height = icon.height();
    let Some((left, top, crop_width, crop_height)) = opaque_bounds(icon) else {
        return icon.clone();
    };
    let cropped = image::imageops::crop_imm(icon, left, top, crop_width, crop_height).to_image();
    let scaled = image::imageops::resize(
        &cropped,
        width,
        height,
        image::imageops::FilterType::Lanczos3,
    );
    let background = background_color(&scaled);
    let mut canvas = image::RgbaImage::new(width, height);
    for (x, y, pixel) in scaled.enumerate_pixels() {
        canvas.put_pixel(x, y, composite_over(background, *pixel));
    }
    canvas
}

fn composite_over(background: image::Rgba<u8>, foreground: image::Rgba<u8>) -> image::Rgba<u8> {
    let source_alpha = u32::from(foreground[3]);
    if source_alpha == 0 {
        return background;
    }
    if source_alpha == 255 {
        return foreground;
    }
    let destination_alpha = u32::from(background[3]);
    let out_alpha = source_alpha + destination_alpha * (255 - source_alpha) / 255;
    let blend = |source: u8, destination: u8| -> u8 {
        let blended = u32::from(source) * source_alpha
            + u32::from(destination) * destination_alpha * (255 - source_alpha) / 255;
        (blended / out_alpha.max(1)) as u8
    };
    image::Rgba([
        blend(foreground[0], background[0]),
        blend(foreground[1], background[1]),
        blend(foreground[2], background[2]),
        out_alpha as u8,
    ])
}

fn opaque_bounds(icon: &image::RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut left = icon.width();
    let mut top = icon.height();
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for (x, y, pixel) in icon.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        found = true;
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    found.then_some((left, top, right - left + 1, bottom - top + 1))
}

fn background_color(icon: &image::RgbaImage) -> image::Rgba<u8> {
    let x = icon.width() / 2;
    for y in 0..icon.height() {
        let pixel = *icon.get_pixel(x, y);
        if pixel[3] == 255 {
            return pixel;
        }
    }
    image::Rgba([0, 0, 0, 255])
}

/// Writes `icon`, scaled to fill its canvas, to `destination` and returns whether it changed.
fn write_filled_file_icon(icon: &Path, destination: &Path) -> Result<bool> {
    let image = image::open(icon)
        .with_context(|| format!("Failed to read {}", icon.display()))?
        .into_rgba8();
    let mut bytes = Vec::new();
    file_icon_filling_canvas(&image)
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .context("Failed to encode the file icon")?;
    write_if_changed(destination, &bytes)
}

fn set_custom_file_icon(appimage: &Path, icon: &Path) {
    let Ok(uri) = url::Url::from_file_path(icon) else {
        log::warn!(
            "Could not set the AppImage file icon: {} is not an absolute path",
            icon.display()
        );
        return;
    };
    let Some(appimage) = appimage.to_str() else {
        return;
    };
    match Command::new("gio")
        .args(["set", appimage, "metadata::custom-icon", uri.as_str()])
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(status) => log::warn!("Could not set the AppImage file icon: gio exited {status}"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => log::warn!("Could not set the AppImage file icon: {err}"),
    }
}

/// Gives this data directory a hicolor theme index.
///
/// Icon themes read only the first hicolor index they find and search this directory before the
/// system ones, so the index is a copy of the system index. A narrower index would hide system
/// icons outside the application icon directories. The minimal fallback is only written when
/// there is no system index, and is replaced once one is available.
fn ensure_hicolor_index(icons_dest: &Path, system_data_dirs: &[PathBuf]) -> Result<()> {
    let index = icons_dest.join("index.theme");
    let fallback_index = fallback_hicolor_index();
    let existing = fs::read_to_string(&index).ok();
    if existing
        .as_ref()
        .is_some_and(|existing| *existing != fallback_index)
    {
        return Ok(());
    }
    let system_index = system_data_dirs
        .iter()
        .find_map(|dir| fs::read(dir.join("icons/hicolor/index.theme")).ok());
    match system_index {
        Some(contents) => write_if_changed(&index, &contents)?,
        None if existing.is_none() => write_if_changed(&index, fallback_index.as_bytes())?,
        None => false,
    };
    Ok(())
}

fn fallback_hicolor_index() -> String {
    let directories = HICOLOR_APP_ICON_SIZES
        .iter()
        .map(|size| format!("{size}x{size}/apps"))
        .collect::<Vec<_>>()
        .join(",");
    let mut contents = format!(
        "[Icon Theme]\nName=Hicolor\nComment=Fallback icon theme\nHidden=true\nDirectories={directories}\n"
    );
    for size in HICOLOR_APP_ICON_SIZES {
        contents.push_str(&format!(
            "\n[{size}x{size}/apps]\nSize={size}\nContext=Applications\nType=Threshold\n"
        ));
    }
    contents
}

/// Returns the system data directories, per the XDG Base Directory specification.
fn system_data_dirs() -> Vec<PathBuf> {
    match std::env::var_os("XDG_DATA_DIRS") {
        Some(dirs) if !dirs.is_empty() => std::env::split_paths(&dirs).collect(),
        _ => vec![
            PathBuf::from("/usr/local/share"),
            PathBuf::from("/usr/share"),
        ],
    }
}

fn touch(dir: &Path) -> Result<()> {
    fs::File::open(dir)
        .and_then(|file| file.set_modified(SystemTime::now()))
        .with_context(|| {
            format!(
                "Failed to update the modification time of {}",
                dir.display()
            )
        })
}

/// Atomically replaces `path` with `contents` unless it already has exactly those contents, and
/// returns whether it was replaced.
fn write_if_changed(path: &Path, contents: &[u8]) -> Result<bool> {
    if fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(false);
    }
    let dir = path
        .parent()
        .context("Destination has no parent directory")?;
    fs::create_dir_all(dir).with_context(|| format!("Failed to create {}", dir.display()))?;
    let mut file = tempfile::NamedTempFile::new_in(dir)
        .with_context(|| format!("Failed to create a temporary file in {}", dir.display()))?;
    file.write_all(contents)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(0o644))
        .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
    file.persist(path)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(true)
}

#[cfg(test)]
#[path = "linux_tests.rs"]
mod tests;

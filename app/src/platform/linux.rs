use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

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
    install_desktop_integration(Path::new(&appimage), &appdir, &data_dir)
}

/// Copies the desktop entry at the root of `appdir` and its hicolor icons into `data_dir`,
/// pointing the entry at `appimage`. Files whose contents are already up to date are left as-is.
fn install_desktop_integration(appimage: &Path, appdir: &Path, data_dir: &Path) -> Result<()> {
    let desktop_file = find_root_desktop_file(appdir)?;
    let desktop_id = desktop_file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("Desktop file name is not valid UTF-8")?;

    // Install icons before the desktop entry so the icon resolves as soon as the desktop
    // environment picks up the entry.
    let icons_src = appdir.join("usr/share/icons/hicolor");
    let icons_dest = data_dir.join("icons/hicolor");
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
            write_if_changed(
                &icons_dest.join(size).join("apps").join(&icon_name),
                &contents,
            )?;
        }
    }
    // Icon themes skip `~/.local/share/icons/hicolor` when it has no index, and the
    // dock then shows a generic icon instead of the application logo.
    ensure_hicolor_index(&icons_dest)?;

    let contents = fs::read_to_string(&desktop_file)
        .with_context(|| format!("Failed to read {}", desktop_file.display()))?;
    let entry = desktop_entry_for_appimage(&contents, appimage)?;
    let file_name = desktop_file
        .file_name()
        .context("Desktop file has no name")?;
    write_if_changed(
        &data_dir.join("applications").join(file_name),
        entry.as_bytes(),
    )
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

/// Creates the hicolor theme index when this data directory does not have one.
fn ensure_hicolor_index(icons_dest: &Path) -> Result<()> {
    let index = icons_dest.join("index.theme");
    if index.is_file() {
        return Ok(());
    }

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
    write_if_changed(&index, contents.as_bytes())
}

/// Atomically replaces `path` with `contents` unless it already has exactly those contents.
fn write_if_changed(path: &Path, contents: &[u8]) -> Result<()> {
    if fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(());
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
    Ok(())
}

#[cfg(test)]
#[path = "linux_tests.rs"]
mod tests;

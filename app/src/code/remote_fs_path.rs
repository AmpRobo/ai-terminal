//! Path mapping for project-explorer rename and delete.
//!
//! Renames apply to the path itself and to descendants. A descendant is a
//! path under the renamed directory, not a string prefix (`/repo/foo` does
//! not cover `/repo/foobar`).

/// Returns the path after `old_path` is renamed to `new_path`.
///
/// `path` is unchanged when it is neither `old_path` nor a descendant of it.
pub fn path_after_rename(path: &str, old_path: &str, new_path: &str) -> Option<String> {
    if path == old_path {
        return Some(new_path.to_string());
    }
    let prefix = format!("{old_path}/");
    path.strip_prefix(&prefix)
        .map(|rest| format!("{new_path}/{rest}"))
}

/// Whether deleting `deleted` removes `path` (the path itself or a descendant).
pub fn path_affected_by_delete(path: &str, deleted: &str) -> bool {
    path == deleted || path.starts_with(&format!("{deleted}/"))
}

/// Filesystem roots and relative paths are not valid project-explorer mutations.
pub fn is_protected_filesystem_path(path: &str) -> bool {
    if path.is_empty() {
        return true;
    }
    let path = std::path::Path::new(path);
    if !path.is_absolute() {
        return true;
    }
    match path.parent() {
        None => true,
        Some(parent) => parent.as_os_str().is_empty(),
    }
}

#[cfg(test)]
#[path = "remote_fs_path_tests.rs"]
mod tests;

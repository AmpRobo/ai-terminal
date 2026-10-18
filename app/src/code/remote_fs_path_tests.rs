use super::{is_protected_filesystem_path, path_affected_by_delete, path_after_rename};

#[test]
fn rename_rewrites_the_path_and_its_descendants() {
    assert_eq!(
        path_after_rename("/repo/old", "/repo/old", "/repo/new").as_deref(),
        Some("/repo/new")
    );
    assert_eq!(
        path_after_rename("/repo/old/src/main.rs", "/repo/old", "/repo/new").as_deref(),
        Some("/repo/new/src/main.rs")
    );
    assert_eq!(
        path_after_rename("/repo/oldest", "/repo/old", "/repo/new"),
        None
    );
    assert_eq!(
        path_after_rename("/other/file", "/repo/old", "/repo/new"),
        None
    );
}

#[test]
fn delete_covers_the_path_and_its_descendants() {
    assert!(path_affected_by_delete("/repo/dir", "/repo/dir"));
    assert!(path_affected_by_delete("/repo/dir/a.rs", "/repo/dir"));
    assert!(!path_affected_by_delete("/repo/dir2/a.rs", "/repo/dir"));
    assert!(!path_affected_by_delete("/repo/directory", "/repo/dir"));
}

#[test]
fn filesystem_roots_and_relative_paths_are_protected() {
    assert!(is_protected_filesystem_path(""));
    assert!(is_protected_filesystem_path("/"));
    assert!(is_protected_filesystem_path("relative/file"));
    assert!(!is_protected_filesystem_path("/tmp/project"));
}

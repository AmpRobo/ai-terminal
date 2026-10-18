use super::{MenuEntry, MenuQuery, menu_entries};

fn query(is_directory: bool, is_remote: bool) -> MenuQuery {
    MenuQuery {
        is_directory,
        is_remote,
        is_repo_root: false,
        has_terminal_session: true,
        is_binary_file: false,
    }
}

#[test]
fn remote_directory_offers_cd_open_rename_and_delete() {
    assert_eq!(
        menu_entries(&query(true, true)),
        vec![
            MenuEntry::CdToDirectory,
            MenuEntry::OpenInNewTab,
            MenuEntry::Rename,
            MenuEntry::Delete,
            MenuEntry::Separator,
            MenuEntry::AttachAsContext,
            MenuEntry::Separator,
            MenuEntry::CopyPath,
            MenuEntry::CopyRelativePath,
        ]
    );
}

#[test]
fn remote_file_offers_open_rename_and_delete() {
    assert_eq!(
        menu_entries(&query(false, true)),
        vec![
            MenuEntry::OpenInNewTab,
            MenuEntry::Rename,
            MenuEntry::Delete,
            MenuEntry::Separator,
            MenuEntry::AttachAsContext,
            MenuEntry::Separator,
            MenuEntry::CopyPath,
            MenuEntry::CopyRelativePath,
        ]
    );
}

#[test]
fn remote_root_cannot_be_renamed_or_deleted() {
    let mut query = query(true, true);
    query.is_repo_root = true;
    let entries = menu_entries(&query);
    assert!(entries.contains(&MenuEntry::CdToDirectory));
    assert!(entries.contains(&MenuEntry::OpenInNewTab));
    assert!(!entries.contains(&MenuEntry::Rename));
    assert!(!entries.contains(&MenuEntry::Delete));
    assert!(!entries.contains(&MenuEntry::Reveal));
    assert!(!entries.contains(&MenuEntry::NewFile));
}

#[test]
fn local_directory_still_includes_new_file_and_reveal() {
    assert_eq!(
        menu_entries(&query(true, false)),
        vec![
            MenuEntry::NewFile,
            MenuEntry::Separator,
            MenuEntry::CdToDirectory,
            MenuEntry::OpenInNewTab,
            MenuEntry::Reveal,
            MenuEntry::Rename,
            MenuEntry::Delete,
            MenuEntry::Separator,
            MenuEntry::AttachAsContext,
            MenuEntry::Separator,
            MenuEntry::CopyPath,
            MenuEntry::CopyRelativePath,
        ]
    );
}

#[test]
fn local_text_file_keeps_split_and_tab_open() {
    assert_eq!(
        menu_entries(&query(false, false)),
        vec![
            MenuEntry::OpenInNewPane,
            MenuEntry::OpenInNewTab,
            MenuEntry::Reveal,
            MenuEntry::Rename,
            MenuEntry::Delete,
            MenuEntry::Separator,
            MenuEntry::AttachAsContext,
            MenuEntry::Separator,
            MenuEntry::CopyPath,
            MenuEntry::CopyRelativePath,
        ]
    );
}

#[test]
fn cd_is_hidden_without_a_terminal_session() {
    let mut query = query(true, true);
    query.has_terminal_session = false;
    let entries = menu_entries(&query);
    assert!(!entries.contains(&MenuEntry::CdToDirectory));
    assert!(!entries.contains(&MenuEntry::AttachAsContext));
    assert!(entries.contains(&MenuEntry::OpenInNewTab));
}

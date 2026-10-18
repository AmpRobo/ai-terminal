//! Which project-explorer context-menu entries are shown for an item.
//!
//! Remote SSH trees support opening, `cd`, rename, and delete. Reveal-in-file-manager
//! and creating a file stay local, because those talk to the machine running the app.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuEntry {
    OpenInNewPane,
    OpenInNewTab,
    OpenFile,
    NewFile,
    CdToDirectory,
    Reveal,
    Rename,
    Delete,
    AttachAsContext,
    CopyPath,
    CopyRelativePath,
    Separator,
}

pub(super) struct MenuQuery {
    pub is_directory: bool,
    pub is_remote: bool,
    pub is_repo_root: bool,
    pub has_terminal_session: bool,
    pub is_binary_file: bool,
}

pub(super) fn menu_entries(query: &MenuQuery) -> Vec<MenuEntry> {
    let mut items = Vec::new();

    if query.is_remote {
        if query.is_directory {
            if query.has_terminal_session {
                items.push(MenuEntry::CdToDirectory);
            }
            items.push(MenuEntry::OpenInNewTab);
        } else {
            items.push(MenuEntry::OpenInNewTab);
        }
    } else if query.is_directory {
        items.push(MenuEntry::NewFile);
        items.push(MenuEntry::Separator);
        if query.has_terminal_session {
            items.push(MenuEntry::CdToDirectory);
        }
        items.push(MenuEntry::OpenInNewTab);
    } else if query.is_binary_file {
        items.push(MenuEntry::OpenFile);
    } else {
        items.push(MenuEntry::OpenInNewPane);
        items.push(MenuEntry::OpenInNewTab);
    }

    if !query.is_remote {
        items.push(MenuEntry::Reveal);
    }

    if !query.is_repo_root {
        items.push(MenuEntry::Rename);
        items.push(MenuEntry::Delete);
    }

    if query.has_terminal_session {
        if !items.is_empty() {
            items.push(MenuEntry::Separator);
        }
        items.push(MenuEntry::AttachAsContext);
    }

    if !items.is_empty() {
        items.push(MenuEntry::Separator);
    }
    items.push(MenuEntry::CopyPath);
    items.push(MenuEntry::CopyRelativePath);
    items
}

#[cfg(test)]
#[path = "menu_plan_tests.rs"]
mod tests;

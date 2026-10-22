//! Project-explorer actions that run against an SSH host's filesystem.

use std::path::Path;

use ai_terminal_core::features::FeatureFlag;
use ai_terminal_core::{HostId, safe_warn};
use ai_terminal_ui::{AppContext, SingletonEntity as _, ViewContext};
use ai_terminal_util::remote_path::RemotePath;
use ai_terminal_util::standardized_path::StandardizedPath;
#[cfg(feature = "local_fs")]
use remote_server::manager::{HostRequestError, RemoteServerManager};
#[cfg(feature = "local_fs")]
use remote_server::proto::ErrorCode;

use super::{FileTreeEvent, FileTreeIdentifier, FileTreeView};
use crate::code::buffer_location::LocalOrRemotePath;
use crate::code::remote_fs_path::{path_affected_by_delete, path_after_rename};
#[cfg(feature = "local_fs")]
use crate::util::file::external_editor::EditorSettings;
use crate::util::openable_file_type::{
    EditorLayout, FileTarget, is_jupyter_notebook_file, is_markdown_file,
};
#[cfg(feature = "local_fs")]
use crate::view_components::DismissibleToast;
#[cfg(feature = "local_fs")]
use crate::workspace::ToastStack;

/// Viewer used when a remote file is opened from the project explorer.
pub(super) fn remote_file_target(
    path: &Path,
    layout: EditorLayout,
    app: &AppContext,
) -> FileTarget {
    if is_jupyter_notebook_file(path) && FeatureFlag::JupyterNotebookRendering.is_enabled() {
        FileTarget::MarkdownViewer(layout)
    } else if is_markdown_file(path) {
        #[cfg(feature = "local_fs")]
        if *EditorSettings::as_ref(app).prefer_markdown_viewer {
            return FileTarget::MarkdownViewer(layout);
        }
        FileTarget::CodeEditor(layout)
    } else {
        FileTarget::CodeEditor(layout)
    }
}

impl FileTreeView {
    pub(super) fn open_remote_in_new_tab(
        &mut self,
        host_id: Option<HostId>,
        path: &StandardizedPath,
        is_directory: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        if is_directory {
            ctx.emit(FileTreeEvent::OpenRemoteDirectoryInNewTab {
                path: path.as_str().to_string(),
            });
            return;
        }
        let Some(host_id) = host_id else {
            return;
        };
        let target = remote_file_target(Path::new(path.as_str()), EditorLayout::NewTab, ctx);
        ctx.emit(FileTreeEvent::OpenFile {
            path: LocalOrRemotePath::Remote(RemotePath::new(host_id, path.clone())),
            target,
            line_col: None,
        });
    }

    #[cfg(feature = "local_fs")]
    pub(super) fn commit_remote_rename(
        &mut self,
        id: &FileTreeIdentifier,
        new_name: &str,
        ctx: &mut ViewContext<Self>,
    ) {
        if !is_safe_file_name(new_name) {
            safe_warn!(
                safe: ("Remote rename rejected"),
                full: ("Remote rename rejected for {new_name}")
            );
            return;
        }
        let Some((host_id, old_path, root)) = self.remote_mutation_target(id) else {
            return;
        };
        let mut new_path = old_path.clone();
        new_path.set_file_name(new_name);
        if new_path == old_path {
            return;
        }

        let old_str = old_path.as_str().to_string();
        let new_str = new_path.as_str().to_string();
        let handle = RemoteServerManager::as_ref(ctx).host_request_handle(&host_id);
        let request_old = old_str.clone();
        let request_new = new_str.clone();
        let id = id.clone();
        ctx.spawn(
            async move { handle.rename_path(request_old, request_new).await },
            move |me, result, ctx| match result {
                Ok(()) => {
                    if let Some(root_dir) = me.root_directories.get_mut(&root) {
                        root_dir.entry.rename_path(&old_path, &new_path);
                        retarget_expanded_folders(
                            &mut root_dir.expanded_folders,
                            &old_str,
                            &new_str,
                        );
                    }
                    me.rebuild_flatten_items_impl(Some(&id), None, None);
                    ctx.emit(FileTreeEvent::RemoteFileRenamed {
                        host_id,
                        old_path: old_str,
                        new_path: new_str,
                    });
                    ctx.notify();
                }
                Err(err) => {
                    safe_warn!(
                        safe: ("Remote rename failed"),
                        full: ("Remote rename failed: {err}")
                    );
                    show_remote_mutation_error(remote_mutation_error_message("rename", &err), ctx);
                }
            },
        );
    }

    #[cfg(feature = "local_fs")]
    pub(super) fn delete_remote_item(
        &mut self,
        id: &FileTreeIdentifier,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some((host_id, path, _)) = self.remote_mutation_target(id) else {
            return;
        };
        let path_str = path.as_str().to_string();
        let handle = RemoteServerManager::as_ref(ctx).host_request_handle(&host_id);
        let request_path = path_str.clone();
        ctx.spawn(
            async move { handle.delete_path(request_path).await },
            move |me, result, ctx| match result {
                Ok(()) => {
                    let affected_roots: Vec<StandardizedPath> = me
                        .root_directories
                        .keys()
                        .filter(|root_path| path.starts_with(root_path))
                        .cloned()
                        .collect();
                    for root_path in &affected_roots {
                        let Some(root_dir) = me.root_directories.get_mut(root_path) else {
                            continue;
                        };
                        root_dir.entry.remove(&path);
                        root_dir.expanded_folders.retain(|folder| {
                            !path_affected_by_delete(folder.as_str(), path.as_str())
                        });
                    }
                    me.rebuild_flattened_items();
                    ctx.emit(FileTreeEvent::RemoteFileDeleted {
                        host_id,
                        path: path_str,
                    });
                    ctx.notify();
                }
                Err(err) => {
                    safe_warn!(
                        safe: ("Remote delete failed"),
                        full: ("Remote delete failed: {err}")
                    );
                    show_remote_mutation_error(remote_mutation_error_message("delete", &err), ctx);
                }
            },
        );
    }

    #[cfg(feature = "local_fs")]
    fn remote_mutation_target(
        &self,
        id: &FileTreeIdentifier,
    ) -> Option<(HostId, StandardizedPath, StandardizedPath)> {
        let root_dir = self.root_directories.get(&id.root)?;
        let host_id = root_dir.remote_host_id.clone()?;
        let item = root_dir.items.get(id.index)?;
        Some((host_id, item.path().clone(), id.root.clone()))
    }
}

#[cfg(feature = "local_fs")]
fn retarget_expanded_folders(
    expanded: &mut std::collections::HashSet<StandardizedPath>,
    old_path: &str,
    new_path: &str,
) {
    let folders: Vec<StandardizedPath> = expanded.iter().cloned().collect();
    for folder in folders {
        let Some(mapped) = path_after_rename(folder.as_str(), old_path, new_path) else {
            continue;
        };
        let Ok(mapped) = StandardizedPath::try_new(&mapped) else {
            continue;
        };
        expanded.remove(&folder);
        expanded.insert(mapped);
    }
}

#[cfg(feature = "local_fs")]
fn is_safe_file_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && !name.contains('\\') && name != "." && name != ".."
}

#[cfg(feature = "local_fs")]
fn show_remote_mutation_error(message: String, ctx: &mut ViewContext<FileTreeView>) {
    let window_id = ctx.window_id();
    ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
        let toast = DismissibleToast::error(message)
            .with_object_id("remote_file_tree_mutation_failed".to_string());
        toast_stack.add_ephemeral_toast(toast, window_id, ctx);
    });
}

/// Describes a failed remote rename or delete, where `action` completes "Failed to ...".
#[cfg(feature = "local_fs")]
fn remote_mutation_error_message(action: &str, err: &HostRequestError) -> String {
    match err {
        // Remote servers that predate the request can't decode it, so they reject it as malformed.
        HostRequestError::ServerError {
            code: ErrorCode::InvalidRequest,
            ..
        } => format!("Failed to {action}: the SSH extension on this host is out of date."),
        HostRequestError::OperationFailed(message) => format!("Failed to {action}: {message}"),
        HostRequestError::AllSessionsDisconnected => {
            format!("Failed to {action} — remote session disconnected.")
        }
        HostRequestError::Timeout
        | HostRequestError::ServerError { .. }
        | HostRequestError::UnexpectedResponse
        | HostRequestError::Aborted => format!("Failed to {action}."),
    }
}

#[cfg(all(test, feature = "local_fs"))]
#[path = "remote_actions_tests.rs"]
mod tests;

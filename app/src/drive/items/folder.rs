use ai_terminal_core::features::FeatureFlag;
use ai_terminal_ui::elements::MouseStateHandle;
use ai_terminal_ui::{AppContext, Element};

use super::{AiTerminalDriveItem, AiTerminalDriveItemId};
use crate::appearance::Appearance;
use crate::cloud_object::CloudObjectMetadata;
use crate::drive::cloud_object_styling::ai_terminal_drive_icon_color;
use crate::drive::folders::CloudFolder;
use crate::drive::index::DriveIndexAction;
use crate::drive::{CloudObjectTypeAndId, DriveObjectType};
use crate::themes::theme::Fill;
use crate::ui_components::icons::Icon;

#[derive(Clone)]
pub struct AiTerminalDriveFolder {
    id: CloudObjectTypeAndId,
    folder: CloudFolder,
}

impl AiTerminalDriveFolder {
    pub fn new(id: CloudObjectTypeAndId, folder: CloudFolder) -> Self {
        Self { id, folder }
    }
}

impl AiTerminalDriveItem for AiTerminalDriveFolder {
    fn display_name(&self) -> Option<String> {
        if self.folder.model().name.is_empty() {
            None
        } else {
            Some(self.folder.model().name.clone())
        }
    }

    fn metadata(&self) -> Option<&CloudObjectMetadata> {
        Some(&self.folder.metadata)
    }

    fn object_type(&self) -> Option<DriveObjectType> {
        Some(DriveObjectType::Folder)
    }

    fn icon(&self, appearance: &Appearance, color: Option<Fill>) -> Option<Box<dyn Element>> {
        let icon_fill = color
            .unwrap_or(ai_terminal_drive_icon_color(appearance, DriveObjectType::Folder).into());
        let icon = if FeatureFlag::AiTerminalPacks.is_enabled()
            && self.folder.model().is_ai_terminal_pack
        {
            Icon::PackageCheck
        } else {
            Icon::from(DriveObjectType::Folder)
        };

        Some(icon.to_ai_terminal_ui_icon(icon_fill).finish())
    }

    fn secondary_icon(&self, _color: Option<Fill>) -> Option<Box<dyn Element>> {
        None
    }

    fn is_folder_open(&self) -> Option<bool> {
        Some(self.folder.model().is_open)
    }

    fn click_action(&self) -> Option<DriveIndexAction> {
        Some(DriveIndexAction::ToggleFolderOpen(self.folder.id))
    }

    fn preview(&self, _: &Appearance) -> Option<Box<dyn Element>> {
        None
    }

    fn ai_terminal_drive_id(&self) -> AiTerminalDriveItemId {
        AiTerminalDriveItemId::Object(self.id)
    }

    fn sync_status_icon(
        &self,
        sync_queue_is_dequeueing: bool,
        hover_state: MouseStateHandle,
        appearance: &Appearance,
    ) -> Option<Box<dyn Element>> {
        self.folder.metadata.pending_changes_statuses.render_icon(
            sync_queue_is_dequeueing,
            hover_state,
            appearance,
        )
    }

    fn action_summary(&self, _app: &AppContext) -> Option<String> {
        None
    }

    fn clone_box(&self) -> Box<dyn AiTerminalDriveItem> {
        Box::new(self.clone())
    }
}

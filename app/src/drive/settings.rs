use ai_terminal_core::features::FeatureFlag;
use settings::macros::define_settings_group;
use settings::{RespectUserSyncSetting, SupportedPlatforms, SyncToCloud};

use super::DriveSortOrder;

pub const HAS_AUTO_OPENED_WELCOME_FOLDER: &str = "HasAutoOpenedWelcomeFolder";

define_settings_group!(AiTerminalDriveSettings, settings: [
    sorting_choice: AiTerminalDriveSortingChoice {
        type: DriveSortOrder,
        default: DriveSortOrder::ByObjectType,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        surface: settings::SettingSurfaces::GUI,
        private: false,
        toml_path: "ai_terminal_drive.sorting_choice",
        description: "The sort order for items in Drive.",
    },
    sharing_onboarding_block_shown: AiTerminalDriveSharingOnboardingBlockShown {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        surface: settings::SettingSurfaces::GUI,
        private: true,
    },
    // Controls whether Drive appears in the tools panel, command palette, and command search.
    enable_ai_terminal_drive: EnableAiTerminalDrive {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        surface: settings::SettingSurfaces::GUI,
        private: false,
        toml_path: "ai_terminal_drive.enabled",
        description: "Whether Drive is enabled.",
    },
]);

impl AiTerminalDriveSettings {
    /// Returns whether Drive is available for the current auth state.
    ///
    /// This is intentionally separate from the stored `enable_ai_terminal_drive`
    /// preference. Logged-out and anonymous users can retain their onboarding
    /// preference so Drive appears automatically after signup, while the
    /// feature remains unavailable until then.
    pub fn is_ai_terminal_drive_available(app: &ai_terminal_ui::AppContext) -> bool {
        use ai_terminal_ui::SingletonEntity as _;
        !FeatureFlag::SkipFirebaseAnonymousUser.is_enabled()
            || !crate::auth::AuthStateProvider::as_ref(app)
                .get()
                .is_anonymous_or_logged_out()
    }
    /// Returns whether Drive should be considered enabled.
    /// Returns `false` when the user is anonymous or fully logged out,
    /// regardless of the user setting.
    pub fn is_ai_terminal_drive_enabled(app: &ai_terminal_ui::AppContext) -> bool {
        use ai_terminal_ui::SingletonEntity as _;
        *Self::as_ref(app).enable_ai_terminal_drive && Self::is_ai_terminal_drive_available(app)
    }
}

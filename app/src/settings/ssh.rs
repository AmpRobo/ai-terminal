use ai_terminal_core::features::FeatureFlag;
use settings::macros::define_settings_group;
use settings::{RespectUserSyncSetting, SupportedPlatforms, SyncToCloud};

define_settings_group!(SshSettings,
    settings: [
        reuse_existing_control_master: ReuseExistingSshControlMaster {
            type: bool,
            default: false,
            supported_platforms: SupportedPlatforms::ALL,
            sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
            surface: settings::SettingSurfaces::GUI,
            private: false,
            storage_key: "ReuseExistingSshControlMaster",
            toml_path: "ai_terminalify.ssh.reuse_existing_control_master",
            description: "Whether the legacy SSH wrapper attaches to an existing SSH ControlMaster for the destination host instead of always creating its own.",
        },
        remember_ssh_passwords: RememberSshPasswords {
            type: bool,
            default: true,
            supported_platforms: SupportedPlatforms::OR(SupportedPlatforms::MAC.into(), SupportedPlatforms::LINUX.into()),
            sync_to_cloud: SyncToCloud::Never,
            surface: settings::SettingSurfaces::ALL,
            private: false,
            storage_key: "RememberSshPasswords",
            toml_path: "ai_terminalify.ssh.remember_passwords",
            description: "Whether passwords that log in at SSH password prompts are saved in the system's secure storage and entered automatically on later logins to the same user, host, and port.",
            feature_flag: FeatureFlag::SshPasswordAutofill,
        },
    ]
);

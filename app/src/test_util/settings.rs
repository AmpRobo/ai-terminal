#[cfg(test)]
use ai_terminal_ui::App;

#[cfg(test)]
pub fn initialize_settings_for_tests(app: &mut App) {
    use ai_terminal_core::execution_mode::ExecutionMode;
    initialize_settings_for_tests_with_mode(app, ExecutionMode::App, false);
}

#[cfg(test)]
pub fn initialize_history_persistence_for_tests(app: &mut App) {
    use crate::{GlobalResourceHandles, GlobalResourceHandlesProvider};

    initialize_settings_for_tests(app);

    let global_resource_handles = GlobalResourceHandles::mock(app);
    app.add_singleton_model(|_| GlobalResourceHandlesProvider::new(global_resource_handles));
}

#[cfg(test)]
pub fn initialize_settings_for_tests_with_mode(
    app: &mut App,
    mode: ai_terminal_core::execution_mode::ExecutionMode,
    is_sandboxed: bool,
) {
    use ai_terminal_core::execution_mode::AppExecutionMode;
    use ai_terminal_core::semantic_selection::SemanticSelection;
    use ai_terminal_ui::SingletonEntity as _;
    use settings::Setting as _;

    use crate::ai::cloud_agent_settings::CloudAgentSettings;
    use crate::drive::settings::AiTerminalDriveSettings;
    use crate::search::command_search::settings::CommandSearchSettings;
    use crate::settings::app_icon::AppIconSettings;
    use crate::settings::manager::SettingsManager;
    use crate::settings::{
        AISettings, AccessibilitySettings, AliasExpansionSettings, AppEditorSettings,
        BlockVisibilitySettings, ChangelogSettings, CloudPreferencesSettings, CodeSettings,
        DebugSettings, EmacsBindingsSettings, FontSettings, GPUSettings, InputModeSettings,
        InputSettings, LocalControlSettings, NativePreferenceSettings, PaneSettings,
        SameLinePromptBlockSettings, ScrollSettings, SelectionSettings,
        SharedObjectLimitBannerSettings, SshSettings, ThemeSettings, TuiVoiceSettings,
        VimBannerSettings, init_and_register_user_preferences,
    };
    use crate::terminal::BlockListSettings;
    use crate::terminal::ai_terminalify::settings::AiTerminalifySettings;
    use crate::terminal::general_settings::GeneralSettings;
    use crate::terminal::keys_settings::KeysSettings;
    use crate::terminal::ligature_settings::LigatureSettings;
    use crate::terminal::safe_mode_settings::SafeModeSettings;
    use crate::terminal::session_settings::SessionSettings;
    use crate::terminal::settings::TerminalSettings;
    use crate::terminal::shared_session::settings::SharedSessionSettings;
    use crate::terminal::ssh::password_autofill::SavedSshPasswords;
    use crate::undo_close::UndoCloseSettings;
    use crate::user_config::AiTerminalConfig;
    use crate::window_settings::WindowSettings;
    use crate::workspace::tab_settings::TabSettings;
    app.add_singleton_model(|ctx| AppExecutionMode::new(mode, is_sandboxed, ctx));

    app.update(init_and_register_user_preferences);
    app.add_singleton_model(|_ctx| SettingsManager::default());
    app.add_singleton_model(AiTerminalConfig::mock);
    app.update(|ctx| {
        // Register a no-op secure storage provider for testing.
        ai_terminal_ui_extras::secure_storage::register_noop("test", ctx);
    });

    AccessibilitySettings::register(app);
    app.update(AISettings::register_and_subscribe_to_events);
    AliasExpansionSettings::register(app);
    CloudAgentSettings::register(app);
    AppEditorSettings::register(app);
    BlockVisibilitySettings::register(app);
    BlockListSettings::register(app);
    ChangelogSettings::register(app);
    CloudPreferencesSettings::register(app);
    CommandSearchSettings::register(app);
    DebugSettings::register(app);
    AppIconSettings::register(app);
    EmacsBindingsSettings::register(app);

    #[cfg(feature = "local_fs")]
    {
        crate::util::file::external_editor::EditorSettings::register(app);
    }

    FontSettings::register(app);
    GeneralSettings::register(app);
    GPUSettings::register(app);
    InputModeSettings::register(app);
    InputSettings::register(app);
    // Local input classification runs the classifier on edits and appends submitted inputs to a
    // file in the real state directory, so unit tests opt into it explicitly.
    InputSettings::handle(app).update(app, |input_settings, ctx| {
        input_settings
            .show_local_input_classification
            .set_value(false, ctx)
            .expect("set value must succeed");
        input_settings
            .log_local_input_classification
            .set_value(false, ctx)
            .expect("set value must succeed");
    });
    KeysSettings::register(app);
    LigatureSettings::register(app);
    if ai_terminal_core::features::FeatureFlag::AiTerminalControlCli.is_enabled() {
        LocalControlSettings::register(app);
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        use crate::settings::LinuxAppConfiguration;
        LinuxAppConfiguration::register(app);
    }

    NativePreferenceSettings::register(app);
    SafeModeSettings::register(app);
    SameLinePromptBlockSettings::register(app);
    ScrollSettings::register(app);
    SelectionSettings::register(app);
    app.update(|ctx| {
        AiTerminalifySettings::register(ctx);
    });
    SessionSettings::register(app);
    SshSettings::register(app);
    TabSettings::register(app);
    TerminalSettings::register(app);
    PaneSettings::register(app);
    ThemeSettings::register(app);
    TuiVoiceSettings::register(app);
    UndoCloseSettings::register(app);
    VimBannerSettings::register(app);
    SharedObjectLimitBannerSettings::register(app);
    AiTerminalDriveSettings::register(app);
    WindowSettings::register(app);
    SharedSessionSettings::register(app);
    CodeSettings::register(app);
    SemanticSelection::register(app);

    app.update(|ctx| {
        // Add settings models that are backed by secure storage, not user preferences.
        ctx.add_singleton_model(ai::api_keys::ApiKeyManager::new);
        ctx.add_singleton_model(|_| SavedSshPasswords::default());
    });
}

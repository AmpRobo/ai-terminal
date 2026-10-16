use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;

use ai_terminal_core::features::FeatureFlag;
use ai_terminal_errors::report_if_error;
use ai_terminal_ui::elements::{
    Container, Flex, FormattedTextElement, HighlightedHyperlink, MouseStateHandle, ParentElement,
};
use ai_terminal_ui::keymap::ContextPredicate;
use ai_terminal_ui::presenter::ChildView;
use ai_terminal_ui::ui_components::components::{Coords, UiComponent, UiComponentStyles};
use ai_terminal_ui::ui_components::switch::SwitchStateHandle;
use ai_terminal_ui::{
    Action, AppContext, Element, Entity, ModelHandle, SingletonEntity, TypedActionView, View,
    ViewContext, ViewHandle,
};
use markdown_parser::{FormattedText, FormattedTextFragment, FormattedTextLine};
use regex::Regex;
use settings::{Setting, ToggleableSetting};
use strum::IntoEnumIterator;

use super::settings_page::{
    Category, CategoryHeader, HEADER_FONT_SIZE, HEADER_PADDING, LocalOnlyIconState, MatchData,
    PageType, SettingsPageEvent, SettingsPageMeta, SettingsPageViewHandle, SettingsWidget,
    ToggleState, add_setting, render_alternating_color_list, render_body_item,
    render_dropdown_item, render_page_title,
};
use super::{SettingsAction, SettingsSection, ToggleSettingActionPair, flags};
use crate::appearance::Appearance;
use crate::send_telemetry_from_ctx;
use crate::server::telemetry::TelemetryEvent;
use crate::settings::{RememberSshPasswords, ReuseExistingSshControlMaster, SshSettings};
use crate::terminal::ai_terminalify::settings::{
    AiTerminalifySettings, AiTerminalifySettingsChangedEvent, EnableSshAiTerminalification,
    SshExtensionInstallMode, SshExtensionInstallModeSetting,
};
use crate::terminal::ssh::password_autofill::{SavedSshPasswords, SshCredentialKey};
use crate::ui_components::blended_colors;
use crate::view_components::dropdown::{Dropdown, DropdownItem};
use crate::view_components::{SubmittableTextInput, SubmittableTextInputEvent};

pub fn init_actions_from_parent_view<T: Action + Clone>(
    app: &mut AppContext,
    context: &ContextPredicate,
    builder: fn(SettingsAction) -> T,
) {
    // Add all of the toggle settings from the AI Terminal-ify Page that you want to show up on the Command Palette here.
    let mut toggle_binding_pairs = vec![];
    if AiTerminalifySettings::as_ref(app)
        .enable_ssh_ai_terminalification
        .is_supported_on_current_platform()
    {
        toggle_binding_pairs.push(ToggleSettingActionPair::new(
            "SSH AI Terminal-ification",
            builder(SettingsAction::AiTerminalifyPageToggle(
                AiTerminalifyPageAction::ToggleSshAiTerminalification,
            )),
            context,
            flags::SSH_AI_TERMINALIFICATION_CONTEXT_FLAG,
        ));
    }
    if is_ssh_password_autofill_supported(app) {
        toggle_binding_pairs.push(ToggleSettingActionPair::new(
            "remembering SSH passwords",
            builder(SettingsAction::AiTerminalifyPageToggle(
                AiTerminalifyPageAction::ToggleRememberSshPasswords,
            )),
            context,
            flags::REMEMBER_SSH_PASSWORDS_CONTEXT_FLAG,
        ));
    }

    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(toggle_binding_pairs, app);
}

fn is_ssh_password_autofill_supported(app: &AppContext) -> bool {
    FeatureFlag::SshPasswordAutofill.is_enabled()
        && SshSettings::as_ref(app)
            .remember_ssh_passwords
            .is_supported_on_current_platform()
}

const CONTENT_FONT_SIZE: f32 = 12.;
const ITEM_VERTICAL_SPACING: f32 = 24.;
/// There's a built-in 10px margin below the text input.
const BUILT_IN_TEXT_INPUT_MARGIN: f32 = 10.;
const SPACE_AFTER_TEXT_INPUT: f32 = ITEM_VERTICAL_SPACING - BUILT_IN_TEXT_INPUT_MARGIN;

const SSH_REUSE_CONTROL_MASTER_DESCRIPTION: &str = "Attach to a live SSH ControlMaster you already have configured for the destination host instead of creating an AI Terminal-owned one. Takes effect in new tabs.";

const SSH_EXTENSION_INSTALL_MODE_DESCRIPTION: &str = "Controls the installation behavior for AI Terminal's SSH extension when a remote host doesn't have it installed.";

const REMEMBER_SSH_PASSWORDS_DESCRIPTION: &str = "After you log in to an SSH host with a password, save it in your system's secure storage and enter it automatically on later logins to the same user, host, and port. A saved password that's rejected is removed, and the next one you type is saved.";

/// This page lets users configure when they get asked to ai_terminalify a session. Some shell commands
/// are recognized by default. Users can add new shell commands, or prevent the default ones from
/// asking. Users can also enable the SSH wrapper, and add hosts to a denylist.
/// This page is essentially the View for the SubshellSettings model, as well as the SshSettings
/// related to ai_terminalification.
pub struct AiTerminalifyPageView {
    page: PageType<Self>,
    /// This needs to mirror the length of SubshellSettings::added_remove_button_states.
    remove_added_command_button_states: Vec<MouseStateHandle>,
    add_added_commands_editor: ViewHandle<SubmittableTextInput>,
    /// This needs to mirror the length of SubshellSettings::denylisted_remove_button_states.
    remove_denylisted_command_button_states: Vec<MouseStateHandle>,
    add_denylisted_commands_editor: ViewHandle<SubmittableTextInput>,

    ssh_extension_install_mode_dropdown: ViewHandle<Dropdown<AiTerminalifyPageAction>>,
}

impl AiTerminalifyPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let ai_terminalify_settings_handle = AiTerminalifySettings::handle(ctx);

        ctx.observe(&ai_terminalify_settings_handle, Self::update_button_states);
        ctx.subscribe_to_model(
            &ai_terminalify_settings_handle,
            move |me, model, event, ctx| {
                me.update_button_states(model, ctx);
                if matches!(
                    event,
                    AiTerminalifySettingsChangedEvent::SshExtensionInstallModeSetting { .. }
                ) {
                    me.update_dropdown(ctx);
                }
                ctx.notify();
            },
        );
        if is_ssh_password_autofill_supported(ctx) {
            ctx.subscribe_to_model(&SavedSshPasswords::handle(ctx), |_, _, _, ctx| ctx.notify());
        }

        // Added commands can be specified by regex, while denied commands are strictly exact
        // match.
        let add_added_commands_editor = ctx.add_typed_action_view(|ctx| {
            let mut input =
                SubmittableTextInput::new(ctx).validate_on_edit(|regex| Regex::new(regex).is_ok());
            input.set_placeholder_text("command (supports regex)", ctx);
            input
        });

        ctx.subscribe_to_view(
            &add_added_commands_editor,
            Self::handle_added_command_editor_event,
        );

        let add_denylisted_commands_editor = ctx.add_typed_action_view(|ctx| {
            let mut input = SubmittableTextInput::new(ctx);
            input.set_placeholder_text("command (supports regex)", ctx);
            input
        });

        ctx.subscribe_to_view(
            &add_denylisted_commands_editor,
            Self::handle_denylisted_command_editor_event,
        );

        let ssh_extension_install_mode_dropdown =
            Self::create_ssh_extension_install_mode_dropdown(ctx);

        let mut instance = Self {
            page: Self::build_page(ctx),
            remove_added_command_button_states: Default::default(),
            add_added_commands_editor,
            remove_denylisted_command_button_states: Default::default(),
            add_denylisted_commands_editor,
            ssh_extension_install_mode_dropdown,
        };

        instance.update_button_states(ai_terminalify_settings_handle, ctx);
        instance
    }

    fn build_page(ctx: &mut ViewContext<Self>) -> PageType<Self> {
        let mut categories = vec![
            Category::new("", vec![Box::new(TitleWidget::default())]),
            Category::with_header(
                CategoryHeader::new("Subshells")
                    .with_subtitle("Subshells supported: bash, zsh, and fish."),
                vec![Box::new(SubshellsWidget::default())],
            ),
        ];

        let ai_terminalify_settings = AiTerminalifySettings::as_ref(ctx);
        if ai_terminalify_settings
            .enable_ssh_ai_terminalification
            .is_supported_on_current_platform()
        {
            categories.push(Category::with_header(
                CategoryHeader::new("SSH")
                    .with_subtitle("AI Terminal-ify your interactive SSH sessions."),
                vec![Box::new(SSHWidget::default())],
            ));
        }
        if is_ssh_password_autofill_supported(ctx) {
            categories.push(Category::new(
                "SSH passwords",
                vec![
                    Box::new(RememberSshPasswordsWidget::default()),
                    Box::new(SavedSshPasswordsWidget::default()),
                ],
            ));
        }
        PageType::new_categorized(categories, None)
    }

    /// This method ensures each command in the SubshellSettings has a matching button state for
    /// its delete button in the View.
    fn update_button_states(
        &mut self,
        ai_terminalify_settings_handle: ModelHandle<AiTerminalifySettings>,
        ctx: &mut ViewContext<Self>,
    ) {
        let ai_terminalify_settings = ai_terminalify_settings_handle.as_ref(ctx);
        self.remove_denylisted_command_button_states = ai_terminalify_settings
            .subshell_command_denylist
            .iter()
            .map(|_| Default::default())
            .collect();
        self.remove_added_command_button_states = ai_terminalify_settings
            .added_subshell_commands
            .iter()
            .map(|_| Default::default())
            .collect();
        ctx.notify();
    }

    /// Syncs the install-mode dropdown selection with the current
    /// `AiTerminalifySettings::ssh_extension_install_mode` value (e.g. after it
    /// was changed from the SSH remote server choice view).
    fn update_dropdown(&mut self, ctx: &mut ViewContext<Self>) {
        let current_mode = *AiTerminalifySettings::as_ref(ctx)
            .ssh_extension_install_mode
            .value();
        self.ssh_extension_install_mode_dropdown
            .update(ctx, |dropdown, ctx| {
                dropdown.set_selected_by_action(
                    AiTerminalifyPageAction::SetSshExtensionInstallMode(current_mode),
                    ctx,
                );
            });
    }

    fn handle_added_command_editor_event(
        &mut self,
        _handle: ViewHandle<SubmittableTextInput>,
        event: &SubmittableTextInputEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            SubmittableTextInputEvent::Submit(new_command) => {
                AiTerminalifySettings::handle(ctx).update(ctx, |ai_terminalify_settings, ctx| {
                    ai_terminalify_settings.add_subshell_command(new_command, ctx);
                });

                send_telemetry_from_ctx!(TelemetryEvent::AddAddedSubshellCommand, ctx);
            }
            SubmittableTextInputEvent::Escape => ctx.emit(SettingsPageEvent::FocusModal),
        }
    }

    fn handle_denylisted_command_editor_event(
        &mut self,
        _handle: ViewHandle<SubmittableTextInput>,
        event: &SubmittableTextInputEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            SubmittableTextInputEvent::Submit(new_command) => {
                AiTerminalifySettings::handle(ctx).update(ctx, |ai_terminalify_settings, ctx| {
                    ai_terminalify_settings.denylist_subshell_command(new_command, ctx);
                });

                send_telemetry_from_ctx!(TelemetryEvent::AddDenylistedSubshellCommand, ctx);
            }
            SubmittableTextInputEvent::Escape => ctx.emit(SettingsPageEvent::FocusModal),
        }
    }

    fn remove_denylisted_command(&self, index: usize, ctx: &mut ViewContext<Self>) {
        send_telemetry_from_ctx!(TelemetryEvent::RemoveDenylistedSubshellCommand, ctx);
        AiTerminalifySettings::handle(ctx).update(ctx, |ai_terminalify, ctx| {
            ai_terminalify.remove_denylisted_subshell_command(index, ctx)
        });
    }

    fn remove_added_command(&self, index: usize, ctx: &mut ViewContext<Self>) {
        send_telemetry_from_ctx!(TelemetryEvent::RemoveAddedSubshellCommand, ctx);
        AiTerminalifySettings::handle(ctx).update(ctx, |ai_terminalify, ctx| {
            ai_terminalify.remove_added_subshell_command(index, ctx)
        });
    }
}

impl Entity for AiTerminalifyPageView {
    type Event = SettingsPageEvent;
}

fn build_sub_sub_title(title: &str, appearance: &Appearance) -> Container {
    appearance
        .ui_builder()
        .span(title.to_string())
        .with_style(UiComponentStyles {
            font_size: Some(CONTENT_FONT_SIZE),
            ..Default::default()
        })
        .build()
}

const SSH_EXTENSION_DROPDOWN_WIDTH: f32 = 250.;

impl AiTerminalifyPageView {
    fn create_ssh_extension_install_mode_dropdown(
        ctx: &mut ViewContext<Self>,
    ) -> ViewHandle<Dropdown<AiTerminalifyPageAction>> {
        let items: Vec<DropdownItem<AiTerminalifyPageAction>> = SshExtensionInstallMode::iter()
            .map(|mode| {
                DropdownItem::new(
                    mode.display_name(),
                    AiTerminalifyPageAction::SetSshExtensionInstallMode(mode),
                )
            })
            .collect();

        let current_mode = *AiTerminalifySettings::as_ref(ctx)
            .ssh_extension_install_mode
            .value();
        let enable_ssh_ai_terminalification = *AiTerminalifySettings::as_ref(ctx)
            .enable_ssh_ai_terminalification
            .value();

        ctx.add_typed_action_view(move |ctx| {
            let mut dropdown = Dropdown::new(ctx);
            dropdown.set_top_bar_max_width(SSH_EXTENSION_DROPDOWN_WIDTH);
            dropdown.set_menu_width(SSH_EXTENSION_DROPDOWN_WIDTH, ctx);
            dropdown.add_items(items, ctx);
            dropdown.set_selected_by_action(
                AiTerminalifyPageAction::SetSshExtensionInstallMode(current_mode),
                ctx,
            );
            if !enable_ssh_ai_terminalification {
                dropdown.set_disabled(ctx);
            }
            dropdown
        })
    }

    /// Renders a title, a list of items that can be removed, and an input field to add new items.
    fn build_input_list<
        ListItem: Display,
        SettingsPageAction: Action + Clone,
        F: Fn(usize) -> SettingsPageAction,
        T: View,
    >(
        &self,
        title: &str,
        patterns: &[ListItem],
        mouse_states: &[MouseStateHandle],
        create_action: F,
        handle: &ViewHandle<T>,
        appearance: &Appearance,
    ) -> Container {
        let mut column = Flex::column();
        let mut title = build_sub_sub_title(title, appearance);

        if !patterns.is_empty() {
            title = title.with_padding_bottom(BUILT_IN_TEXT_INPUT_MARGIN);
        }

        column.add_child(title.finish());

        render_alternating_color_list(
            &mut column,
            patterns,
            mouse_states,
            create_action,
            appearance,
        );

        Container::new(
            column
                .with_child(
                    Container::new(ChildView::new(handle).finish())
                        .with_margin_bottom(SPACE_AFTER_TEXT_INPUT)
                        .finish(),
                )
                .finish(),
        )
    }
}

impl View for AiTerminalifyPageView {
    fn ui_name() -> &'static str {
        "AiTerminalifyPageView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AiTerminalifyPageAction {
    RemoveAddedCommand(usize),
    RemoveDenylistedCommand(usize),
    ToggleSshAiTerminalification,
    /// Toggles whether the legacy SSH wrapper attaches to an existing
    /// ControlMaster for the destination host instead of creating its own.
    ToggleReuseSshControlMaster,
    /// Set the SSH extension installation mode (always ask / always install / always skip).
    SetSshExtensionInstallMode(SshExtensionInstallMode),
    ToggleRememberSshPasswords,
    ForgetSshPassword(SshCredentialKey),
    OpenUrl(String),
}

impl TypedActionView for AiTerminalifyPageView {
    type Action = AiTerminalifyPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        use AiTerminalifyPageAction::*;
        match action {
            RemoveDenylistedCommand(index) => self.remove_denylisted_command(*index, ctx),
            RemoveAddedCommand(index) => self.remove_added_command(*index, ctx),
            ToggleSshAiTerminalification => {
                AiTerminalifySettings::handle(ctx).update(ctx, |ssh_settings, ctx| {
                    report_if_error!(
                        ssh_settings
                            .enable_ssh_ai_terminalification
                            .toggle_and_save_value(ctx)
                    );
                    send_telemetry_from_ctx!(
                        TelemetryEvent::ToggleSshAiTerminalification {
                            enabled: *ssh_settings.enable_ssh_ai_terminalification.value(),
                        },
                        ctx
                    );
                });
                let enabled = *AiTerminalifySettings::as_ref(ctx)
                    .enable_ssh_ai_terminalification
                    .value();
                self.ssh_extension_install_mode_dropdown
                    .update(ctx, |dropdown, ctx| {
                        if enabled {
                            dropdown.set_enabled(ctx);
                        } else {
                            dropdown.set_disabled(ctx);
                        }
                    });
            }
            ToggleReuseSshControlMaster => {
                SshSettings::handle(ctx).update(ctx, |ssh_settings, ctx| {
                    report_if_error!(
                        ssh_settings
                            .reuse_existing_control_master
                            .toggle_and_save_value(ctx)
                    );
                    send_telemetry_from_ctx!(
                        TelemetryEvent::FeaturesPageAction {
                            action: "ToggleSshReuseControlMaster".to_string(),
                            value: ssh_settings
                                .reuse_existing_control_master
                                .value()
                                .to_string(),
                        },
                        ctx
                    );
                });
            }
            SetSshExtensionInstallMode(mode) => {
                AiTerminalifySettings::handle(ctx).update(ctx, |ai_terminalify_settings, ctx| {
                    report_if_error!(
                        ai_terminalify_settings
                            .ssh_extension_install_mode
                            .set_value(*mode, ctx)
                    );
                    send_telemetry_from_ctx!(
                        TelemetryEvent::SetSshExtensionInstallMode {
                            mode: mode.display_name(),
                        },
                        ctx
                    );
                });
            }
            ToggleRememberSshPasswords => {
                SshSettings::handle(ctx).update(ctx, |ssh_settings, ctx| {
                    report_if_error!(
                        ssh_settings
                            .remember_ssh_passwords
                            .toggle_and_save_value(ctx)
                    );
                });
                ctx.notify();
            }
            ForgetSshPassword(account) => {
                SavedSshPasswords::handle(ctx).update(ctx, |saved_passwords, ctx| {
                    saved_passwords.forget(account, ctx);
                });
            }
            OpenUrl(url) => {
                ctx.open_url(url.as_str());
            }
        }
    }
}

impl SettingsPageMeta for AiTerminalifyPageView {
    fn section() -> SettingsSection {
        SettingsSection::AiTerminalify
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<AiTerminalifyPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<AiTerminalifyPageView>) -> Self {
        SettingsPageViewHandle::AiTerminalify(view_handle)
    }
}

#[derive(Default)]
struct TitleWidget {
    learn_more_highlight_index: HighlightedHyperlink,
}

impl TitleWidget {
    fn render_top_of_page(&self, appearance: &Appearance, _app: &AppContext) -> Box<dyn Element> {
        let ai_terminalify_description = vec![
            FormattedTextFragment::plain_text(
                "Configure whether AI Terminal attempts to “AI Terminal-ify” (add support for blocks, \
                    input modes, etc) certain shells. ",
            ),
            FormattedTextFragment::hyperlink(
                "Learn more",
                "https://docs.ai-terminal.invalid/terminal/ai_terminalify/subshells",
            ),
        ];

        let ai_terminalify_description = FormattedTextElement::new(
            FormattedText::new([FormattedTextLine::Line(ai_terminalify_description)]),
            CONTENT_FONT_SIZE,
            appearance.ui_font_family(),
            appearance.ui_font_family(),
            blended_colors::text_sub(appearance.theme(), appearance.theme().surface_1()),
            self.learn_more_highlight_index.clone(),
        )
        .with_hyperlink_font_color(appearance.theme().accent().into_solid())
        .register_default_click_handlers(|url, _, ctx| {
            ctx.open_url(&url.url);
        })
        .finish();

        Flex::column()
            .with_child(render_page_title(
                "AI Terminal-ify",
                HEADER_FONT_SIZE,
                appearance,
            ))
            .with_child(ai_terminalify_description)
            .finish()
    }
}

impl SettingsWidget for TitleWidget {
    type View = AiTerminalifyPageView;

    fn search_terms(&self) -> &str {
        "ssh subshell ai_terminalify session"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        Container::new(self.render_top_of_page(appearance, app))
            .with_margin_bottom(ITEM_VERTICAL_SPACING)
            .finish()
    }
}

#[derive(Default)]
struct SubshellsWidget {}

impl SubshellsWidget {
    fn render_subshells_section(
        &self,
        view: &AiTerminalifyPageView,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let mut column = Flex::column();

        let ai_terminalify_settings = AiTerminalifySettings::as_ref(app);

        column.add_child(
            view.build_input_list(
                "Added commands",
                &ai_terminalify_settings.added_subshell_commands,
                &view.remove_added_command_button_states,
                AiTerminalifyPageAction::RemoveAddedCommand,
                &view.add_added_commands_editor,
                appearance,
            )
            .finish(),
        );

        column.add_child(
            view.build_input_list(
                "Denylisted commands",
                &ai_terminalify_settings.subshell_command_denylist,
                &view.remove_denylisted_command_button_states,
                AiTerminalifyPageAction::RemoveDenylistedCommand,
                &view.add_denylisted_commands_editor,
                appearance,
            )
            .with_margin_bottom(-BUILT_IN_TEXT_INPUT_MARGIN)
            .finish(),
        );

        column.finish()
    }
}

impl SettingsWidget for SubshellsWidget {
    type View = AiTerminalifyPageView;

    fn search_terms(&self) -> &str {
        "ai_terminalify subshell"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        Container::new(self.render_subshells_section(view, appearance, app))
            .with_margin_bottom(ITEM_VERTICAL_SPACING)
            .finish()
    }
}

#[derive(Default)]
struct SSHWidget {
    enable_ssh_ai_terminalification_switch_state: SwitchStateHandle,
    reuse_control_master_switch_state: SwitchStateHandle,
    local_only_icon_tooltip_states: RefCell<HashMap<String, MouseStateHandle>>,
}

impl SettingsWidget for SSHWidget {
    type View = AiTerminalifyPageView;

    fn search_terms(&self) -> &str {
        "ai_terminalify ssh"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let mut column = Flex::column();
        let ui_builder = appearance.ui_builder();
        let description_text_color = appearance
            .theme()
            .sub_text_color(appearance.theme().surface_2());

        let enable_ssh_ai_terminalification = *AiTerminalifySettings::as_ref(app)
            .enable_ssh_ai_terminalification
            .value();

        add_setting(
            &mut column,
            &AiTerminalifySettings::as_ref(app).enable_ssh_ai_terminalification,
            move || {
                render_body_item::<AiTerminalifyPageAction>(
                    "AI Terminal-ify SSH Sessions".into(),
                    None,
                    LocalOnlyIconState::for_setting(
                        EnableSshAiTerminalification::storage_key(),
                        EnableSshAiTerminalification::sync_to_cloud(),
                        &mut self.local_only_icon_tooltip_states.borrow_mut(),
                        app,
                    ),
                    ToggleState::Enabled,
                    appearance,
                    ui_builder
                        .switch(self.enable_ssh_ai_terminalification_switch_state.clone())
                        .check(enable_ssh_ai_terminalification)
                        .build()
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(
                                AiTerminalifyPageAction::ToggleSshAiTerminalification,
                            );
                        })
                        .finish(),
                    None,
                )
            },
        );

        if FeatureFlag::SshRemoteServer.is_enabled() {
            let label_color_override = if !enable_ssh_ai_terminalification {
                Some(appearance.theme().disabled_ui_text_color())
            } else {
                None
            };
            add_setting(
                &mut column,
                &AiTerminalifySettings::as_ref(app).ssh_extension_install_mode,
                move || {
                    Container::new(render_dropdown_item(
                        appearance,
                        "Install SSH extension",
                        Some(SSH_EXTENSION_INSTALL_MODE_DESCRIPTION),
                        None,
                        LocalOnlyIconState::for_setting(
                            SshExtensionInstallModeSetting::storage_key(),
                            SshExtensionInstallModeSetting::sync_to_cloud(),
                            &mut self.local_only_icon_tooltip_states.borrow_mut(),
                            app,
                        ),
                        label_color_override,
                        &view.ssh_extension_install_mode_dropdown,
                    ))
                    .with_padding_bottom(HEADER_PADDING)
                    .finish()
                },
            );
        }

        let reuse_existing_control_master = *SshSettings::as_ref(app)
            .reuse_existing_control_master
            .value();
        add_setting(
            &mut column,
            &SshSettings::as_ref(app).reuse_existing_control_master,
            move || {
                let mut column = Flex::column();
                column.add_child(render_body_item::<AiTerminalifyPageAction>(
                    "Reuse existing SSH ControlMaster".into(),
                    None,
                    LocalOnlyIconState::for_setting(
                        ReuseExistingSshControlMaster::storage_key(),
                        ReuseExistingSshControlMaster::sync_to_cloud(),
                        &mut self.local_only_icon_tooltip_states.borrow_mut(),
                        app,
                    ),
                    enable_ssh_ai_terminalification.into(),
                    appearance,
                    ui_builder
                        .switch(self.reuse_control_master_switch_state.clone())
                        .check(reuse_existing_control_master)
                        .with_disabled(!enable_ssh_ai_terminalification)
                        .build()
                        .on_click(move |ctx, _, _| {
                            if !enable_ssh_ai_terminalification {
                                return;
                            }
                            ctx.dispatch_typed_action(
                                AiTerminalifyPageAction::ToggleReuseSshControlMaster,
                            );
                        })
                        .finish(),
                    None,
                ));
                column.add_child(
                    ui_builder
                        .paragraph(SSH_REUSE_CONTROL_MASTER_DESCRIPTION.to_owned())
                        .with_style(UiComponentStyles {
                            font_color: Some(description_text_color.into_solid()),
                            margin: Some(
                                Coords::default()
                                    .top(styles::DESCRIPTION_NEGATIVE_MARGIN_OFFSET)
                                    .bottom(styles::DESCRIPTION_LINE_MARGIN_BOTTOM),
                            ),
                            ..Default::default()
                        })
                        .build()
                        .finish(),
                );
                column.finish()
            },
        );

        column.finish()
    }
}

#[derive(Default)]
struct RememberSshPasswordsWidget {
    switch_state: SwitchStateHandle,
    local_only_icon_tooltip_states: RefCell<HashMap<String, MouseStateHandle>>,
}

impl SettingsWidget for RememberSshPasswordsWidget {
    type View = AiTerminalifyPageView;

    fn search_terms(&self) -> &str {
        "ssh remember save passwords autofill login"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let remember_ssh_passwords = *SshSettings::as_ref(app).remember_ssh_passwords.value();
        render_body_item::<AiTerminalifyPageAction>(
            "Remember SSH passwords".into(),
            None,
            LocalOnlyIconState::for_setting(
                RememberSshPasswords::storage_key(),
                RememberSshPasswords::sync_to_cloud(),
                &mut self.local_only_icon_tooltip_states.borrow_mut(),
                app,
            ),
            ToggleState::Enabled,
            appearance,
            appearance
                .ui_builder()
                .switch(self.switch_state.clone())
                .check(remember_ssh_passwords)
                .build()
                .on_click(|ctx, _, _| {
                    ctx.dispatch_typed_action(AiTerminalifyPageAction::ToggleRememberSshPasswords);
                })
                .finish(),
            Some(REMEMBER_SSH_PASSWORDS_DESCRIPTION.to_owned()),
        )
    }
}

#[derive(Default)]
struct SavedSshPasswordsWidget {
    forget_button_states: RefCell<HashMap<SshCredentialKey, MouseStateHandle>>,
}

impl SettingsWidget for SavedSshPasswordsWidget {
    type View = AiTerminalifyPageView;

    fn search_terms(&self) -> &str {
        "ssh saved passwords forget remove delete login host"
    }

    fn render(
        &self,
        _view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let accounts: Vec<_> = SavedSshPasswords::as_ref(app)
            .accounts(app)
            .cloned()
            .collect();
        let mut forget_button_states = self.forget_button_states.borrow_mut();
        forget_button_states.retain(|account, _| accounts.contains(account));
        let mouse_states: Vec<_> = accounts
            .iter()
            .map(|account| {
                forget_button_states
                    .entry(account.clone())
                    .or_default()
                    .clone()
            })
            .collect();

        let mut column = Flex::column().with_child(
            build_sub_sub_title("Saved SSH passwords", appearance)
                .with_padding_bottom(BUILT_IN_TEXT_INPUT_MARGIN)
                .finish(),
        );
        if accounts.is_empty() {
            column.add_child(
                appearance
                    .ui_builder()
                    .paragraph("No saved passwords.".to_owned())
                    .with_style(UiComponentStyles {
                        font_color: Some(blended_colors::text_sub(
                            appearance.theme(),
                            appearance.theme().surface_1(),
                        )),
                        font_size: Some(CONTENT_FONT_SIZE),
                        ..Default::default()
                    })
                    .build()
                    .finish(),
            );
        }
        render_alternating_color_list(
            &mut column,
            &accounts,
            &mouse_states,
            |index| AiTerminalifyPageAction::ForgetSshPassword(accounts[index].clone()),
            appearance,
        );

        Container::new(column.finish())
            .with_margin_bottom(ITEM_VERTICAL_SPACING)
            .finish()
    }
}

mod styles {
    // Apply a negative margin to the description text so it appears closer to the main
    // settings option text.
    pub const DESCRIPTION_NEGATIVE_MARGIN_OFFSET: f32 = -8.;

    /// The space after a description.
    pub const DESCRIPTION_LINE_MARGIN_BOTTOM: f32 = 18.;
}

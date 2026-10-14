use std::sync::Arc;

use ai_terminal_ui::elements::{
    ChildView, Container, CrossAxisAlignment, Expanded, Flex, MainAxisSize, ParentElement,
};
use ai_terminal_ui::prelude::Empty;
use ai_terminal_ui::{AppContext, Element, Entity, TypedActionView, View, ViewContext, ViewHandle};
use parking_lot::FairMutex;

use super::{AgentFooterButtonTheme, USE_AGENT_KEYSTROKE};
use crate::terminal::view::{PADDING_LEFT, TerminalModel};
use crate::ui_components::icons::Icon;
use crate::view_components::action_button::{
    ActionButton, ButtonSize, KeystrokeSource, TooltipAlignment,
};

/// Footer view rendered for detected subshell commands, offering both
/// "AI Terminal-ify" and "Use agent" buttons in a horizontal row.
pub(super) struct AiTerminalifyFooterView {
    terminal_model: Arc<FairMutex<TerminalModel>>,
    ai_terminalify_button: ViewHandle<ActionButton>,
    use_agent_button: ViewHandle<ActionButton>,
    dismiss_button: ViewHandle<ActionButton>,
    /// Whether the footer is currently offering subshell ai_terminalification.
    is_active: bool,
}

impl AiTerminalifyFooterView {
    pub fn new(terminal_model: Arc<FairMutex<TerminalModel>>, ctx: &mut ViewContext<Self>) -> Self {
        let button_size = ButtonSize::XSmall;

        let ai_terminalify_button = ctx.add_typed_action_view(|_ctx| {
            ActionButton::new(
                "AI Terminal-ify subshell",
                AgentFooterButtonTheme::new(None),
            )
            .with_icon(Icon::AiTerminal)
            .with_size(button_size)
            .with_tooltip("Enable AI Terminal shell integration in this session")
            .with_tooltip_alignment(TooltipAlignment::Left)
            .on_click(|ctx| {
                ctx.dispatch_typed_action(AiTerminalifyFooterViewAction::AiTerminalify);
            })
        });

        let use_agent_button = ctx.add_typed_action_view(|ctx| {
            ActionButton::new("Use agent", AgentFooterButtonTheme::new(None))
                .with_icon(Icon::Agent)
                .with_keybinding(KeystrokeSource::Fixed(USE_AGENT_KEYSTROKE.clone()), ctx)
                .with_size(button_size)
                .with_tooltip("Ask the AI Terminal agent to assist")
                .with_tooltip_alignment(TooltipAlignment::Left)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(AiTerminalifyFooterViewAction::UseAgent);
                })
        });

        let dismiss_button = ctx.add_typed_action_view(|_ctx| {
            ActionButton::new("Dismiss", AgentFooterButtonTheme::new(None))
                .with_size(button_size)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(AiTerminalifyFooterViewAction::Dismiss);
                })
        });

        Self {
            terminal_model,
            ai_terminalify_button,
            use_agent_button,
            dismiss_button,
            is_active: false,
        }
    }

    /// Activates the footer so it offers subshell ai_terminalification.
    pub fn show(&mut self, ctx: &mut ViewContext<Self>) {
        self.ai_terminalify_button.update(ctx, |button, ctx| {
            button.set_keybinding(
                Some(KeystrokeSource::Binding("terminal:ai_terminalify_subshell")),
                ctx,
            );
        });
        self.is_active = true;
        ctx.notify();
    }

    /// Returns whether the footer is currently offering subshell ai_terminalification.
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Deactivates the footer.
    pub fn clear(&mut self, ctx: &mut ViewContext<Self>) {
        self.is_active = false;
        self.ai_terminalify_button.update(ctx, |button, ctx| {
            button.set_keybinding(None, ctx);
        });
        ctx.notify();
    }
}

#[derive(Debug, Clone)]
pub enum AiTerminalifyFooterViewAction {
    AiTerminalify,
    UseAgent,
    Dismiss,
}

pub enum AiTerminalifyFooterViewEvent {
    AiTerminalify,
    UseAgent,
    Dismiss,
}

impl Entity for AiTerminalifyFooterView {
    type Event = AiTerminalifyFooterViewEvent;
}

impl View for AiTerminalifyFooterView {
    fn ui_name() -> &'static str {
        "AiTerminalifyFooterView"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        let terminal_model = self.terminal_model.lock();

        let button_row = Flex::row()
            .with_spacing(4.)
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(ChildView::new(&self.ai_terminalify_button).finish())
            .with_child(ChildView::new(&self.use_agent_button).finish())
            .with_child(Expanded::new(1., Empty::new().finish()).finish())
            .with_child(ChildView::new(&self.dismiss_button).finish());

        let mut container = Container::new(button_row.finish())
            .with_horizontal_padding(*PADDING_LEFT)
            .with_vertical_padding(4.);

        if terminal_model.is_alt_screen_active()
            && let Some(bg_color) = terminal_model.alt_screen().inferred_bg_color()
        {
            container = container.with_background(bg_color);
        }

        container.finish()
    }
}

impl TypedActionView for AiTerminalifyFooterView {
    type Action = AiTerminalifyFooterViewAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            AiTerminalifyFooterViewAction::AiTerminalify => {
                if self.is_active {
                    self.clear(ctx);
                    ctx.emit(AiTerminalifyFooterViewEvent::AiTerminalify);
                }
            }
            AiTerminalifyFooterViewAction::UseAgent => {
                self.clear(ctx);
                ctx.emit(AiTerminalifyFooterViewEvent::UseAgent);
            }
            AiTerminalifyFooterViewAction::Dismiss => {
                self.clear(ctx);
                ctx.emit(AiTerminalifyFooterViewEvent::Dismiss);
            }
        }
    }
}

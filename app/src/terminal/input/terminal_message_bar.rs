use std::sync::Arc;

use ai_terminal_core::ui::Icon;
use ai_terminal_core::ui::theme::AiTerminalTheme;
use ai_terminal_ui::elements::{Container, Element, MouseStateHandle};
use ai_terminal_ui::keymap::Keystroke;
use ai_terminal_ui::{
    AppContext, Entity, ModelHandle, SingletonEntity, TypedActionView, View, ViewContext,
};
use parking_lot::FairMutex;
use pathfinder_color::ColorU;

use super::buffer_model::InputBufferModel;
use super::local_classification::{
    LocalInputClassification, LocalInputClassificationModel, PLACEHOLDER_HINT_TEXT,
};
use super::message_bar::common::render_terminal_message;
use super::message_bar::{Message, MessageItem, MessageProvider, truncated_command_for_block};
use crate::ai::blocklist::{
    BlocklistAIContextEvent, BlocklistAIContextModel, BlocklistAIInputModel,
};
use crate::ai::pricing_promotion::{
    PricingPromotionState, PricingPromotionStateEvent, PricingPromotionSurface,
};
use crate::appearance::Appearance;
use crate::search::slash_command_menu::static_commands::commands;
use crate::settings::{AISettings, InputSettings};
use crate::terminal::input::SET_INPUT_MODE_TERMINAL_ACTION_NAME;
use crate::terminal::input::inline_history::{AcceptHistoryItem, HistoryTab};
use crate::terminal::input::inline_menu::{InlineMenuModel, InlineMenuModelEvent};
use crate::terminal::input::message_bar::MessageTransformer;
use crate::terminal::input::suggestions_mode_model::{
    InputSuggestionsModeEvent, InputSuggestionsModeModel,
};
use crate::terminal::model::TerminalModel;
use crate::terminal::view::init::SELECT_PREVIOUS_BLOCK_ACTION_NAME;
use crate::util::bindings::keybinding_name_to_keystroke;

/// Renders contextual hint text at the bottom of the terminal input when `FeatureFlag::AgentView`
/// is enabled.
pub struct TerminalInputMessageBar {
    terminal_model: Arc<FairMutex<TerminalModel>>,
    ai_input_model: ModelHandle<BlocklistAIInputModel>,
    input_buffer_model: ModelHandle<InputBufferModel>,
    context_model: ModelHandle<BlocklistAIContextModel>,
    suggestions_mode_model: ModelHandle<InputSuggestionsModeModel>,
    inline_history_model: ModelHandle<InlineMenuModel<AcceptHistoryItem, HistoryTab>>,
    local_classification_model: ModelHandle<LocalInputClassificationModel>,
    promotion_close_mouse_state: MouseStateHandle,
}

impl Entity for TerminalInputMessageBar {
    type Event = ();
}
#[derive(Clone, Debug)]
pub enum TerminalInputMessageBarAction {
    DismissPricingPromotion,
}

impl TerminalInputMessageBar {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        terminal_model: Arc<FairMutex<TerminalModel>>,
        ai_input_model: ModelHandle<BlocklistAIInputModel>,
        input_buffer_model: ModelHandle<InputBufferModel>,
        context_model: ModelHandle<BlocklistAIContextModel>,
        suggestions_mode_model: ModelHandle<InputSuggestionsModeModel>,
        inline_history_model: ModelHandle<InlineMenuModel<AcceptHistoryItem, HistoryTab>>,
        local_classification_model: ModelHandle<LocalInputClassificationModel>,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        ctx.subscribe_to_model(&ai_input_model, |_, _, _, ctx| {
            ctx.notify();
        });
        ctx.subscribe_to_model(&local_classification_model, |_, _, _, ctx| {
            ctx.notify();
        });
        ctx.subscribe_to_model(&input_buffer_model, |_, _, _, ctx| {
            ctx.notify();
        });
        ctx.subscribe_to_model(&context_model, |_, _, event, ctx| {
            if let BlocklistAIContextEvent::UpdatedPendingContext { .. } = event {
                ctx.notify();
            }
        });
        ctx.subscribe_to_model(&suggestions_mode_model, |_, _, event, ctx| {
            let InputSuggestionsModeEvent::ModeChanged { .. } = event;
            ctx.notify();
        });
        ctx.subscribe_to_model(&inline_history_model, |_, _, event, ctx| {
            if let InlineMenuModelEvent::UpdatedSelectedItem = event {
                ctx.notify();
            }
        });
        ctx.subscribe_to_model(&PricingPromotionState::handle(ctx), |_, _, event, ctx| {
            if matches!(event, PricingPromotionStateEvent::Updated) {
                ctx.notify();
            }
        });
        Self {
            terminal_model,
            ai_input_model,
            input_buffer_model,
            context_model,
            suggestions_mode_model,
            inline_history_model,
            local_classification_model,
            promotion_close_mouse_state: MouseStateHandle::default(),
        }
    }
}

impl View for TerminalInputMessageBar {
    fn ui_name() -> &'static str {
        "TerminalInputMessageBar"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        if self
            .suggestions_mode_model
            .as_ref(app)
            .is_inline_history_menu()
        {
            let selected = self.inline_history_model.as_ref(app).selected_item();
            let message = InlineHistoryMessageProducer
                .produce_message(selected)
                .unwrap_or_default();
            return Container::new(render_terminal_message(message, app))
                .with_padding_bottom(8.)
                .with_padding_right(8.)
                .finish();
        }

        let terminal_model = self.terminal_model.lock();
        let current_buffer = self.input_buffer_model.as_ref(app).current_value();
        let context_model = self.context_model.as_ref(app);
        let input_model = self.ai_input_model.as_ref(app);

        let local_classification = self
            .local_classification_model
            .as_ref(app)
            .latest_classification();

        let args = TerminalMessageArgs {
            current_input: current_buffer,
            terminal_model: &terminal_model,
            context_model,
            input_model,
            local_classification,
            app,
            promotion_close_mouse_state: &self.promotion_close_mouse_state,
        };

        // Every producer advertises an agent feature, so they only apply when AI is available.
        let mut message = if AISettings::as_ref(app).is_any_ai_enabled(app) {
            ErroredBlockMessageProducer
                .produce_message(args)
                .or_else(|| AgentMessageProducer.produce_message(args))
                .or_else(|| PlanMessageProducer.produce_message(args))
                .or_else(|| ContinueConversationMessageProducer.produce_message(args))
                .or_else(|| DefaultMessageProducer.produce_message(args))
                .unwrap_or_default()
        } else {
            Message::default()
        };

        let transformers: [Box<dyn MessageTransformer<TerminalMessageArgs<'_>>>; 4] = [
            Box::new(AttachedBlocksMessageTransformer),
            Box::new(AttachedTextSelectionMessageTransformer),
            Box::new(AutodetectedPromptMessageTransformer),
            Box::new(LocalInputClassificationMessageTransformer),
        ];

        for transformer in transformers {
            transformer.transform_message(&mut message, args);
        }

        Container::new(render_terminal_message(message, app))
            .with_padding_bottom(8.)
            .with_padding_right(8.)
            .finish()
    }
}

#[derive(Copy, Clone)]
pub struct TerminalMessageArgs<'a> {
    current_input: &'a str,
    terminal_model: &'a TerminalModel,
    context_model: &'a BlocklistAIContextModel,
    input_model: &'a BlocklistAIInputModel,
    local_classification: Option<&'a LocalInputClassification>,
    app: &'a AppContext,
    promotion_close_mouse_state: &'a MouseStateHandle,
}

impl<'a> TerminalMessageArgs<'a> {
    fn is_input_ai_detected(&self) -> bool {
        !self.current_input.is_empty()
            && self.input_model.is_ai_input_enabled()
            && !self.input_model.is_input_type_locked()
    }
}

struct ErroredBlockMessageProducer;
impl MessageProvider<TerminalMessageArgs<'_>> for ErroredBlockMessageProducer {
    fn produce_message(&self, args: TerminalMessageArgs<'_>) -> Option<Message> {
        let block = args.terminal_model.block_list().last_non_hidden_block()?;
        let context_block_ids = args.context_model.pending_context_block_ids();
        if block.exit_code().was_successful()
            || !args.current_input.is_empty()
            || !context_block_ids.is_empty()
        {
            return None;
        }
        let keystroke = keybinding_name_to_keystroke(SELECT_PREVIOUS_BLOCK_ACTION_NAME, args.app)?;
        Some(Message::new(vec![
            MessageItem::keystroke(keystroke),
            MessageItem::text(format!(
                " attach `{}` output as agent context",
                truncated_command_for_block(&block.command_to_string())
            )),
        ]))
    }
}

struct AgentMessageProducer;
impl MessageProvider<TerminalMessageArgs<'_>> for AgentMessageProducer {
    fn produce_message(&self, args: TerminalMessageArgs<'_>) -> Option<Message> {
        let TerminalMessageArgs {
            current_input, app, ..
        } = args;

        if !current_input.starts_with(commands::AGENT.name)
            && !current_input.starts_with(commands::NEW.name)
        {
            return None;
        }
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();

        Some(
            Message::new(vec![
                MessageItem::keystroke(Keystroke {
                    key: "enter".to_owned(),
                    ..Default::default()
                }),
                MessageItem::text(" new conversation"),
            ])
            .with_color(message_magenta(theme)),
        )
    }
}

struct PlanMessageProducer;
impl MessageProvider<TerminalMessageArgs<'_>> for PlanMessageProducer {
    fn produce_message(&self, args: TerminalMessageArgs<'_>) -> Option<Message> {
        let TerminalMessageArgs {
            current_input, app, ..
        } = args;

        if !current_input.trim_start().starts_with(commands::PLAN.name) {
            return None;
        }

        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let is_input_ai_detected = args.is_input_ai_detected();

        Some(
            Message::new(vec![
                MessageItem::keystroke(Keystroke {
                    cmd: !is_input_ai_detected && cfg!(target_os = "macos"),
                    ctrl: !is_input_ai_detected && !cfg!(target_os = "macos"),
                    shift: !is_input_ai_detected && !cfg!(target_os = "macos"),
                    key: "enter".to_owned(),
                    ..Default::default()
                }),
                MessageItem::text(" plan with agent"),
            ])
            .with_color(message_magenta(theme)),
        )
    }
}

struct ContinueConversationMessageProducer;
impl MessageProvider<TerminalMessageArgs<'_>> for ContinueConversationMessageProducer {
    fn produce_message(&self, args: TerminalMessageArgs<'_>) -> Option<Message> {
        let TerminalMessageArgs {
            current_input,
            terminal_model,
            ..
        } = args;
        if !current_input.is_empty() || !terminal_model.is_last_visible_item_agent_view_block() {
            return None;
        }

        let keystroke = keybinding_name_to_keystroke(commands::CONVERSATIONS.name, args.app)?;
        Some(Message::new(vec![
            MessageItem::keystroke(keystroke),
            MessageItem::text(" to continue conversation"),
        ]))
    }
}

mod internal {
    use crate::terminal::TerminalModel;
    use crate::terminal::model::blocks::{
        BlockHeight, BlockHeightItem, BlockHeightSummary, RichContentItem,
    };

    impl TerminalModel {
        pub(super) fn is_last_visible_item_agent_view_block(&self) -> bool {
            let block_list = self.block_list();

            // When we insert rich content (including agent view blocks) we insert it immediately before
            // the active block (unless explicitly inserting below a long-running block). The active
            // block is a special "ai-terminal input" block that often exists even when it isn't user-visible.
            //
            // So, for dedupe we check the first visible (non-zero height) item *immediately before the
            // active block*. This avoids false negatives caused by the active block itself.
            let active_block_index = block_list.active_block_index();

            let mut cursor = block_list
                .block_heights()
                .cursor::<BlockHeight, BlockHeightSummary>();
            cursor.descend_to_last_item(block_list.block_heights());

            // Seek backwards until we're at the active block's height item.
            while let Some(item) = cursor.item() {
                match item {
                    BlockHeightItem::Block(_)
                        if cursor.start().block_count == active_block_index.0 =>
                    {
                        break;
                    }
                    _ => cursor.prev(),
                }
            }

            // Now walk backwards to find the first visible item before the active block.
            cursor.prev();
            while let Some(item) = cursor.item() {
                let is_hidden = item.height() == BlockHeight::zero();
                match item {
                    BlockHeightItem::RichContent(RichContentItem { content_type, .. })
                        if !is_hidden
                            && content_type
                                .is_some_and(|content| content.is_agent_view_block()) =>
                    {
                        return true;
                    }
                    _ => {
                        if is_hidden {
                            cursor.prev();
                        } else {
                            return false;
                        }
                    }
                }
            }

            false
        }
    }
}

struct DefaultMessageProducer;
impl MessageProvider<TerminalMessageArgs<'_>> for DefaultMessageProducer {
    fn produce_message(&self, args: TerminalMessageArgs<'_>) -> Option<Message> {
        let is_input_ai_detected = args.is_input_ai_detected();

        let keystroke = if is_input_ai_detected {
            Some(Keystroke {
                key: "enter".to_owned(),
                ..Default::default()
            })
        } else if let Some(keystroke) = keybinding_name_to_keystroke(commands::AGENT.name, args.app)
        {
            Some(keystroke)
        } else {
            keybinding_name_to_keystroke(commands::NEW.name, args.app)
        };

        if let Some(keystroke) = keystroke {
            let promotion_message = PricingPromotionState::as_ref(args.app)
                .visible_message(PricingPromotionSurface::TerminalMessageBar, args.app);
            let mut text = " new /agent conversation".to_string();
            if let Some(promotion_message) = &promotion_message {
                text.push_str(" · ");
                text.push_str(promotion_message);
            }
            let mut items = vec![MessageItem::keystroke(keystroke), MessageItem::text(text)];
            if promotion_message.is_some() {
                items.push(MessageItem::text(" "));
                items.push(MessageItem::clickable(
                    vec![MessageItem::icon(Icon::X)],
                    |ctx| {
                        ctx.dispatch_typed_action(
                            TerminalInputMessageBarAction::DismissPricingPromotion,
                        );
                    },
                    args.promotion_close_mouse_state.clone(),
                ));
            }
            Some(Message::new(items))
        } else {
            Some(Message::new(vec![MessageItem::text(
                "/agent for new conversation",
            )]))
        }
    }
}

impl TypedActionView for TerminalInputMessageBar {
    type Action = TerminalInputMessageBarAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            TerminalInputMessageBarAction::DismissPricingPromotion => {
                PricingPromotionState::handle(ctx).update(ctx, |state, ctx| {
                    state.dismiss(PricingPromotionSurface::TerminalMessageBar, ctx);
                });
            }
        }
    }
}

struct InlineHistoryMessageProducer;
impl MessageProvider<Option<&AcceptHistoryItem>> for InlineHistoryMessageProducer {
    fn produce_message(&self, selected: Option<&AcceptHistoryItem>) -> Option<Message> {
        let enter = MessageItem::keystroke(Keystroke {
            key: "enter".to_owned(),
            ..Default::default()
        });
        let items = match selected {
            Some(AcceptHistoryItem::Command { .. }) => {
                vec![enter, MessageItem::text(" to execute")]
            }
            Some(AcceptHistoryItem::AIPrompt { .. }) => {
                vec![enter, MessageItem::text(" to send")]
            }
            Some(AcceptHistoryItem::Conversation { title, .. }) => {
                vec![enter, MessageItem::text(format!(" to open '{title}'"))]
            }
            None => {
                vec![MessageItem::text("")]
            }
        };
        Some(Message::new(items))
    }
}

struct AutodetectedPromptMessageTransformer;
impl MessageTransformer<TerminalMessageArgs<'_>> for AutodetectedPromptMessageTransformer {
    fn transform_message(&self, message: &mut Message, args: TerminalMessageArgs<'_>) -> bool {
        if !args.is_input_ai_detected()
            || args.current_input.starts_with(commands::AGENT.name)
            || args.current_input.starts_with(commands::NEW.name)
        {
            return false;
        }

        // Don't append this message if there is attached context, just cause its
        // too much text and overwhelming.
        if args.context_model.pending_context_block_ids().is_empty()
            && args.context_model.pending_context_selected_text().is_none()
        {
            let set_terminal_mode_keystroke =
                keybinding_name_to_keystroke(SET_INPUT_MODE_TERMINAL_ACTION_NAME, args.app)
                    .unwrap_or_else(|| Keystroke {
                        key: "escape".to_owned(),
                        ..Default::default()
                    });

            message.items.extend([
                MessageItem::text(" (autodetected) "),
                MessageItem::keystroke(set_terminal_mode_keystroke),
                MessageItem::text(" to override"),
            ]);
        }
        message.set_color(message_magenta(Appearance::as_ref(args.app).theme()));
        true
    }
}

struct LocalInputClassificationMessageTransformer;
impl MessageTransformer<TerminalMessageArgs<'_>> for LocalInputClassificationMessageTransformer {
    fn transform_message(&self, message: &mut Message, args: TerminalMessageArgs<'_>) -> bool {
        let Some(classification) = args.local_classification else {
            // Without AI the hint is often the only content, so the placeholder keeps the input
            // from shifting as the hint appears and disappears.
            if message.items.is_empty()
                && InputSettings::as_ref(args.app).is_local_input_classification_hint_enabled()
            {
                *message = Message::from_text(PLACEHOLDER_HINT_TEXT);
                return true;
            }
            return false;
        };

        let hint_text = classification.hint_text();
        if message.items.is_empty() {
            *message = Message::from_text(hint_text);
            if classification.is_natural_language() {
                message.set_color(message_magenta(Appearance::as_ref(args.app).theme()));
            }
        } else {
            message.append_text(&format!(" · {hint_text}"));
        }
        true
    }
}

struct AttachedBlocksMessageTransformer;
impl MessageTransformer<TerminalMessageArgs<'_>> for AttachedBlocksMessageTransformer {
    fn transform_message(&self, message: &mut Message, args: TerminalMessageArgs<'_>) -> bool {
        let context_block_ids = args.context_model.pending_context_block_ids();
        if context_block_ids.is_empty() {
            return false;
        }

        let Some(block_command) = context_block_ids
            .iter()
            .find_map(|id| args.terminal_model.block_list().block_with_id(id))
            .map(|block| truncated_command_for_block(&block.command_to_string()))
        else {
            return false;
        };

        if context_block_ids.len() == 1 {
            message.append_text(format!(" with `{}` attached", block_command).as_str());
        } else {
            let text = if context_block_ids.len() == 2 {
                format!(" with `{}` and 1 other command attached", block_command)
            } else {
                format!(
                    " with `{}` and {} other commands attached",
                    block_command,
                    context_block_ids.len().saturating_sub(1)
                )
            };
            message.append_text(text.as_str());
        }

        true
    }
}

struct AttachedTextSelectionMessageTransformer;
impl MessageTransformer<TerminalMessageArgs<'_>> for AttachedTextSelectionMessageTransformer {
    fn transform_message(&self, message: &mut Message, args: TerminalMessageArgs<'_>) -> bool {
        if args.context_model.pending_context_selected_text().is_none()
            || !args.context_model.pending_context_block_ids().is_empty()
        {
            return false;
        }
        message.append_text(" with text selection attached");
        true
    }
}

fn message_magenta(theme: &AiTerminalTheme) -> ColorU {
    let mut color = theme.ansi_fg_magenta();
    color.a = (255. * 0.65) as u8;
    color
}

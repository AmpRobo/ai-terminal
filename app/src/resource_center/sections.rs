use ai_terminal_core::context_flag::ContextFlag;
use ai_terminal_core::features::FeatureFlag;
use ai_terminal_ui::ViewContext;

use super::{
    ContentItem, ContentSectionData, FeatureItem, FeatureSection, FeatureSectionData,
    ResourceCenterMainView, Section, Tip, TipAction, TipHint,
};

pub fn sections(ctx: &mut ViewContext<ResourceCenterMainView>) -> Vec<Section> {
    let mut sections = vec![Section::Changelog()];

    if FeatureFlag::AvatarInTabBar.is_enabled() {
        return sections;
    }

    let get_started = FeatureSectionData {
        section_name: FeatureSection::GettingStarted,
        items: vec![
            FeatureItem::new(
                "Create your first block",
                "Run a command to see your command and output grouped.",
                Tip::Hint(TipHint::CreateBlock),
                ctx,
            ),
            FeatureItem::new(
                "Navigate blocks",
                "Click to select a block and navigate with arrow keys.",
                Tip::Hint(TipHint::BlockSelect),
                ctx,
            ),
            FeatureItem::new(
                "Take an action on block",
                "Right click on a block to copy/paste, share, more.",
                Tip::Hint(TipHint::BlockAction),
                ctx,
            ),
            FeatureItem::new(
                "Open command palette",
                "Access all of AI Terminal via the keyboard.",
                Tip::Action(TipAction::CommandPalette),
                ctx,
            ),
            FeatureItem::new(
                "Set your theme",
                "Make AI Terminal your own by choosing a theme.",
                Tip::Action(TipAction::ThemePicker),
                ctx,
            ),
        ],
    };
    sections.push(Section::Feature(get_started));

    let maximize_ai_terminal = FeatureSectionData {
        section_name: FeatureSection::MaximizeAiTerminal,
        items: maximize_ai_terminal_items(ctx),
    };
    sections.push(Section::Feature(maximize_ai_terminal));

    let advanced_setup = ContentSectionData {
        section_name: FeatureSection::AdvancedSetup,
        items: vec![
            ContentItem {
                title: "Use your custom prompt",
                description: "Set up AI Terminal to honor your PS1 setting",
                url: "https://docs.ai-terminal.invalid/terminal/appearance/prompt",
                button_label: "View documentation",
            },
            ContentItem {
                title: "Integrate AI Terminal with your IDE",
                description: "Configure AI Terminal to launch from your most used development tools",
                url: "https://docs.ai-terminal.invalid/terminal/integrations-and-plugins",
                button_label: "View documentation",
            },
            ContentItem {
                title: "How AI Terminal uses AI Terminal",
                description: "Learn how AI Terminal's engineering team uses their favorite features",
                url: "https://www.ai-terminal.invalid/blog/how-ai-terminal-uses-ai-terminal",
                button_label: "Read article",
            },
        ],
    };
    sections.push(Section::Content(advanced_setup));

    sections
}

fn maximize_ai_terminal_items(ctx: &mut ViewContext<ResourceCenterMainView>) -> Vec<FeatureItem> {
    let mut maximize_ai_terminal_items = vec![];

    maximize_ai_terminal_items.push(FeatureItem::new(
        "Command search",
        "Find and run previously executed commands, workflows, and more.",
        Tip::Action(TipAction::CommandSearch),
        ctx,
    ));

    maximize_ai_terminal_items.push(FeatureItem::new(
        "AI command search",
        "Generate shell commands with natural language.",
        Tip::Action(TipAction::AiCommandSearch),
        ctx,
    ));

    if ContextFlag::CreateNewSession.is_enabled() {
        maximize_ai_terminal_items.push(FeatureItem::new(
            "Split panes",
            "Split tabs into multiple panes to make your ideal layout.",
            Tip::Action(TipAction::SplitPane),
            ctx,
        ));
    }

    if ContextFlag::LaunchConfigurations.is_enabled() {
        maximize_ai_terminal_items.push(FeatureItem::new(
            "Launch configuration",
            "Save your current configuration of windows, tabs, and panes.",
            Tip::Action(TipAction::SaveNewLaunchConfig),
            ctx,
        ));
    }

    maximize_ai_terminal_items
}

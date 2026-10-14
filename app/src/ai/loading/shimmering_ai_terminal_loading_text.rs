//! Shimmering AI Terminal loading text - renders AI Terminal logo with shimmering text for loading states.

use ai_terminal_core::ui::appearance::Appearance;
use ai_terminal_ui::elements::Element;
use ai_terminal_ui::elements::shimmering_text::{
    ShimmerConfig, ShimmeringTextElement, ShimmeringTextStateHandle,
};
use ai_terminal_ui::{AppContext, SingletonEntity};

/// AI Terminal icon glyph character
const AI_TERMINAL_GLYPH: &str = "\u{E500}";

/// Creates a shimmering text element with the AI Terminal glyph.
pub fn shimmering_ai_terminal_loading_text(
    text: impl Into<String>,
    font_size: f32,
    shimmer_handle: ShimmeringTextStateHandle,
    app: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();

    // Use same colors as common.rs for consistency
    let base_color = theme.disabled_text_color(theme.surface_1()).into_solid();
    let shimmer_color = theme.main_text_color(theme.surface_1()).into_solid();

    // Hardcoded shimmer config for consistent animation
    let config = ShimmerConfig::default();

    // Create a single shimmering element with glyph and text
    ShimmeringTextElement::new(
        format!("{} {}", AI_TERMINAL_GLYPH, text.into()),
        appearance.ui_font_family(),
        font_size,
        base_color,
        shimmer_color,
        config,
        shimmer_handle,
    )
    .finish()
}

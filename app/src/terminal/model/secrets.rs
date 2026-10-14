pub use ai_terminal_terminal::model::secrets::*;
use ai_terminal_ui::EntityId;
use string_offset::StringRange;

use crate::ai::blocklist::block::TextLocation;
#[derive(Clone, Debug)]
pub struct RichContentSecretTooltipInfo {
    pub secret: String,
    pub secret_range: StringRange,
    pub location: TextLocation,
    pub is_obfuscated: bool,
    pub position_id: String,
    pub view_id: EntityId,
    pub secret_level: SecretLevel,
}

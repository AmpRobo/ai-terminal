use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiTerminalAiOsContext {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub category: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub distribution: Option<String>,
}

/// The execution context of the active session. This struct
/// is sent as a JSON blob in our AI prompts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiTerminalAiExecutionContext {
    pub os: AiTerminalAiOsContext,
    pub shell_name: String,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub shell_version: Option<String>,
}

impl AiTerminalAiExecutionContext {
    pub fn to_json_string(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

use ai_terminal_core::telemetry::{EnablementState, TelemetryEvent, TelemetryEventDesc};
use serde_json::{Value, json};
use strum_macros::{EnumDiscriminants, EnumIter};

use crate::features::FeatureFlag;

#[derive(Debug, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
pub(super) enum CliTelemetryEvent {
    /// Executing `ai-terminal agent run`
    AgentRun {
        gui: bool,
        requested_mcp_servers: usize,
        has_environment: bool,
        /// Optional task ID when running against an ambient agent task.
        task_id: Option<String>,
        /// Which execution harness was selected (e.g. "oz", "claude").
        harness: String,
    },
    /// Executing `ai-terminal agent run-ambient`
    AgentRunAmbient,
    /// Executing `ai-terminal agent profile list`
    AgentProfileList,
    /// Executing `ai-terminal agent list`
    AgentList,
    /// Executing `ai-terminal agent get`
    AgentGet,
    /// Executing `ai-terminal agent create`
    AgentCreate,
    /// Executing `ai-terminal agent update`
    AgentUpdate,
    /// Executing `ai-terminal agent delete`
    AgentDelete,
    /// Executing `ai-terminal agent skills`
    AgentSkills,
    /// Executing `ai-terminal environment list`
    EnvironmentList,
    /// Executing `ai-terminal environment create`
    EnvironmentCreate,
    /// Executing `ai-terminal environment delete`
    EnvironmentDelete,
    /// Executing `ai-terminal environment update`
    EnvironmentUpdate,
    /// Executing `ai-terminal environment get`
    EnvironmentGet,
    /// Executing `ai-terminal environment image list`
    EnvironmentImageList,
    EnvironmentCheckout,
    /// Executing `ai-terminal mcp list`
    MCPList,
    /// Executing `ai-terminal model list`
    ModelList,
    /// Executing `ai-terminal memory-store list`
    MemoryStoreList,
    /// Executing `ai-terminal memory list`
    MemoryStoreListMemories,
    /// Executing `ai-terminal memory create`
    MemoryStoreCreateMemory,
    /// Executing `ai-terminal memory update`
    MemoryStoreUpdateMemory,
    /// Executing `ai-terminal memory delete`
    MemoryStoreDeleteMemory,
    /// Executing `ai-terminal memory-store get`
    MemoryStoreGetStore,
    /// Executing `ai-terminal memory-store update`
    MemoryStoreUpdateStore,
    /// Executing `ai-terminal memory-store list-store-agents`
    MemoryStoreListStoreAgents,
    /// Executing `ai-terminal memory versions`
    MemoryStoreListVersions,
    /// Executing `ai-terminal task list`
    TaskList,
    /// Executing `ai-terminal task get`
    TaskGet,
    /// Executing `ai-terminal run conversation get`
    ConversationGet,
    /// Executing `ai-terminal run get <id> --conversation`
    RunConversationGet,
    /// Executing `ai-terminal run message watch`
    RunMessageWatch {
        harness: &'static str,
    },
    /// Executing `ai-terminal run message send`
    RunMessageSend {
        harness: &'static str,
    },
    /// Executing `ai-terminal run message list`
    RunMessageList {
        harness: &'static str,
    },
    /// Executing `ai-terminal run message read`
    RunMessageRead {
        harness: &'static str,
    },
    /// Executing `ai-terminal run message mark-delivered`
    RunMessageMarkDelivered {
        harness: &'static str,
    },
    /// Executing `ai-terminal login`
    Login,
    /// Executing `ai-terminal logout`
    Logout,
    /// Executing `ai-terminal whoami`
    Whoami,
    /// Executing `ai-terminal provider setup`
    ProviderSetup,
    /// Executing `ai-terminal provider list`
    ProviderList,
    /// Executing `ai-terminal integration create`
    IntegrationCreate,
    /// Executing `ai-terminal integration update`
    IntegrationUpdate,
    /// Executing `ai-terminal integration list`
    IntegrationList,
    /// Executing `ai-terminal artifact upload`
    ArtifactUpload,
    /// Executing `ai-terminal artifact get`
    ArtifactGet,
    /// Executing `ai-terminal artifact download`
    ArtifactDownload,
    /// Executing `ai-terminal api-key list`
    ApiKeyList,
    /// Executing `ai-terminal api-key create`
    ApiKeyCreate,
    /// Executing `ai-terminal api-key expire`
    ApiKeyExpire,
    /// Executing `ai-terminal schedule create`
    ScheduleCreate,
    /// Executing `ai-terminal schedule list`
    ScheduleList,
    /// Executing `ai-terminal schedule get`
    ScheduleGet,
    /// Executing `ai-terminal schedule pause`
    SchedulePause,
    /// Executing `ai-terminal schedule unpause`
    ScheduleUnpause,
    /// Executing `ai-terminal schedule update`
    ScheduleUpdate,
    /// Executing `ai-terminal schedule delete`
    ScheduleDelete,
    /// Executing `ai-terminal secret create`
    SecretCreate,
    /// Executing `ai-terminal secret delete`
    SecretDelete,
    /// Executing `ai-terminal secret update`
    SecretUpdate,
    /// Executing `ai-terminal secret list`
    SecretList,
    /// Executing `ai-terminal federate issue-token`
    FederateIssueToken,
    /// Executing `ai-terminal federate issue-gcp-token`
    FederateIssueGcpToken,
    /// Executing `ai-terminal harness-support ping`
    HarnessSupportPing,
    /// Executing `ai-terminal harness-support report-artifact`
    HarnessSupportReportArtifact {
        artifact_type: &'static str,
    },
    /// Executing `ai-terminal harness-support notify-user`
    HarnessSupportNotifyUser,
    /// Executing `ai-terminal harness-support finish-task`
    HarnessSupportFinishTask {
        success: bool,
    },
    /// Executing `ai-terminal harness-support report-shutdown`
    HarnessSupportReportShutdown,
    /// Executing `ai-terminal runner list`
    RunnerList,
    /// Executing `ai-terminal runner create`
    RunnerCreate,
    /// Executing `ai-terminal runner update`
    RunnerUpdate,
    /// Executing `ai-terminal runner delete`
    RunnerDelete,
}

impl TelemetryEvent for CliTelemetryEvent {
    fn name(&self) -> &'static str {
        CliTelemetryEventDiscriminants::from(self).name()
    }

    fn payload(&self) -> Option<Value> {
        match self {
            CliTelemetryEvent::AgentRun {
                gui,
                requested_mcp_servers,
                has_environment,
                task_id,
                harness,
            } => Some(json!({
                "gui": gui,
                "requested_mcp_servers": requested_mcp_servers,
                "has_environment": has_environment,
                "task_id": task_id,
                "harness": harness,
            })),
            CliTelemetryEvent::AgentRunAmbient => None,
            CliTelemetryEvent::AgentProfileList => None,
            CliTelemetryEvent::AgentList => None,
            CliTelemetryEvent::AgentGet => None,
            CliTelemetryEvent::AgentCreate => None,
            CliTelemetryEvent::AgentUpdate => None,
            CliTelemetryEvent::AgentDelete => None,
            CliTelemetryEvent::AgentSkills => None,
            CliTelemetryEvent::EnvironmentList => None,
            CliTelemetryEvent::EnvironmentCreate => None,
            CliTelemetryEvent::EnvironmentDelete => None,
            CliTelemetryEvent::EnvironmentUpdate => None,
            CliTelemetryEvent::EnvironmentGet => None,
            CliTelemetryEvent::EnvironmentImageList => None,
            CliTelemetryEvent::EnvironmentCheckout => None,
            CliTelemetryEvent::MCPList => None,
            CliTelemetryEvent::ModelList => None,
            CliTelemetryEvent::MemoryStoreList => None,
            CliTelemetryEvent::MemoryStoreListMemories => None,
            CliTelemetryEvent::MemoryStoreCreateMemory => None,
            CliTelemetryEvent::MemoryStoreUpdateMemory => None,
            CliTelemetryEvent::MemoryStoreDeleteMemory => None,
            CliTelemetryEvent::MemoryStoreGetStore => None,
            CliTelemetryEvent::MemoryStoreUpdateStore => None,
            CliTelemetryEvent::MemoryStoreListStoreAgents => None,
            CliTelemetryEvent::MemoryStoreListVersions => None,
            CliTelemetryEvent::TaskList => None,
            CliTelemetryEvent::TaskGet => None,
            CliTelemetryEvent::ConversationGet => None,
            CliTelemetryEvent::RunConversationGet => None,
            CliTelemetryEvent::RunMessageWatch { harness } => Some(json!({ "harness": harness })),
            CliTelemetryEvent::RunMessageSend { harness } => Some(json!({ "harness": harness })),
            CliTelemetryEvent::RunMessageList { harness } => Some(json!({ "harness": harness })),
            CliTelemetryEvent::RunMessageRead { harness } => Some(json!({ "harness": harness })),
            CliTelemetryEvent::RunMessageMarkDelivered { harness } => {
                Some(json!({ "harness": harness }))
            }
            CliTelemetryEvent::Login => None,
            CliTelemetryEvent::Logout => None,
            CliTelemetryEvent::Whoami => None,
            CliTelemetryEvent::ProviderSetup => None,
            CliTelemetryEvent::ProviderList => None,
            CliTelemetryEvent::IntegrationCreate => None,
            CliTelemetryEvent::IntegrationUpdate => None,
            CliTelemetryEvent::IntegrationList => None,
            CliTelemetryEvent::ArtifactUpload => None,
            CliTelemetryEvent::ArtifactGet => None,
            CliTelemetryEvent::ArtifactDownload => None,
            CliTelemetryEvent::ApiKeyList => None,
            CliTelemetryEvent::ApiKeyCreate => None,
            CliTelemetryEvent::ApiKeyExpire => None,
            CliTelemetryEvent::ScheduleCreate => None,
            CliTelemetryEvent::ScheduleList => None,
            CliTelemetryEvent::ScheduleGet => None,
            CliTelemetryEvent::SchedulePause => None,
            CliTelemetryEvent::ScheduleUnpause => None,
            CliTelemetryEvent::ScheduleUpdate => None,
            CliTelemetryEvent::ScheduleDelete => None,
            CliTelemetryEvent::SecretCreate => None,
            CliTelemetryEvent::SecretDelete => None,
            CliTelemetryEvent::SecretUpdate => None,
            CliTelemetryEvent::SecretList => None,
            CliTelemetryEvent::FederateIssueToken => None,
            CliTelemetryEvent::FederateIssueGcpToken => None,
            CliTelemetryEvent::HarnessSupportPing => None,
            CliTelemetryEvent::HarnessSupportReportArtifact { artifact_type } => {
                Some(json!({ "artifact_type": artifact_type }))
            }
            CliTelemetryEvent::HarnessSupportNotifyUser => None,
            CliTelemetryEvent::HarnessSupportFinishTask { success } => {
                Some(json!({ "success": success }))
            }
            CliTelemetryEvent::HarnessSupportReportShutdown => None,
            CliTelemetryEvent::RunnerList => None,
            CliTelemetryEvent::RunnerCreate => None,
            CliTelemetryEvent::RunnerUpdate => None,
            CliTelemetryEvent::RunnerDelete => None,
        }
    }

    fn description(&self) -> &'static str {
        CliTelemetryEventDiscriminants::from(self).description()
    }

    fn enablement_state(&self) -> EnablementState {
        CliTelemetryEventDiscriminants::from(self).enablement_state()
    }

    fn contains_ugc(&self) -> bool {
        false
    }

    fn event_descs() -> impl Iterator<Item = Box<dyn TelemetryEventDesc>> {
        ai_terminal_core::telemetry::enum_events::<Self>()
    }
}

impl TelemetryEventDesc for CliTelemetryEventDiscriminants {
    fn name(&self) -> &'static str {
        match self {
            CliTelemetryEventDiscriminants::AgentRun => "CLI.Execute.Agent.Run",
            CliTelemetryEventDiscriminants::AgentRunAmbient => "CLI.Execute.Agent.RunAmbient",
            CliTelemetryEventDiscriminants::AgentProfileList => "CLI.Execute.Agent.Profile.List",
            CliTelemetryEventDiscriminants::AgentList => "CLI.Execute.Agent.List",
            CliTelemetryEventDiscriminants::AgentGet => "CLI.Execute.Agent.Get",
            CliTelemetryEventDiscriminants::AgentCreate => "CLI.Execute.Agent.Create",
            CliTelemetryEventDiscriminants::AgentUpdate => "CLI.Execute.Agent.Update",
            CliTelemetryEventDiscriminants::AgentDelete => "CLI.Execute.Agent.Delete",
            CliTelemetryEventDiscriminants::AgentSkills => "CLI.Execute.Agent.Skills",
            CliTelemetryEventDiscriminants::EnvironmentList => "CLI.Execute.Environment.List",
            CliTelemetryEventDiscriminants::EnvironmentCreate => "CLI.Execute.Environment.Create",
            CliTelemetryEventDiscriminants::EnvironmentDelete => "CLI.Execute.Environment.Delete",
            CliTelemetryEventDiscriminants::EnvironmentUpdate => "CLI.Execute.Environment.Update",
            CliTelemetryEventDiscriminants::EnvironmentGet => "CLI.Execute.Environment.Get",
            CliTelemetryEventDiscriminants::EnvironmentImageList => {
                "CLI.Execute.Environment.Image.List"
            }
            CliTelemetryEventDiscriminants::EnvironmentCheckout => {
                "CLI.Execute.EnvironmentCheckout"
            }
            CliTelemetryEventDiscriminants::MCPList => "CLI.Execute.MCP.List",
            CliTelemetryEventDiscriminants::ModelList => "CLI.Execute.Model.List",
            CliTelemetryEventDiscriminants::MemoryStoreList => "CLI.Execute.MemoryStore.List",
            CliTelemetryEventDiscriminants::MemoryStoreListMemories => {
                "CLI.Execute.MemoryStore.ListMemories"
            }
            CliTelemetryEventDiscriminants::MemoryStoreCreateMemory => {
                "CLI.Execute.MemoryStore.CreateMemory"
            }
            CliTelemetryEventDiscriminants::MemoryStoreUpdateMemory => {
                "CLI.Execute.MemoryStore.UpdateMemory"
            }
            CliTelemetryEventDiscriminants::MemoryStoreDeleteMemory => {
                "CLI.Execute.MemoryStore.DeleteMemory"
            }
            CliTelemetryEventDiscriminants::MemoryStoreGetStore => {
                "CLI.Execute.MemoryStore.GetStore"
            }
            CliTelemetryEventDiscriminants::MemoryStoreUpdateStore => {
                "CLI.Execute.MemoryStore.UpdateStore"
            }
            CliTelemetryEventDiscriminants::MemoryStoreListStoreAgents => {
                "CLI.Execute.MemoryStore.ListStoreAgents"
            }
            CliTelemetryEventDiscriminants::MemoryStoreListVersions => {
                "CLI.Execute.MemoryStore.ListVersions"
            }
            CliTelemetryEventDiscriminants::TaskList => "CLI.Execute.Task.List",
            CliTelemetryEventDiscriminants::TaskGet => "CLI.Execute.Task.Get",
            CliTelemetryEventDiscriminants::ConversationGet => "CLI.Execute.Conversation.Get",
            CliTelemetryEventDiscriminants::RunConversationGet => {
                "CLI.Execute.Run.Conversation.Get"
            }
            CliTelemetryEventDiscriminants::RunMessageWatch => "CLI.Execute.Run.Message.Watch",
            CliTelemetryEventDiscriminants::RunMessageSend => "CLI.Execute.Run.Message.Send",
            CliTelemetryEventDiscriminants::RunMessageList => "CLI.Execute.Run.Message.List",
            CliTelemetryEventDiscriminants::RunMessageRead => "CLI.Execute.Run.Message.Read",
            CliTelemetryEventDiscriminants::RunMessageMarkDelivered => {
                "CLI.Execute.Run.Message.MarkDelivered"
            }
            CliTelemetryEventDiscriminants::Login => "CLI.Execute.Login",
            CliTelemetryEventDiscriminants::Logout => "CLI.Execute.Logout",
            CliTelemetryEventDiscriminants::Whoami => "CLI.Execute.Whoami",
            CliTelemetryEventDiscriminants::ProviderSetup => "CLI.Execute.Provider.Setup",
            CliTelemetryEventDiscriminants::ProviderList => "CLI.Execute.Provider.List",
            CliTelemetryEventDiscriminants::IntegrationCreate => "CLI.Execute.Integration.Create",
            CliTelemetryEventDiscriminants::IntegrationUpdate => "CLI.Execute.Integration.Update",
            CliTelemetryEventDiscriminants::IntegrationList => "CLI.Execute.Integration.List",
            CliTelemetryEventDiscriminants::ArtifactUpload => "CLI.Execute.Artifact.Upload",
            CliTelemetryEventDiscriminants::ArtifactGet => "CLI.Execute.Artifact.Get",
            CliTelemetryEventDiscriminants::ArtifactDownload => "CLI.Execute.Artifact.Download",
            CliTelemetryEventDiscriminants::ApiKeyList => "CLI.Execute.ApiKey.List",
            CliTelemetryEventDiscriminants::ApiKeyCreate => "CLI.Execute.ApiKey.Create",
            CliTelemetryEventDiscriminants::ApiKeyExpire => "CLI.Execute.ApiKey.Expire",
            CliTelemetryEventDiscriminants::ScheduleCreate => "CLI.Execute.Schedule.Create",
            CliTelemetryEventDiscriminants::ScheduleList => "CLI.Execute.Schedule.List",
            CliTelemetryEventDiscriminants::ScheduleGet => "CLI.Execute.Schedule.Get",
            CliTelemetryEventDiscriminants::SchedulePause => "CLI.Execute.Schedule.Pause",
            CliTelemetryEventDiscriminants::ScheduleUnpause => "CLI.Execute.Schedule.Unpause",
            CliTelemetryEventDiscriminants::ScheduleUpdate => "CLI.Execute.Schedule.Update",
            CliTelemetryEventDiscriminants::ScheduleDelete => "CLI.Execute.Schedule.Delete",
            CliTelemetryEventDiscriminants::SecretCreate => "CLI.Execute.Secret.Create",
            CliTelemetryEventDiscriminants::SecretDelete => "CLI.Execute.Secret.Delete",
            CliTelemetryEventDiscriminants::SecretUpdate => "CLI.Execute.Secret.Update",
            CliTelemetryEventDiscriminants::SecretList => "CLI.Execute.Secret.List",
            CliTelemetryEventDiscriminants::FederateIssueToken => "CLI.Execute.Federate.IssueToken",
            CliTelemetryEventDiscriminants::FederateIssueGcpToken => {
                "CLI.Execute.Federate.IssueGcpToken"
            }
            CliTelemetryEventDiscriminants::HarnessSupportPing => "CLI.Execute.HarnessSupport.Ping",
            CliTelemetryEventDiscriminants::HarnessSupportReportArtifact => {
                "CLI.Execute.HarnessSupport.ReportArtifact"
            }
            CliTelemetryEventDiscriminants::HarnessSupportNotifyUser => {
                "CLI.Execute.HarnessSupport.NotifyUser"
            }
            CliTelemetryEventDiscriminants::HarnessSupportFinishTask => {
                "CLI.Execute.HarnessSupport.FinishTask"
            }
            CliTelemetryEventDiscriminants::HarnessSupportReportShutdown => {
                "CLI.Execute.HarnessSupport.ReportShutdown"
            }
            CliTelemetryEventDiscriminants::RunnerList => "CLI.Execute.Runner.List",
            CliTelemetryEventDiscriminants::RunnerCreate => "CLI.Execute.Runner.Create",
            CliTelemetryEventDiscriminants::RunnerUpdate => "CLI.Execute.Runner.Update",
            CliTelemetryEventDiscriminants::RunnerDelete => "CLI.Execute.Runner.Delete",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            CliTelemetryEventDiscriminants::AgentRun => "Ran an agent from the AI Terminal CLI",
            CliTelemetryEventDiscriminants::AgentRunAmbient => {
                "Ran an ambient agent from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::AgentProfileList => {
                "Listed agent profiles from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::AgentList => "Listed agents from the AI Terminal CLI",
            CliTelemetryEventDiscriminants::AgentGet => {
                "Got agent details from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::AgentCreate => {
                "Created an agent from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::AgentUpdate => {
                "Updated an agent from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::AgentDelete => {
                "Deleted an agent from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::AgentSkills => {
                "Listed agent skills from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::EnvironmentList => {
                "Listed cloud environments from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::EnvironmentCreate => {
                "Created a cloud environment from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::EnvironmentDelete => {
                "Deleted a cloud environment from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::EnvironmentUpdate => {
                "Updated a cloud environment from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::EnvironmentGet => {
                "Got cloud environment details from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::EnvironmentImageList => {
                "Listed available base images from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::EnvironmentCheckout => {
                "Prepared structured environment checkouts"
            }
            CliTelemetryEventDiscriminants::MCPList => {
                "Listed MCP servers from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ModelList => "Listed models from the AI Terminal CLI",
            CliTelemetryEventDiscriminants::MemoryStoreList => {
                "Listed memory stores from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::MemoryStoreListMemories => {
                "Listed memories in a memory store from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::MemoryStoreCreateMemory => {
                "Created a manual memory in a memory store from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::MemoryStoreUpdateMemory => {
                "Updated a memory in a memory store from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::MemoryStoreDeleteMemory => {
                "Deleted a memory from a memory store from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::MemoryStoreGetStore => {
                "Got a memory store from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::MemoryStoreUpdateStore => {
                "Updated a memory store from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::MemoryStoreListStoreAgents => {
                "Listed agents attached to a memory store from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::MemoryStoreListVersions => {
                "Listed version history of a memory from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::TaskList => "Listed tasks from the AI Terminal CLI",
            CliTelemetryEventDiscriminants::TaskGet => {
                "Got status of task from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ConversationGet => {
                "Got conversation by ID from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::RunConversationGet => {
                "Got run conversation from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::RunMessageWatch => {
                "Watched run messages from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::RunMessageSend => {
                "Sent a run message from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::RunMessageList => {
                "Listed run messages from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::RunMessageRead => {
                "Read a run message from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::RunMessageMarkDelivered => {
                "Marked a run message as delivered from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::Login => "Logged in via the AI Terminal CLI",
            CliTelemetryEventDiscriminants::Logout => "Logged out via the AI Terminal CLI",
            CliTelemetryEventDiscriminants::Whoami => {
                "Printed current user info from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ProviderSetup => {
                "Set up a provider via the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ProviderList => {
                "Listed providers from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::IntegrationCreate => {
                "Created an integration from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::IntegrationUpdate => {
                "Updated an integration from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::IntegrationList => {
                "Listed integrations from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ArtifactUpload => {
                "Uploaded an artifact from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ArtifactGet => {
                "Got artifact metadata from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ArtifactDownload => {
                "Downloaded an artifact from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ApiKeyList => {
                "Listed API keys from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ApiKeyCreate => {
                "Created an API key from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ApiKeyExpire => {
                "Expired an API key from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ScheduleCreate => {
                "Created a scheduled agent from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ScheduleList => {
                "Listed scheduled agents from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ScheduleGet => {
                "Got scheduled agent configuration from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::SchedulePause => {
                "Paused a scheduled agent from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ScheduleUnpause => {
                "Unpaused a scheduled agent from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ScheduleUpdate => {
                "Updated a scheduled agent from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::ScheduleDelete => {
                "Deleted a scheduled agent from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::SecretCreate => {
                "Created a secret from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::SecretDelete => {
                "Deleted a secret from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::SecretUpdate => {
                "Updated a secret from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::SecretList => "Listed secrets from the AI Terminal CLI",
            CliTelemetryEventDiscriminants::FederateIssueToken => {
                "Issued a federated identity token from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::FederateIssueGcpToken => {
                "Issued a GCP federated identity token from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::HarnessSupportPing => {
                "Pinged harness-support from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::HarnessSupportReportArtifact => {
                "Reported an artifact via harness-support from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::HarnessSupportNotifyUser => {
                "Sent a user notification via harness-support from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::HarnessSupportFinishTask => {
                "Reported task completion via harness-support from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::HarnessSupportReportShutdown => {
                "Reported agent shutdown via harness-support from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::RunnerList => "Listed runners from the AI Terminal CLI",
            CliTelemetryEventDiscriminants::RunnerCreate => {
                "Created a runner from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::RunnerUpdate => {
                "Updated a runner from the AI Terminal CLI"
            }
            CliTelemetryEventDiscriminants::RunnerDelete => {
                "Deleted a runner from the AI Terminal CLI"
            }
        }
    }

    fn enablement_state(&self) -> EnablementState {
        match self {
            Self::FederateIssueToken | Self::FederateIssueGcpToken => {
                EnablementState::Flag(FeatureFlag::OzIdentityFederation)
            }
            Self::HarnessSupportPing
            | Self::HarnessSupportReportArtifact
            | Self::HarnessSupportNotifyUser
            | Self::HarnessSupportFinishTask => EnablementState::Flag(FeatureFlag::AgentHarness),
            Self::ArtifactUpload | Self::ArtifactGet | Self::ArtifactDownload => {
                EnablementState::Flag(FeatureFlag::ArtifactCommand)
            }
            Self::ApiKeyList | Self::ApiKeyCreate | Self::ApiKeyExpire => {
                EnablementState::Flag(FeatureFlag::APIKeyManagement)
            }
            Self::RunnerList | Self::RunnerCreate | Self::RunnerUpdate | Self::RunnerDelete => {
                EnablementState::Flag(FeatureFlag::CloudAgentRunners)
            }
            Self::RunMessageWatch
            | Self::RunMessageSend
            | Self::RunMessageList
            | Self::RunMessageRead
            | Self::RunMessageMarkDelivered => EnablementState::Always,
            _ => EnablementState::Always,
        }
    }
}

ai_terminal_core::register_telemetry_event!(CliTelemetryEvent);

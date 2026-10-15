//! GUI agent mode backed by a local `qwen --acp` process.
//!
//! The client speaks ACP over the process's stdio, reads and writes files, and runs shell
//! commands on this machine. Model calls stay inside Qwen Code. This module does not open a
//! network connection.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use ai_terminal_core::channel::ChannelState;
use ai_terminal_multi_agent_api::response_event::{StreamFinished, stream_finished};
use ai_terminal_multi_agent_api::{ResponseEvent, response_event};
use ai_terminal_ui::r#async::Timer;
use ai_terminal_ui::r#async::executor::Background;
use ai_terminal_ui::{EntityId, ModelSpawner, SingletonEntity};
use ai_terminal_util::path::EscapeChar;
use anyhow::{Context as _, Result, anyhow};
use futures::{FutureExt, pin_mut, select};
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::{Value, json};

use super::connection::{AcpConnection, InboundNotification, RpcError};
use super::mapping::{AcpTurnMapper, TurnEvent};
use super::policy::{PolicyDecision, PolicyRequest};
use super::protocol::{
    CancelParams, ClientCapabilities, ClientInfo, ContentBlock, FsCapabilities, InitializeParams,
    InitializeResponse, NewSessionParams, NewSessionResponse, PROTOCOL_VERSION, PermissionOption,
    PermissionOutcome, PromptParams, PromptResponse, RequestPermissionParams,
    RequestPermissionResponse, SessionUpdateParams, StopReason,
};
use super::{read_text_file, stream_finished_for};
use crate::ai::agent::conversation::AIConversationId;
use crate::ai::blocklist::history_model::{BlocklistAIHistoryModel, UpdateHistoryError};
use crate::ai::blocklist::{BlocklistAIController, ExternalHarnessTurn};

const QWEN_PROGRAM: &str = "qwen";
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_TERMINAL_OUTPUT_LIMIT: usize = 1024 * 1024;

/// Tool names Qwen Code must not register. Shell, git, and file tools stay available.
const EXCLUDED_NETWORK_TOOLS: &[&str] = &["web_fetch", "web_search", "google_web_search"];

const METHOD_TERMINAL_CREATE: &str = "terminal/create";
const METHOD_TERMINAL_OUTPUT: &str = "terminal/output";
const METHOD_TERMINAL_WAIT: &str = "terminal/wait_for_exit";
const METHOD_TERMINAL_KILL: &str = "terminal/kill";
const METHOD_TERMINAL_RELEASE: &str = "terminal/release";

/// Arguments passed to the local `qwen` binary.
pub(super) fn qwen_arguments() -> Vec<String> {
    vec![
        "--acp".to_owned(),
        "--approval-mode".to_owned(),
        "yolo".to_owned(),
        "--exclude-tools".to_owned(),
        EXCLUDED_NETWORK_TOOLS.join(","),
    ]
}

/// Qwen Code rejects `session/new` until an auth type is selected. With no
/// `OPENAI_API_KEY`, these values select its OpenAI-compatible auth type so the ACP process can
/// start. A key already in the environment is left as-is; later model calls use that key together
/// with `OPENAI_MODEL` and `OPENAI_BASE_URL`.
pub(super) fn fallback_auth_env(
    existing_api_key: Option<&std::ffi::OsStr>,
) -> Option<[(&'static str, &'static str); 3]> {
    if existing_api_key.is_some() {
        return None;
    }
    Some([
        ("OPENAI_API_KEY", "local"),
        ("OPENAI_MODEL", "local"),
        ("OPENAI_BASE_URL", "http://127.0.0.1:9"),
    ])
}

/// Whether an ACP tool kind is something this local client will allow.
pub(super) enum LocalToolPermission {
    Allow,
    Deny { reason: &'static str },
}

pub(super) fn permission_for_tool_kind(
    kind: Option<super::protocol::ToolKind>,
) -> LocalToolPermission {
    use super::protocol::ToolKind;
    match kind {
        Some(ToolKind::Fetch) => LocalToolPermission::Deny {
            reason: "network tools are disabled; this agent only reads, edits, and runs local commands",
        },
        Some(
            ToolKind::Read
            | ToolKind::Edit
            | ToolKind::Delete
            | ToolKind::Move
            | ToolKind::Search
            | ToolKind::Execute
            | ToolKind::Think
            | ToolKind::SwitchMode
            | ToolKind::Other
            | ToolKind::Unknown,
        )
        | None => LocalToolPermission::Allow,
    }
}

/// A user prompt for one local Qwen Code turn.
#[derive(Clone, Debug)]
pub(crate) struct LocalQwenTurn {
    pub text: String,
    pub cwd: PathBuf,
}

/// Identifiers the session needs to render a turn into the agent view.
pub(crate) struct LocalQwenLaunch {
    pub conversation_id: AIConversationId,
    pub terminal_surface_id: EntityId,
    pub escape_char: EscapeChar,
}

enum SessionCommand {
    Prompt(LocalQwenTurn),
    Cancel,
}

/// A live `qwen --acp` process bound to one GUI conversation.
pub(crate) struct LocalQwenSession {
    inbox: async_channel::Sender<SessionCommand>,
}

impl LocalQwenSession {
    /// Spawns the session task. The process itself starts when the first prompt arrives.
    pub(crate) fn open(
        foreground: ModelSpawner<BlocklistAIController>,
        background: Arc<Background>,
        launch: LocalQwenLaunch,
    ) -> Self {
        let (inbox, commands) = async_channel::unbounded();
        let executor = background.clone();
        background
            .spawn(async move {
                run_session(foreground, executor, commands, launch).await;
            })
            .detach();
        Self { inbox }
    }

    pub(crate) fn submit(&self, turn: LocalQwenTurn) -> Result<(), ()> {
        self.inbox
            .try_send(SessionCommand::Prompt(turn))
            .map_err(|_| ())
    }

    pub(crate) fn cancel(&self) {
        let _ = self.inbox.try_send(SessionCommand::Cancel);
    }
}

struct StartedTurn {
    turn: ExternalHarnessTurn,
    task_id: String,
}

struct QwenProcess {
    connection: Arc<AcpConnection>,
    notifications: async_channel::Receiver<InboundNotification>,
    session_id: String,
    cwd: PathBuf,
    /// Kept so `kill_on_drop` ends the process when the session shuts down.
    /// The mutex exists so the process handle is `Sync`: the turn loop holds `&QwenProcess`
    /// across awaits, and `Box<dyn Any + Send>` is not `Sync`.
    _child: Mutex<Box<dyn std::any::Any + Send>>,
    client: Arc<LocalClient>,
}

struct LocalClient {
    foreground: ModelSpawner<BlocklistAIController>,
    terminal_surface_id: EntityId,
    escape_char: EscapeChar,
    cwd: Mutex<PathBuf>,
    terminals: Mutex<HashMap<String, LocalTerminal>>,
}

enum PromptOutcome {
    Finished,
    Interrupted(SessionCommand),
    /// The turn was already written into the conversation, and the process is no longer usable.
    AgentLost,
    /// The turn never opened, so the caller still has to surface `message`.
    Unreported(String),
    Stopped,
}

async fn run_session(
    foreground: ModelSpawner<BlocklistAIController>,
    background: Arc<Background>,
    commands: async_channel::Receiver<SessionCommand>,
    launch: LocalQwenLaunch,
) {
    let client = Arc::new(LocalClient {
        foreground: foreground.clone(),
        terminal_surface_id: launch.terminal_surface_id,
        escape_char: launch.escape_char,
        cwd: Mutex::new(PathBuf::new()),
        terminals: Mutex::new(HashMap::new()),
    });
    let mut agent: Option<QwenProcess> = None;
    let mut pending = match commands.recv().await {
        Ok(command) => command,
        Err(_) => return,
    };
    loop {
        let turn = match pending {
            SessionCommand::Cancel => {
                if let Some(agent) = agent.as_ref() {
                    let _ = agent
                        .connection
                        .notify(
                            super::protocol::METHOD_SESSION_CANCEL,
                            CancelParams {
                                session_id: agent.session_id.clone(),
                            },
                        )
                        .await;
                }
                pending = match commands.recv().await {
                    Ok(command) => command,
                    Err(_) => break,
                };
                continue;
            }
            SessionCommand::Prompt(turn) => turn,
        };

        if agent.as_ref().is_some_and(|agent| agent.cwd != turn.cwd)
            && let Some(agent) = agent.take()
        {
            agent.shutdown().await;
        }
        if agent.is_none() {
            match QwenProcess::start(turn.cwd.clone(), client.clone(), background.clone()).await {
                Ok(process) => agent = Some(process),
                Err(error) => {
                    log::warn!("Local Qwen Code did not start");
                    if report_turn_failure(&foreground, launch.conversation_id, &turn.text, &error)
                        .await
                        .is_err()
                    {
                        break;
                    }
                    pending = match commands.recv().await {
                        Ok(command) => command,
                        Err(_) => break,
                    };
                    continue;
                }
            }
        }
        let Some(process) = agent.as_ref() else {
            pending = SessionCommand::Prompt(turn);
            continue;
        };
        match run_prompt(
            process,
            &foreground,
            launch.conversation_id,
            &turn.text,
            &commands,
        )
        .await
        {
            PromptOutcome::Finished => {
                pending = match commands.recv().await {
                    Ok(command) => command,
                    Err(_) => break,
                };
            }
            PromptOutcome::Interrupted(command) => pending = command,
            PromptOutcome::AgentLost => {
                log::error!("Local Qwen Code stopped during a turn");
                if let Some(process) = agent.take() {
                    process.shutdown().await;
                }
                pending = match commands.recv().await {
                    Ok(command) => command,
                    Err(_) => break,
                };
            }
            PromptOutcome::Unreported(error) => {
                log::warn!("Local Qwen Code turn did not start");
                if let Some(process) = agent.take() {
                    process.shutdown().await;
                }
                if report_turn_failure(&foreground, launch.conversation_id, &turn.text, &error)
                    .await
                    .is_err()
                {
                    break;
                }
                pending = match commands.recv().await {
                    Ok(command) => command,
                    Err(_) => break,
                };
            }
            PromptOutcome::Stopped => break,
        }
    }
    if let Some(process) = agent.take() {
        process.shutdown().await;
    }
}

impl QwenProcess {
    async fn start(
        cwd: PathBuf,
        client: Arc<LocalClient>,
        background: Arc<Background>,
    ) -> Result<Self, String> {
        let program = crate::util::path::resolve_executable(QWEN_PROGRAM).ok_or_else(|| {
            format!(
                "'{QWEN_PROGRAM}' was not found on PATH. Install Qwen Code and retry this prompt."
            )
        })?;
        *client.cwd.lock() = cwd.clone();
        let mut command = command::r#async::Command::new(program.as_ref());
        command
            .args(qwen_arguments())
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(vars) = fallback_auth_env(std::env::var_os("OPENAI_API_KEY").as_deref()) {
            command.envs(vars);
        }
        let mut child = command
            .spawn()
            .map_err(|error| format!("Failed to start '{QWEN_PROGRAM}': {error}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| format!("'{QWEN_PROGRAM}' has no stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| format!("'{QWEN_PROGRAM}' has no stdout"))?;
        if let Some(stderr) = child.stderr.take() {
            background
                .spawn(async move {
                    let mut reader = futures::io::BufReader::new(stderr);
                    let mut line = String::new();
                    loop {
                        line.clear();
                        match futures::io::AsyncBufReadExt::read_line(&mut reader, &mut line).await
                        {
                            Ok(0) | Err(_) => break,
                            Ok(_) => {}
                        }
                    }
                })
                .detach();
        }
        let handler_client = client.clone();
        let (connection, notifications) = AcpConnection::new(
            stdout,
            stdin,
            Arc::new(move |method, params| {
                let client = handler_client.clone();
                let method = method.to_owned();
                async move { answer_agent_request(&client, &method, params).await }.boxed()
            }),
            &background,
        );
        let connection = Arc::new(connection);
        let session_id = handshake(&connection, &cwd).await?;
        log::info!("Local Qwen Code ACP session is ready");
        Ok(Self {
            connection,
            notifications,
            session_id,
            cwd,
            _child: Mutex::new(Box::new(child)),
            client,
        })
    }

    async fn shutdown(self) {
        let _ = self
            .connection
            .notify(
                super::protocol::METHOD_SESSION_CANCEL,
                CancelParams {
                    session_id: self.session_id,
                },
            )
            .await;
        self.connection.close().await;
        drop(self._child);
        self.client.terminals.lock().clear();
    }
}

async fn handshake(connection: &AcpConnection, cwd: &Path) -> Result<String, String> {
    let handshake = async {
        let initialize: InitializeResponse = connection
            .request(
                super::protocol::METHOD_INITIALIZE,
                InitializeParams {
                    protocol_version: PROTOCOL_VERSION,
                    client_capabilities: ClientCapabilities {
                        fs: FsCapabilities {
                            read_text_file: true,
                            write_text_file: true,
                        },
                        terminal: true,
                    },
                    client_info: ClientInfo {
                        name: "ai-terminal".to_owned(),
                        title: "AI Terminal".to_owned(),
                        version: ChannelState::app_version()
                            .unwrap_or(env!("CARGO_PKG_VERSION"))
                            .to_owned(),
                    },
                },
            )
            .await
            .context("initialize failed")?;
        if initialize.protocol_version != PROTOCOL_VERSION {
            anyhow::bail!(
                "qwen speaks ACP protocol v{} but this client requires v{PROTOCOL_VERSION}",
                initialize.protocol_version
            );
        }
        let session: NewSessionResponse = connection
            .request(
                super::protocol::METHOD_SESSION_NEW,
                NewSessionParams {
                    cwd: cwd.display().to_string(),
                    mcp_servers: Vec::new(),
                },
            )
            .await
            .context("session/new failed")?;
        Ok::<_, anyhow::Error>(session.session_id)
    }
    .fuse();
    pin_mut!(handshake);
    select! {
        result = handshake => result.map_err(|error| format!("{error:#}")),
        _ = Timer::after(HANDSHAKE_TIMEOUT).fuse() => Err(format!(
            "qwen did not finish its ACP handshake within {HANDSHAKE_TIMEOUT:?}"
        )),
    }
}

async fn run_prompt(
    process: &QwenProcess,
    foreground: &ModelSpawner<BlocklistAIController>,
    conversation_id: AIConversationId,
    prompt: &str,
    commands: &async_channel::Receiver<SessionCommand>,
) -> PromptOutcome {
    let started = match begin_turn(foreground, conversation_id).await {
        Ok(started) => started,
        Err(error) => return PromptOutcome::Unreported(error),
    };
    let mut mapper = AcpTurnMapper::new(started.task_id.clone(), started.turn.request_id.clone());
    let initial = vec![TurnEvent::Actions(mapper.initial_actions(prompt))];
    let mut current = started;
    if apply_turn_events(
        foreground,
        conversation_id,
        &mut mapper,
        &mut current,
        initial,
    )
    .await
    .is_err()
    {
        return PromptOutcome::Stopped;
    }

    let prompt_fut = process
        .connection
        .request::<_, PromptResponse>(
            super::protocol::METHOD_SESSION_PROMPT,
            PromptParams {
                session_id: process.session_id.clone(),
                prompt: vec![ContentBlock::Text {
                    text: prompt.to_owned(),
                }],
            },
        )
        .fuse();
    pin_mut!(prompt_fut);
    loop {
        select! {
            notification = process.notifications.recv().fuse() => {
                match notification {
                    Ok(notification) => {
                        if handle_notification(
                            process,
                            foreground,
                            conversation_id,
                            &mut mapper,
                            &mut current,
                            notification,
                        )
                        .await
                        .is_err()
                        {
                            return PromptOutcome::Stopped;
                        }
                    }
                    Err(_) => {
                        let _ = finish_stream(
                            foreground,
                            current.turn.stream_id.clone(),
                            failed_stream("qwen closed its output"),
                        )
                        .await;
                        return PromptOutcome::AgentLost;
                    }
                }
            }
            response = prompt_fut => {
                let stop_reason = match response {
                    Ok(response) => response.stop_reason,
                    Err(error) => {
                        let message = format!("{error:#}");
                        let _ = finish_stream(
                            foreground,
                            current.turn.stream_id.clone(),
                            failed_stream(&message),
                        )
                        .await;
                        return PromptOutcome::AgentLost;
                    }
                };
                while let Ok(notification) = process.notifications.try_recv() {
                    if handle_notification(
                        process,
                        foreground,
                        conversation_id,
                        &mut mapper,
                        &mut current,
                        notification,
                    )
                    .await
                    .is_err()
                    {
                        return PromptOutcome::Stopped;
                    }
                }
                let finished = mapper.finish_events();
                if apply_turn_events(
                    foreground,
                    conversation_id,
                    &mut mapper,
                    &mut current,
                    finished,
                )
                .await
                .is_err()
                {
                    return PromptOutcome::Stopped;
                }
                if finish_stream(
                    foreground,
                    current.turn.stream_id.clone(),
                    stream_finished_for(stop_reason),
                )
                .await
                .is_err()
                {
                    return PromptOutcome::Stopped;
                }
                return PromptOutcome::Finished;
            }
            command = commands.recv().fuse() => {
                let Ok(command) = command else {
                    return PromptOutcome::Stopped;
                };
                let _ = process
                    .connection
                    .notify(
                        super::protocol::METHOD_SESSION_CANCEL,
                        CancelParams {
                            session_id: process.session_id.clone(),
                        },
                    )
                    .await;
                select! {
                    response = prompt_fut => {
                        let _ = response;
                    }
                    _ = Timer::after(Duration::from_secs(2)).fuse() => {}
                }
                let _ = finish_stream(
                    foreground,
                    current.turn.stream_id.clone(),
                    stream_finished_for(StopReason::Cancelled),
                )
                .await;
                return PromptOutcome::Interrupted(command);
            }
        }
    }
}

async fn handle_notification(
    _process: &QwenProcess,
    foreground: &ModelSpawner<BlocklistAIController>,
    conversation_id: AIConversationId,
    mapper: &mut AcpTurnMapper,
    current: &mut StartedTurn,
    notification: InboundNotification,
) -> Result<()> {
    if notification.method != super::protocol::METHOD_SESSION_UPDATE {
        return Ok(());
    }
    let params: SessionUpdateParams = match serde_json::from_value(notification.params) {
        Ok(params) => params,
        Err(error) => {
            log::warn!("Ignoring unparseable Qwen session update: {error}");
            return Ok(());
        }
    };
    let events = mapper.map_update(params.update);
    apply_turn_events(foreground, conversation_id, mapper, current, events).await
}

async fn apply_turn_events(
    foreground: &ModelSpawner<BlocklistAIController>,
    conversation_id: AIConversationId,
    mapper: &mut AcpTurnMapper,
    current: &mut StartedTurn,
    events: Vec<TurnEvent>,
) -> Result<()> {
    for event in events {
        match event {
            TurnEvent::Actions(actions) => {
                apply_actions(foreground, &current.turn.stream_id, actions).await?;
            }
            TurnEvent::SegmentBoundary => {
                finish_stream(
                    foreground,
                    current.turn.stream_id.clone(),
                    stream_finished_for(StopReason::EndTurn),
                )
                .await?;
                let next = begin_turn(foreground, conversation_id)
                    .await
                    .map_err(|error| anyhow!("{error}"))?;
                mapper.start_segment(next.turn.request_id.clone());
                *current = next;
            }
        }
    }
    Ok(())
}

async fn begin_turn(
    foreground: &ModelSpawner<BlocklistAIController>,
    conversation_id: AIConversationId,
) -> Result<StartedTurn, String> {
    foreground
        .spawn(move |controller, ctx| {
            let task_id = BlocklistAIHistoryModel::handle(ctx)
                .as_ref(ctx)
                .conversation(&conversation_id)
                .map(|conversation| conversation.get_root_task_id().to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            controller
                .begin_external_harness_turn(conversation_id, None, ctx)
                .map(|turn| StartedTurn { turn, task_id })
                .map_err(|error: UpdateHistoryError| error.to_string())
        })
        .await
        .map_err(|_| "the agent view closed before Qwen Code could start".to_owned())?
}

async fn apply_actions(
    foreground: &ModelSpawner<BlocklistAIController>,
    stream_id: &crate::ai::blocklist::ResponseStreamId,
    actions: Vec<ai_terminal_multi_agent_api::ClientAction>,
) -> Result<()> {
    if actions.is_empty() {
        return Ok(());
    }
    let stream_id = stream_id.clone();
    foreground
        .spawn(move |controller, ctx| {
            controller.apply_external_harness_event(
                &stream_id,
                ResponseEvent {
                    r#type: Some(response_event::Type::ClientActions(
                        response_event::ClientActions { actions },
                    )),
                },
                ctx,
            );
        })
        .await
        .map_err(|_| anyhow!("the agent view closed"))
}

async fn finish_stream(
    foreground: &ModelSpawner<BlocklistAIController>,
    stream_id: crate::ai::blocklist::ResponseStreamId,
    finished: StreamFinished,
) -> Result<()> {
    foreground
        .spawn(move |controller, ctx| {
            controller.apply_external_harness_event(
                &stream_id,
                ResponseEvent {
                    r#type: Some(response_event::Type::Finished(finished)),
                },
                ctx,
            );
        })
        .await
        .map_err(|_| anyhow!("the agent view closed"))
}

fn failed_stream(message: &str) -> StreamFinished {
    StreamFinished {
        reason: Some(stream_finished::Reason::InternalError(
            stream_finished::InternalError {
                message: message.to_owned(),
            },
        )),
        ..Default::default()
    }
}

async fn report_turn_failure(
    foreground: &ModelSpawner<BlocklistAIController>,
    conversation_id: AIConversationId,
    prompt: &str,
    message: &str,
) -> Result<(), ()> {
    let started = begin_turn(foreground, conversation_id)
        .await
        .map_err(|_| ())?;
    let mapper = AcpTurnMapper::new(started.task_id, started.turn.request_id.clone());
    if apply_actions(
        foreground,
        &started.turn.stream_id,
        mapper.initial_actions(prompt),
    )
    .await
    .is_err()
    {
        return Err(());
    }
    finish_stream(foreground, started.turn.stream_id, failed_stream(message))
        .await
        .map_err(|_| ())
}

async fn answer_agent_request(
    client: &LocalClient,
    method: &str,
    params: Value,
) -> Result<Value, RpcError> {
    match method {
        super::protocol::METHOD_REQUEST_PERMISSION => answer_permission(client, params).await,
        super::protocol::METHOD_FS_READ_TEXT_FILE => {
            super::blocking(move || read_text_file(params)).await
        }
        super::protocol::METHOD_FS_WRITE_TEXT_FILE => write_text_file(client, params).await,
        METHOD_TERMINAL_CREATE => create_terminal(client, params).await,
        METHOD_TERMINAL_OUTPUT => terminal_output(client, params),
        METHOD_TERMINAL_WAIT => wait_for_terminal(client, params).await,
        METHOD_TERMINAL_KILL => kill_terminal(client, params),
        METHOD_TERMINAL_RELEASE => {
            release_terminal(client, params);
            Ok(Value::Null)
        }
        other => Err(RpcError::method_not_found(other)),
    }
}

async fn answer_permission(client: &LocalClient, params: Value) -> Result<Value, RpcError> {
    let params: RequestPermissionParams =
        serde_json::from_value(params).map_err(RpcError::internal)?;
    let decision = match permission_for_tool_kind(params.tool_call.kind) {
        LocalToolPermission::Deny { reason } => PolicyDecision::Deny {
            reason: reason.to_owned(),
        },
        LocalToolPermission::Allow => match PolicyRequest::for_tool_call(&params.tool_call) {
            Some(request) => evaluate_policy(client, request).await?,
            None => PolicyDecision::Allow,
        },
    };
    let outcome = permission_outcome(&params.options, &decision);
    serde_json::to_value(RequestPermissionResponse { outcome }).map_err(RpcError::internal)
}

fn permission_outcome(
    options: &[PermissionOption],
    decision: &PolicyDecision,
) -> PermissionOutcome {
    use super::protocol::PermissionOptionKind;
    let preferred = match decision {
        PolicyDecision::Allow => [
            PermissionOptionKind::AllowAlways,
            PermissionOptionKind::AllowOnce,
        ],
        PolicyDecision::Deny { .. } => [
            PermissionOptionKind::RejectOnce,
            PermissionOptionKind::RejectAlways,
        ],
    };
    let chosen = preferred.into_iter().find_map(|kind| {
        options
            .iter()
            .find(|option| option.kind == kind)
            .map(|option| option.option_id.clone())
    });
    match (decision, chosen) {
        (PolicyDecision::Allow, Some(option_id))
        | (PolicyDecision::Deny { .. }, Some(option_id)) => {
            PermissionOutcome::Selected { option_id }
        }
        (PolicyDecision::Allow, None) => match options.first() {
            Some(option) => PermissionOutcome::Selected {
                option_id: option.option_id.clone(),
            },
            None => PermissionOutcome::Cancelled,
        },
        (PolicyDecision::Deny { .. }, None) => PermissionOutcome::Cancelled,
    }
}

async fn evaluate_policy(
    client: &LocalClient,
    request: PolicyRequest,
) -> Result<PolicyDecision, RpcError> {
    let terminal_surface_id = client.terminal_surface_id;
    let escape_char = client.escape_char;
    client
        .foreground
        .spawn(move |controller, ctx| {
            request.evaluate(controller, terminal_surface_id, escape_char, ctx)
        })
        .await
        .map_err(|_| RpcError::internal("the agent view closed"))
}

async fn write_text_file(client: &LocalClient, params: Value) -> Result<Value, RpcError> {
    let params: super::protocol::WriteTextFileParams =
        serde_json::from_value(params).map_err(RpcError::internal)?;
    if let PolicyDecision::Deny { reason } =
        evaluate_policy(client, PolicyRequest::write(&params.path)).await?
    {
        return Err(RpcError::internal(format!(
            "writing {} is not permitted: {reason}",
            params.path
        )));
    }
    super::blocking(move || {
        if let Some(parent) = Path::new(&params.path).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(RpcError::internal)?;
        }
        std::fs::write(&params.path, params.content)
            .with_context(|| format!("writing {}", params.path))
            .map_err(RpcError::internal)?;
        Ok(Value::Null)
    })
    .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTerminalParams {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    env: Vec<TerminalEnv>,
    #[serde(default)]
    output_byte_limit: Option<u64>,
}

#[derive(Deserialize)]
struct TerminalEnv {
    name: String,
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TerminalIdParams {
    terminal_id: String,
}

async fn create_terminal(client: &LocalClient, params: Value) -> Result<Value, RpcError> {
    let params: CreateTerminalParams =
        serde_json::from_value(params).map_err(RpcError::internal)?;
    if params.command.is_empty() {
        return Err(RpcError::internal("terminal command is empty"));
    }
    let command_line = command_line(&params.command, &params.args);
    if let PolicyDecision::Deny { reason } = evaluate_policy(
        client,
        PolicyRequest::Execute {
            command: command_line,
        },
    )
    .await?
    {
        return Err(RpcError::internal(format!(
            "the command was not permitted: {reason}"
        )));
    }
    let cwd = params.cwd.map(PathBuf::from).or_else(|| {
        let cwd = client.cwd.lock().clone();
        (!cwd.as_os_str().is_empty()).then_some(cwd)
    });
    let limit = params
        .output_byte_limit
        .map(|limit| usize::try_from(limit).unwrap_or(DEFAULT_TERMINAL_OUTPUT_LIMIT))
        .unwrap_or(DEFAULT_TERMINAL_OUTPUT_LIMIT);
    let terminal = start_local_terminal(
        &params.command,
        &params.args,
        cwd.as_deref(),
        &params.env,
        limit,
    )
    .map_err(RpcError::internal)?;
    let id = terminal.id.clone();
    client.terminals.lock().insert(id.clone(), terminal);
    Ok(json!({ "terminalId": id }))
}

fn terminal_output(client: &LocalClient, params: Value) -> Result<Value, RpcError> {
    let params: TerminalIdParams = serde_json::from_value(params).map_err(RpcError::internal)?;
    let terminals = client.terminals.lock();
    let Some(terminal) = terminals.get(&params.terminal_id) else {
        return Err(RpcError::internal("unknown terminal"));
    };
    let (output, truncated, exit_code) = terminal.snapshot();
    let mut response = json!({
        "output": output,
        "truncated": truncated,
    });
    if let Some(exit_code) = exit_code {
        response["exitStatus"] = json!({ "exitCode": exit_code });
    }
    Ok(response)
}

async fn wait_for_terminal(client: &LocalClient, params: Value) -> Result<Value, RpcError> {
    let params: TerminalIdParams = serde_json::from_value(params).map_err(RpcError::internal)?;
    let terminal = {
        let terminals = client.terminals.lock();
        terminals
            .get(&params.terminal_id)
            .cloned()
            .ok_or_else(|| RpcError::internal("unknown terminal"))?
    };
    let exit_code = super::blocking(move || {
        Ok(match terminal.wait_for_exit() {
            Some(code) => Value::from(code),
            None => Value::Null,
        })
    })
    .await?;
    Ok(json!({ "exitCode": exit_code, "signal": Value::Null }))
}

fn kill_terminal(client: &LocalClient, params: Value) -> Result<Value, RpcError> {
    let params: TerminalIdParams = serde_json::from_value(params).map_err(RpcError::internal)?;
    let terminals = client.terminals.lock();
    let Some(terminal) = terminals.get(&params.terminal_id) else {
        return Err(RpcError::internal("unknown terminal"));
    };
    terminal.kill();
    Ok(Value::Null)
}

fn release_terminal(client: &LocalClient, params: Value) {
    let Ok(params) = serde_json::from_value::<TerminalIdParams>(params) else {
        return;
    };
    if let Some(terminal) = client.terminals.lock().remove(&params.terminal_id) {
        terminal.kill();
    }
}

fn command_line(program: &str, args: &[String]) -> String {
    let mut line = String::from(program);
    for arg in args {
        line.push(' ');
        line.push_str(arg);
    }
    line
}

struct LocalTerminal {
    id: String,
    output: Arc<Mutex<String>>,
    truncated: Arc<AtomicBool>,
    child: Arc<Mutex<std::process::Child>>,
}

impl Clone for LocalTerminal {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            output: self.output.clone(),
            truncated: self.truncated.clone(),
            child: self.child.clone(),
        }
    }
}

impl Drop for LocalTerminal {
    fn drop(&mut self) {
        // The map and the wait path can both hold a clone. Kill only when the last clone drops
        // so a snapshot cannot reap a command that is still being read.
        if Arc::strong_count(&self.child) == 1 {
            self.kill();
        }
    }
}

static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);

fn start_local_terminal(
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
    env: &[TerminalEnv],
    output_limit: usize,
) -> Result<LocalTerminal, String> {
    let mut command = command::blocking::Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    for variable in env {
        command.env(&variable.name, &variable.value);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to start '{program}': {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("'{program}' has no stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("'{program}' has no stderr"))?;
    let output = Arc::new(Mutex::new(String::new()));
    let truncated = Arc::new(AtomicBool::new(false));
    spawn_output_reader(stdout, output.clone(), truncated.clone(), output_limit);
    spawn_output_reader(stderr, output.clone(), truncated.clone(), output_limit);
    let id = NEXT_TERMINAL_ID.fetch_add(1, Ordering::Relaxed).to_string();
    Ok(LocalTerminal {
        id,
        output,
        truncated,
        child: Arc::new(Mutex::new(child)),
    })
}

fn spawn_output_reader(
    mut reader: impl Read + Send + 'static,
    output: Arc<Mutex<String>>,
    truncated: Arc<AtomicBool>,
    limit: usize,
) {
    std::thread::Builder::new()
        .name("qwen-terminal".into())
        .spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                if truncated.load(Ordering::Relaxed) {
                    break;
                }
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => {
                        let mut captured = output.lock();
                        let remaining = limit.saturating_sub(captured.len());
                        if remaining == 0 {
                            truncated.store(true, Ordering::Relaxed);
                            break;
                        }
                        let take = count.min(remaining);
                        captured.push_str(&String::from_utf8_lossy(&buffer[..take]));
                        if take < count {
                            truncated.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                }
            }
        })
        .ok();
}

impl LocalTerminal {
    fn snapshot(&self) -> (String, bool, Option<i32>) {
        let exit_code = self
            .child
            .lock()
            .try_wait()
            .ok()
            .flatten()
            .and_then(|status| status.code());
        (
            self.output.lock().clone(),
            self.truncated.load(Ordering::Relaxed),
            exit_code,
        )
    }

    fn wait_for_exit(&self) -> Option<i32> {
        // Poll so `kill` can take the child lock. `Child::wait` would hold it until exit.
        loop {
            {
                let mut child = self.child.lock();
                match child.try_wait() {
                    Ok(Some(status)) => return status.code(),
                    Ok(None) => {}
                    Err(_) => return None,
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn kill(&self) {
        let mut child = self.child.lock();
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
#[path = "local_gui_tests.rs"]
mod tests;

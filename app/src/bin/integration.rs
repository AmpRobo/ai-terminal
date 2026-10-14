use ai_terminal_cli::WorkerCommand;
use ai_terminal_core::AppId;
use ai_terminal_core::channel::{
    AiTerminalServerConfig, Channel, ChannelConfig, ChannelState, OzConfig,
};
use anyhow::Result;
use clap::Parser;

#[derive(Debug, Default, Parser, Clone)]
#[command(name = "ai-terminal-integration")]
#[clap(args_conflicts_with_subcommands = true)]
pub struct Args {
    #[command(subcommand)]
    command: Option<WorkerCommand>,
}

pub fn main() -> Result<()> {
    ChannelState::set(ChannelState::new(
        Channel::Integration,
        ChannelConfig {
            app_id: AppId::new(
                "dev",
                "aiterminal",
                if cfg!(target_os = "macos") {
                    "AiTerminal-Integration"
                } else {
                    "AiTerminalIntegration"
                },
            ),
            logfile_name: "ai_terminal_integration.log".into(),
            server_config: AiTerminalServerConfig {
                firebase_auth_api_key: "".into(),
                // Use an IP in the IANA testing range, with the TCP discard port, to
                // black-hole server traffic.
                server_root_url: "http://192.0.2.0:9".into(),
                rtc_server_url: "ws://192.0.2.0:9/graphql/v2".into(),
                session_sharing_server_url: None,
                iap_config: None,
            },
            oz_config: OzConfig {
                // Use an IP in the IANA testing range, with the TCP discard port, to
                // black-hole server traffic.
                oz_root_url: "http://192.0.2.0:9".into(),
                workload_audience_url: None,
            },
            telemetry_config: None,
            crash_reporting_config: None,
            autoupdate_config: None,
            mcp_static_config: None,
        },
    ));

    let args = Args::parse();

    if let Some(command) = &args.command {
        match command {
            #[cfg(unix)]
            WorkerCommand::TerminalServer(args) => {
                // If we were asked to run as a terminal server (as opposed to the main
                // GUI application), do so.  This must occur before init_logging, as the
                // terminal server sets up its own logger, and attempting to set a second
                // logger leads to a panic.
                ai_terminal::terminal::local_tty::run_terminal_server(args);
                return Ok(());
            }
            #[cfg(not(target_family = "wasm"))]
            WorkerCommand::RemoteServerProxy(_) | WorkerCommand::RemoteServerDaemon(_) => {
                return ai_terminal::run();
            }
            #[allow(unreachable_patterns)]
            other => panic!("Worker not supported in integration tests: {other:?}"),
        }
    }

    ai_terminal::run()
}

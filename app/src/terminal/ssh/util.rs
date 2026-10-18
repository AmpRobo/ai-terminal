use std::net::IpAddr;
use std::path::Path;

use lazy_static::lazy_static;
use regex::Regex;

/// Converts a multiline bash or zsh script to one line by turning newlines into semicolons or
/// deleting them, as appropriate.
///
/// Extra semicolons are a syntax error in bash, so this is careful to avoid adding them except
/// where necessary.
///
/// This function exists because there's a strange macOS ssh server bug where sending a lot of data
/// containing newlines to a shell results in data corruption.
pub fn convert_script_to_one_line(script: &str) -> String {
    lazy_static! {
        static ref EXTRA_SPACES_REGEX: Regex = Regex::new(r"\n+\s*").expect("invalid regex");
        static ref NO_SEMICOLON_REGEX: Regex =
            Regex::new("(; ?|\\{|do|then|else|in)\n").expect("invalid regex");
        static ref REMOVE_COMMENTS_REGEX: Regex = Regex::new(r"(?m)^ *#.*").expect("invalid regex");
        static ref REMOVE_LEADING_NEWLINES: Regex = Regex::new(r"^\n*").expect("invalid regex");
    };
    let script = REMOVE_COMMENTS_REGEX.replace_all(script, "");
    let script = REMOVE_LEADING_NEWLINES.replace_all(&script, "");
    let script = EXTRA_SPACES_REGEX.replace_all(&script, "\n");
    let script = NO_SEMICOLON_REGEX.replace_all(&script, "$1 ");
    let mut script = script.replace('\n', ";");
    script.push('\n');
    script
}

pub enum SshLoginState {
    LastLogin,
    NonSshOutput,
    Authenticating,
    PromptDetected,
}

/// Reads the contents of the output grid to determine SSH login state. Returns [SshLoginState::LastLogin] if
/// "Last login:" is detected in the output. Returns [SshLoginState::NonSshOutput] if certain keywords
/// known to be a part of ssh login prompts are found in the current last line of command output. The
/// "password" and "Password" are for password authentication. "passphrase" is intended to cover authentication
/// by public key. And "yes/no" relates to trust-on-first-use prompts for host-based authentication.
pub fn check_ssh_login_state(block_output: &str) -> SshLoginState {
    lazy_static! {
        // Common final prompt characters followed by a space.
        static ref PROMPT_REGEX: Regex = Regex::new(r"[$#%>❯│⟫»▶λ→] $").expect("invalid regex");
    };

    let mut last_line = None;

    for line in block_output.lines() {
        if line.starts_with("Last login:") {
            return SshLoginState::LastLogin;
        }
        // With an iterator, there's no way to know if it's the last element so
        // we overwrite last_line at each iteration.
        last_line = Some(line);
    }

    last_line.map_or(SshLoginState::Authenticating, |line| {
        if line.contains("password")
            || line.contains("Password")
            || line.contains("passphrase")
            || line.contains("yes/no")
            || line.contains("Please type")
            || line.contains("'yes'")
            || line.contains("Confirm user presence")
            || line.starts_with("Enter ")
            || line.starts_with("Allow ")
        {
            SshLoginState::Authenticating
        } else if PROMPT_REGEX.is_match(line) {
            SshLoginState::PromptDetected
        } else {
            SshLoginState::NonSshOutput
        }
    })
}

/// Represents the parsed components of an interactive SSH command.
/// For some [`SshAiTerminalifyCommand`]s, we do not support parsing
/// a host or port In these cases, we can still parse to a valid
/// empty `InteractiveSshCommand` to indicate that we did
/// successfully detect an interactive SSH command.
#[derive(Clone, Debug, Default)]
pub struct InteractiveSshCommand {
    pub host: Option<String>,
    pub port: Option<String>,
}

impl InteractiveSshCommand {
    /// Parses ssh commands of the form `ssh ...`.
    /// Only returns an `InteractiveSshCommand` if we determine the command is interactive.
    fn parse_ssh_command(command: &str) -> Option<InteractiveSshCommand> {
        let command = if let Some(suffix) = command.strip_prefix("command ") {
            suffix
        } else {
            command
        };
        let tokens = parse_ssh_command_tokens(command)?;
        let invocation = parse_ssh_invocation(&tokens)?;
        let host = invocation.host_index.map(|index| tokens[index].clone());
        Some(InteractiveSshCommand {
            host,
            port: invocation.port,
        })
    }
}

pub enum SshLikeCommand {
    Gcloud,
    ElasticBeanstalk,
    DigitalOceanDroplet,
}

/// Represents the different kinds of commands we recognize as starting an interactive SSH
/// session. `Ssh` means a literal `ssh` command, where all other commands (e.g. `gcloud
/// compute ssh`) are categorized as SSH-like commands.
pub enum SshAiTerminalifyCommand {
    Ssh,
    SshLike(SshLikeCommand),
}

impl SshAiTerminalifyCommand {
    /// Not a literal `ssh` command, but another command that starts an interactive SSH
    /// session.
    pub fn is_ssh_like_command(&self) -> bool {
        matches!(self, SshAiTerminalifyCommand::SshLike(_))
    }
}

lazy_static! {
    static ref INTERACTIVE_SSH: Regex = Regex::new(r"^ssh\s+").expect("interactive SSH regex invalid");

    /// Matches "gcloud compute ssh" for connecting to GCP VMs.
    static ref GCLOUD_REGEX: Regex = Regex::new(r"^gcloud\s+compute\s+ssh\s.+").expect("gcloud SSH regex invalid");

    /// Matches "eb ssh" for connecting to AWS Elastic Beanstalk VMs.
    static ref ELASTIC_BEANSTALK_REGEX: Regex = Regex::new(r"^eb\s+ssh\s.+").expect("elastic beanstalk SSH regex invalid");

    /// Matches "doctl compute ssh" for connecting to a digital ocean droplet.
    static ref DIGITAL_OCEAN_DROPLET_REGEX: Regex = Regex::new(r"^doctl\s+compute\s+ssh\s.+").expect("digital ocean SSH regex invalid");
}

impl SshAiTerminalifyCommand {
    pub fn matches(command: &str) -> Option<SshAiTerminalifyCommand> {
        let command = if let Some(suffix) = command.strip_prefix("command ") {
            suffix
        } else {
            command
        };
        if INTERACTIVE_SSH.is_match(command) {
            Some(SshAiTerminalifyCommand::Ssh)
        } else if GCLOUD_REGEX.is_match(command) {
            Some(SshAiTerminalifyCommand::SshLike(SshLikeCommand::Gcloud))
        } else if ELASTIC_BEANSTALK_REGEX.is_match(command) {
            Some(SshAiTerminalifyCommand::SshLike(
                SshLikeCommand::ElasticBeanstalk,
            ))
        } else if DIGITAL_OCEAN_DROPLET_REGEX.is_match(command) {
            Some(SshAiTerminalifyCommand::SshLike(
                SshLikeCommand::DigitalOceanDroplet,
            ))
        } else {
            None
        }
    }
}

pub fn parse_interactive_ssh_command(command: &str) -> Option<InteractiveSshCommand> {
    match SshAiTerminalifyCommand::matches(command) {
        Some(SshAiTerminalifyCommand::Ssh) => InteractiveSshCommand::parse_ssh_command(command),
        Some(SshAiTerminalifyCommand::SshLike(SshLikeCommand::Gcloud)) => {
            Some(InteractiveSshCommand::default())
        }
        Some(SshAiTerminalifyCommand::SshLike(SshLikeCommand::ElasticBeanstalk)) => {
            Some(InteractiveSshCommand::default())
        }
        Some(SshAiTerminalifyCommand::SshLike(SshLikeCommand::DigitalOceanDroplet)) => {
            Some(InteractiveSshCommand::default())
        }
        None => None,
    }
}

fn parse_ssh_command_tokens(command: &str) -> Option<Vec<String>> {
    let Ok(tokens) = shell_words::split(command) else {
        return None;
    };

    // Cases: "", "ls", "ssh-add-key"
    if tokens.is_empty() || tokens[0] != "ssh" {
        return None;
    }
    Some(tokens)
}

struct SshInvocation {
    host_index: Option<usize>,
    port: Option<String>,
    uses_jump_host: bool,
}

fn parse_ssh_invocation(tokens: &[String]) -> Option<SshInvocation> {
    let mut host_index = None;
    let mut port = None;
    let mut uses_jump_host = false;
    let mut i = 1;
    while i < tokens.len() {
        let arg = tokens[i].as_str();
        if ssh_arg_uses_jump_host(arg, tokens.get(i + 1).map(String::as_str)) {
            uses_jump_host = true;
        }
        match arg {
            // -T or -W imply a non-interactive session.
            "-T" | "-W" => return None,

            "-p" => {
                i += 1;
                if i < tokens.len() {
                    port = Some(tokens[i].clone());
                } else {
                    return None;
                }
            }

            // SSH option that doesn't change interactivity and require an argument: Skip the next item.
            "-B" | "-b" | "-c" | "-D" | "-E" | "-e" | "-F" | "-I" | "-i" | "-J" | "-L" | "-l"
            | "-m" | "-O" | "-o" | "-P" | "-Q" | "-R" | "-S" | "-w" => {
                i += 1;
            }

            // SSH option(s) that don't change interactivity.
            arg if arg.starts_with('-') => {}

            // Otherwise, it's a positional argument (e.g., hostname, command to run).
            // A second positional is a remote command, which we must not append to.
            _ => {
                if host_index.is_some() {
                    return None;
                }
                host_index = Some(i);
            }
        }
        i += 1;
    }

    Some(SshInvocation {
        host_index,
        port,
        uses_jump_host,
    })
}

fn ssh_arg_uses_jump_host(arg: &str, next: Option<&str>) -> bool {
    let lower = arg.to_ascii_lowercase();
    if lower == "-j" || lower.contains("proxyjump") || lower.contains("proxycommand") {
        return true;
    }
    matches!(arg, "-o" | "-J")
        && next.is_some_and(|value| {
            let value = value.to_ascii_lowercase();
            value.starts_with("proxyjump") || value.starts_with("proxycommand")
        })
}

/// `true` when `command` reaches the destination through a jump host.
///
/// The local TCP peer of that session is the jump host, not the destination.
pub fn ssh_uses_jump_host(command: &str) -> bool {
    let command = if let Some(suffix) = command.strip_prefix("command ") {
        suffix
    } else {
        command
    };
    parse_ssh_command_tokens(command)
        .and_then(|tokens| parse_ssh_invocation(&tokens))
        .is_some_and(|invocation| invocation.uses_jump_host)
}

/// Creates an sftp command that copies a given local file into the pwd in the ai_terminalified ssh session.
pub fn transfer_file_sftp_command(
    local_file_path: String,
    ssh_host: String,
    ssh_port: Option<String>,
    pwd: Option<String>,
) -> Option<String> {
    // "sftp "
    let mut command = String::from("sftp ");

    // "sftp -P 2222"
    if let Some(port) = ssh_port {
        command += &format!("-P {port} ");
    }

    // "sftp -P 2222 sshuser@127.0.0.1 <<< "put "
    command += &ssh_host;
    command += " <<< \"put ";

    // "sftp -P 2222 sshuser@127.0.0.1 <<< "put -r"
    let is_dir = Path::new(&local_file_path)
        .metadata()
        .is_ok_and(|m| m.is_dir());
    if is_dir {
        command += "-r "
    }

    // "sftp -P 2222 sshuser@127.0.0.1 <<< "put -r \"path/to/local/file\""
    command += &format!("\\\"{}\\\"", &local_file_path);

    // "sftp -P 2222 sshuser@127.0.0.1 <<< "put -r path/to/local/file pwd/on/remote"
    if let Some(pwd) = pwd {
        command += " ";
        command += &format!("\\\"{}\\\"", &pwd);
    }

    // "sftp -P 2222 sshuser@127.0.0.1 <<< "put -r path/to/local/file pwd/on/remote""
    command += "\"";

    Some(command)
}

/// Remote command that starts a login shell after `cd` into `directory`.
fn remote_login_shell(directory: &str) -> String {
    format!("cd {} && exec \"$SHELL\" -l", shell_words::quote(directory))
}

/// Host part of an `ssh` destination (`user@host`, `[ipv6]`, or a bare host).
pub fn destination_hostname(destination: &str) -> &str {
    let host = match destination.rsplit_once('@') {
        Some((user, host)) if !user.is_empty() && !host.is_empty() => host,
        _ => destination,
    };
    host.strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(host)
}

/// `true` when `host` is an IP address, including a `user@ip` destination.
pub fn hostname_is_ip(host: &str) -> bool {
    let host = destination_hostname(host);
    !host.is_empty() && host.parse::<IpAddr>().is_ok()
}

fn bracket_ipv6(host: &str) -> String {
    if host.contains(':') && !(host.starts_with('[') && host.ends_with(']')) {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

fn token_is_host_key_alias(token: &str) -> bool {
    let token = token.to_ascii_lowercase();
    token == "hostkeyalias" || token.starts_with("hostkeyalias=")
}

fn rewrite_destination_token(token: &str, new_host: &str) -> String {
    let new_host = bracket_ipv6(new_host);
    match token.rsplit_once('@') {
        Some((user, host)) if !user.is_empty() && !host.is_empty() => {
            format!("{user}@{new_host}")
        }
        _ => new_host,
    }
}

/// Rewrites a literal `ssh` command so the remote shell starts in `directory`.
///
/// Returns `None` when `spawning_command` is not an interactive `ssh` we can
/// append a remote command to (for example `gcloud compute ssh`, or an `ssh`
/// that already has a remote command).
pub fn ssh_command_starting_in_directory(
    spawning_command: &str,
    directory: &str,
) -> Option<String> {
    ssh_command_starting_in_directory_with_host(spawning_command, directory, None)
}

/// Like [`ssh_command_starting_in_directory`], replacing the destination
/// hostname with `host_override` when that address should be dialed directly.
///
/// `host_override` is ignored for jump-host commands, because the local TCP
/// peer is the jump host, and when the destination is already an IP address.
pub fn ssh_command_starting_in_directory_with_host(
    spawning_command: &str,
    directory: &str,
    host_override: Option<&str>,
) -> Option<String> {
    let command = spawning_command
        .strip_prefix("command ")
        .unwrap_or(spawning_command)
        .trim();
    let tokens = parse_ssh_command_tokens(command)?;
    let invocation = parse_ssh_invocation(&tokens)?;
    let host_index = invocation.host_index?;
    if tokens[host_index].is_empty() {
        return None;
    }
    let replace_host = host_override
        .map(str::trim)
        .filter(|override_host| !override_host.is_empty())
        .filter(|_| !hostname_is_ip(&tokens[host_index]) && !invocation.uses_jump_host);
    let command = if let Some(override_host) = replace_host {
        let mut tokens = tokens;
        // The known_hosts entry is for the name the user dialed. HostKeyAlias
        // keeps that check when the dialed address is the connection's IP.
        let alias = destination_hostname(&tokens[host_index]).to_string();
        tokens[host_index] = rewrite_destination_token(&tokens[host_index], override_host);
        if !alias.is_empty() && !tokens.iter().any(|token| token_is_host_key_alias(token)) {
            tokens.insert(host_index, format!("HostKeyAlias={alias}"));
            tokens.insert(host_index, "-o".to_string());
        }
        shell_words::join(tokens)
    } else {
        command.to_string()
    };
    let remote = remote_login_shell(directory);
    Some(format!("{command} -t {}", shell_words::quote(&remote)))
}

/// Builds an `ssh -t` command that starts a login shell in `directory`.
pub fn ssh_command_for_endpoint(
    user: &str,
    host: &str,
    port: Option<&str>,
    directory: &str,
) -> String {
    let remote = remote_login_shell(directory);
    let mut command = String::from("ssh");
    if let Some(port) = port.filter(|port| !port.is_empty()) {
        command.push_str(" -p ");
        command.push_str(&shell_words::quote(port));
    }
    command.push(' ');
    let host = bracket_ipv6(host);
    let destination = if user.is_empty() {
        host
    } else {
        format!("{user}@{host}")
    };
    command.push_str(&shell_words::quote(&destination));
    command.push_str(" -t ");
    command.push_str(&shell_words::quote(&remote));
    command
}

#[cfg(test)]
#[path = "util_tests.rs"]
mod tests;

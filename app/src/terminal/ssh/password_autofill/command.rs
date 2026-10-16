use std::path::Path;

use ai_terminal_core::command::ExitCode;

use super::SshCredentialKey;
use super::prompt::PasswordPrompt;

const SSH_OPTIONS_WITH_ARGUMENT: &str = "BbcDEeFIiJLlmOoPpQRSWw";
const SCP_OPTIONS_WITH_ARGUMENT: &str = "cDFiJloPSX";
const SFTP_OPTIONS_WITH_ARGUMENT: &str = "BbcDFiJloPRSsX";

/// The programs that log in through the OpenSSH client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SshProgram {
    Ssh,
    Scp,
    Sftp,
    Rsync,
}

/// The SSH destination of a command line, as far as it can be told without reading the SSH
/// config.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SshLoginTarget {
    program: SshProgram,
    user: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    /// Whether the login may pass through other hosts (e.g. with `-J`), whose password prompts
    /// can't be told apart from the destination's unless they name their account.
    may_relay: bool,
}

impl SshLoginTarget {
    /// Parses an `ssh`, `scp`, `sftp`, or `rsync` command line, or returns `None` for any other
    /// command.
    pub fn parse(command: &str) -> Option<Self> {
        let words = shell_words::split(command).ok()?;
        let mut words = words
            .into_iter()
            .skip_while(|word| is_env_assignment(word))
            .skip_while(|word| word == "command");
        let program = match Path::new(&words.next()?).file_name()?.to_str()? {
            "ssh" => SshProgram::Ssh,
            "scp" => SshProgram::Scp,
            "sftp" => SshProgram::Sftp,
            "rsync" => SshProgram::Rsync,
            _ => return None,
        };
        let args: Vec<String> = words.collect();
        Some(match program {
            SshProgram::Ssh => Self::parse_ssh_args(&args),
            SshProgram::Scp => {
                let (options, operands) = split_options(&args, SCP_OPTIONS_WITH_ARGUMENT);
                let remotes = operands
                    .iter()
                    .filter_map(|operand| parse_remote_operand(operand, "scp"))
                    .collect();
                Self::file_transfer_target(program, options, remotes)
            }
            SshProgram::Sftp => {
                let (options, operands) = split_options(&args, SFTP_OPTIONS_WITH_ARGUMENT);
                // sftp's destination is remote even without a `:path` suffix.
                let remotes = operands
                    .first()
                    .map(|operand| {
                        parse_remote_operand(operand, "sftp")
                            .unwrap_or_else(|| Remote::from_user_host(operand, None))
                    })
                    .into_iter()
                    .collect();
                Self::file_transfer_target(program, options, remotes)
            }
            // rsync passes its remote shell command through `-e`, so only prompts that name their
            // account can be attributed.
            SshProgram::Rsync => Self {
                program,
                user: None,
                host: None,
                port: None,
                may_relay: true,
            },
        })
    }

    fn parse_ssh_args(args: &[String]) -> Self {
        let (options, operands) = split_options(args, SSH_OPTIONS_WITH_ARGUMENT);
        let mut login = LoginOptions::default();
        login.apply_ssh_options(&options.values);
        let destination = operands.first().and_then(|operand| {
            if operand.contains("://") {
                parse_uri(operand, "ssh")
            } else {
                Some(Remote::from_user_host(operand, None))
            }
        });
        if let Some(destination) = &destination {
            login.set_user(destination.user.clone());
            login.set_port(destination.port);
        }
        // ssh resumes option parsing after the destination, e.g. `ssh host -p 2222`.
        if let Some(rest) = operands.get(1..)
            && !options.terminated
        {
            login.apply_ssh_options(&split_options(rest, SSH_OPTIONS_WITH_ARGUMENT).0.values);
        }

        Self {
            program: SshProgram::Ssh,
            user: login.user,
            host: destination.map(|destination| destination.host),
            port: login.port,
            may_relay: login.may_relay,
        }
    }

    /// Builds the target of an scp or sftp command line, whose options mostly match.
    fn file_transfer_target(
        program: SshProgram,
        options: ParsedOptions,
        mut remotes: Vec<Remote>,
    ) -> Self {
        // These programs pass their `-o` options to ssh ahead of the destination's user and port,
        // so those options take precedence.
        let mut login = LoginOptions::default();
        let mut port = None;
        for (flag, value) in options.values {
            match flag {
                'P' => port = parse_port(value).or(port),
                'o' => login.apply_config(value),
                'J' => login.apply_jump(value),
                _ => {}
            }
        }

        remotes.dedup();
        let remote = match <[Remote; 1]>::try_from(remotes) {
            Ok([remote]) => Some(remote),
            // A copy between two remote hosts logs in to both.
            Err(_) => {
                login.may_relay = true;
                None
            }
        };
        let mut host = None;
        if let Some(remote) = remote {
            login.set_user(remote.user);
            // A port in an `scp://` or `sftp://` URI overrides `-P`.
            login.set_port(remote.port.or(port));
            host = Some(remote.host);
        }

        Self {
            program,
            user: login.user,
            host,
            port: login.port,
            may_relay: login.may_relay,
        }
    }

    /// Returns the account that a password typed at `prompt` belongs to, or `None` if it can't be
    /// told which host is asking.
    pub(super) fn credential_key(&self, prompt: &PasswordPrompt) -> Option<SshCredentialKey> {
        match &prompt.account {
            Some(account) => {
                let is_destination = self
                    .host
                    .as_deref()
                    .is_some_and(|host| hosts_match(host, &account.host));
                // The prompt shows the resolved host name, which differs from the command's host
                // for SSH config aliases, so the command's port only belongs to it if the login
                // doesn't pass through other hosts.
                let port = if is_destination || !self.may_relay {
                    self.port
                } else {
                    None
                };
                Some(SshCredentialKey::new(
                    Some(account.user.clone()),
                    &account.host,
                    port,
                ))
            }
            None if self.may_relay => None,
            None => Some(SshCredentialKey::new(
                self.user.clone(),
                self.host.as_deref()?,
                self.port,
            )),
        }
    }

    /// Whether a command that exited with `exit_code` must have logged in successfully.
    pub(super) fn exit_code_proves_login(&self, exit_code: ExitCode) -> bool {
        match self.program {
            // ssh exits with 255 when the connection or login fails and otherwise with the
            // remote command's status. Statuses above 128 may come from a signal sent to ssh.
            SshProgram::Ssh => (0..128).contains(&exit_code.value()),
            SshProgram::Scp | SshProgram::Sftp | SshProgram::Rsync => exit_code.value() == 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Remote {
    user: Option<String>,
    host: String,
    port: Option<u16>,
}

impl Remote {
    /// Parses `[user@]host`. The user is everything before the last `@`, matching OpenSSH.
    fn from_user_host(user_host: &str, port: Option<u16>) -> Self {
        match user_host.rsplit_once('@') {
            Some((user, host)) => Self {
                user: Some(user.to_owned()),
                host: host.to_owned(),
                port,
            },
            None => Self {
                user: None,
                host: user_host.to_owned(),
                port,
            },
        }
    }
}

#[derive(Default)]
struct ParsedOptions<'a> {
    values: Vec<(char, &'a str)>,
    /// Whether the options ended with `--`.
    terminated: bool,
}

/// Splits leading options from operands the way OpenSSH's `getopt` does: the first operand or a
/// `--` ends the options. Options without an argument get an empty value.
fn split_options<'a>(
    args: &'a [String],
    options_with_argument: &str,
) -> (ParsedOptions<'a>, &'a [String]) {
    let mut options = ParsedOptions::default();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        if arg == "--" {
            options.terminated = true;
            index += 1;
            break;
        }
        let Some(flags) = arg.strip_prefix('-').filter(|flags| !flags.is_empty()) else {
            break;
        };
        index += 1;
        for (position, flag) in flags.char_indices() {
            if options_with_argument.contains(flag) {
                let attached = &flags[position + flag.len_utf8()..];
                let value = if attached.is_empty() {
                    index += 1;
                    args.get(index - 1).map_or("", String::as_str)
                } else {
                    attached
                };
                options.values.push((flag, value));
                break;
            }
            options.values.push((flag, ""));
        }
    }
    (options, args.get(index..).unwrap_or_default())
}

/// The parts of an SSH command line that affect which account it logs in to. As in OpenSSH, the
/// first user and port given win, so they must be applied in command line order.
#[derive(Default)]
struct LoginOptions {
    user: Option<String>,
    port: Option<u16>,
    may_relay: bool,
}

impl LoginOptions {
    fn set_user(&mut self, user: Option<String>) {
        if self.user.is_none() {
            self.user = user;
        }
    }

    fn set_port(&mut self, port: Option<u16>) {
        if self.port.is_none() {
            self.port = port;
        }
    }

    fn apply_ssh_options(&mut self, options: &[(char, &str)]) {
        for &(flag, value) in options {
            match flag {
                'l' => self.set_user(Some(value.to_owned())),
                'p' => self.set_port(parse_port(value)),
                'o' => self.apply_config(value),
                'J' => self.apply_jump(value),
                _ => {}
            }
        }
    }

    /// Applies an `-o` option.
    fn apply_config(&mut self, option: &str) {
        let Some((key, value)) = option
            .trim()
            .split_once(|c: char| c == '=' || c.is_whitespace())
        else {
            return;
        };
        let value = value
            .trim_start_matches(|c: char| c == '=' || c.is_whitespace())
            .trim_end();
        match key.to_ascii_lowercase().as_str() {
            "user" => self.set_user(Some(value.to_owned())),
            "port" => self.set_port(parse_port(value)),
            "proxyjump" | "proxycommand" => self.apply_jump(value),
            _ => {}
        }
    }

    fn apply_jump(&mut self, value: &str) {
        self.may_relay |= !value.eq_ignore_ascii_case("none");
    }
}

/// Parses an scp-style `[user@]host:[path]` or `scheme://[user@]host[:port][/path]` operand, or
/// returns `None` for a local path.
fn parse_remote_operand(operand: &str, scheme: &str) -> Option<Remote> {
    if operand.contains("://") {
        return parse_uri(operand, scheme);
    }
    // Mirrors scp's `colon()`: a colon before any slash marks a remote path, and brackets
    // protect the colons of an IPv6 address.
    if operand.starts_with(':') {
        return None;
    }
    let bytes = operand.as_bytes();
    let mut in_brackets = operand.starts_with('[');
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'@' if bytes.get(index + 1) == Some(&b'[') => in_brackets = true,
            b']' if in_brackets && bytes.get(index + 1) == Some(&b':') => {
                return Some(Remote::from_user_host(&operand[..=index], None));
            }
            b':' if !in_brackets => return Some(Remote::from_user_host(&operand[..index], None)),
            b'/' => return None,
            _ => {}
        }
    }
    None
}

/// Parses `scheme://[user[;parameters]@]host[:port][/path]`.
fn parse_uri(uri: &str, scheme: &str) -> Option<Remote> {
    let rest = uri.strip_prefix(scheme)?.strip_prefix("://")?;
    let authority = rest.split('/').next().unwrap_or_default();
    let (user, host_port) = match authority.rsplit_once('@') {
        Some((user, host_port)) => (
            Some(user.split(';').next().unwrap_or_default().to_owned()),
            host_port,
        ),
        None => (None, authority),
    };
    let (host, port) = match host_port.strip_prefix('[') {
        Some(bracketed) => {
            let (host, after_host) = bracketed.split_once(']')?;
            (host, after_host.strip_prefix(':').and_then(parse_port))
        }
        None => match host_port.split_once(':') {
            Some((host, port)) => (host, parse_port(port)),
            None => (host_port, None),
        },
    };
    (!host.is_empty()).then(|| Remote {
        user,
        host: host.to_owned(),
        port,
    })
}

fn parse_port(value: &str) -> Option<u16> {
    value.parse().ok().filter(|port| *port != 0)
}

fn is_env_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty() && name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
    })
}

fn hosts_match(command_host: &str, prompt_host: &str) -> bool {
    let unbracket = |host: &str| {
        host.strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
            .unwrap_or(host)
            .to_owned()
    };
    unbracket(command_host).eq_ignore_ascii_case(&unbracket(prompt_host))
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;

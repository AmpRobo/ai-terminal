//! Peer address of an SSH client that is already connected.
//!
//! A new tab must dial that address instead of resolving the hostname again.
//! The name that opened the current session can fail DNS (mDNS, split horizon,
//! a laptop hostname) while the TCP session is still up.

use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;

use super::util::destination_hostname;

/// IP of the established SSH connection for `hostname`.
///
/// Prefers the ControlMaster that owns `control_socket`. Otherwise uses an
/// `ssh` process whose command line contains `hostname`. Returns `None` when
/// the peer cannot be read, so the caller can keep the original hostname.
pub fn ssh_peer_address(
    hostname: &str,
    port: Option<u16>,
    control_socket: Option<&Path>,
) -> Option<String> {
    if hostname.is_empty() && control_socket.is_none() {
        return None;
    }
    let snapshot = ProcSnapshot::from_proc();
    let socket = control_socket.and_then(Path::to_str);
    let home = std::env::var("HOME").ok();
    peer_ip(&snapshot, hostname, port, socket, home.as_deref()).map(format_ip)
}

struct SshProc {
    cmdline: Vec<String>,
    inodes: Vec<u64>,
}

struct ProcSnapshot {
    tcp: String,
    tcp6: String,
    unix: String,
    processes: Vec<SshProc>,
}

impl ProcSnapshot {
    fn from_proc() -> Self {
        Self {
            tcp: read_to_string("/proc/net/tcp"),
            tcp6: read_to_string("/proc/net/tcp6"),
            unix: read_to_string("/proc/net/unix"),
            processes: ssh_processes(),
        }
    }
}

struct TcpConn {
    ip: IpAddr,
    port: u16,
    inode: u64,
    established: bool,
}

fn peer_ip(
    snapshot: &ProcSnapshot,
    hostname: &str,
    port: Option<u16>,
    control_socket: Option<&str>,
    home: Option<&str>,
) -> Option<IpAddr> {
    let conns = parse_tcp_tables(&snapshot.tcp, &snapshot.tcp6);
    if let Some(path) = control_socket.filter(|path| !path.is_empty()) {
        let inodes = unix_socket_inodes(&snapshot.unix, path, home);
        if !inodes.is_empty() {
            let peers = peers_of(&snapshot.processes, &conns, |process| {
                process.inodes.iter().any(|inode| inodes.contains(inode))
            });
            if let Some(ip) = select_peer(&peers, port) {
                return Some(ip);
            }
        }
    }
    if hostname.is_empty() {
        return None;
    }
    let peers = peers_of(&snapshot.processes, &conns, |process| {
        cmdline_mentions_host(&process.cmdline, hostname)
    });
    select_peer(&peers, port)
}

fn peers_of(
    processes: &[SshProc],
    conns: &[TcpConn],
    include: impl Fn(&SshProc) -> bool,
) -> Vec<(IpAddr, u16)> {
    processes
        .iter()
        .filter(|process| include(process))
        .flat_map(|process| established_peers(conns, &process.inodes))
        .collect()
}

fn established_peers(conns: &[TcpConn], inodes: &[u64]) -> Vec<(IpAddr, u16)> {
    conns
        .iter()
        .filter(|conn| conn.established && inodes.contains(&conn.inode))
        .map(|conn| (conn.ip, conn.port))
        .collect()
}

fn select_peer(peers: &[(IpAddr, u16)], port: Option<u16>) -> Option<IpAddr> {
    let usable: Vec<(IpAddr, u16)> = peers
        .iter()
        .copied()
        .filter(|(ip, _)| is_usable_peer(*ip))
        .collect();
    if usable.is_empty() {
        return None;
    }
    let preferred_port = port.unwrap_or(22);
    let on_port: Vec<(IpAddr, u16)> = usable
        .iter()
        .copied()
        .filter(|(_, peer_port)| *peer_port == preferred_port)
        .collect();
    let pool = if on_port.is_empty() {
        &usable
    } else {
        &on_port
    };
    pool.iter()
        .find(|(ip, _)| !ip.is_loopback())
        .or_else(|| pool.first())
        .map(|(ip, _)| *ip)
}

fn is_usable_peer(ip: IpAddr) -> bool {
    !ip.is_unspecified() && !ip.is_multicast()
}

fn cmdline_mentions_host(cmdline: &[String], hostname: &str) -> bool {
    if hostname.is_empty() {
        return false;
    }
    cmdline.iter().any(|arg| {
        !arg.starts_with('-') && destination_hostname(arg).eq_ignore_ascii_case(hostname)
    })
}

fn format_ip(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| ip.to_string()),
    }
}

fn parse_tcp_tables(tcp: &str, tcp6: &str) -> Vec<TcpConn> {
    let mut conns = parse_tcp_table(tcp);
    conns.extend(parse_tcp_table(tcp6));
    conns
}

fn parse_tcp_table(contents: &str) -> Vec<TcpConn> {
    contents.lines().filter_map(parse_tcp_line).collect()
}

fn parse_tcp_line(line: &str) -> Option<TcpConn> {
    let mut fields = line.split_whitespace();
    let index = fields.next()?;
    // Data rows are `0:`, `1:`, ... The header's first field is `sl`.
    if !index.ends_with(':') {
        return None;
    }
    fields.next()?;
    let remote = fields.next()?;
    let state = fields.next()?;
    // tx:rx, tr:when, retrnsmt, uid, timeout — then inode.
    for _ in 0..5 {
        fields.next()?;
    }
    let inode = fields.next()?.parse().ok()?;
    let (ip, port) = decode_endpoint(remote)?;
    Some(TcpConn {
        ip,
        port,
        inode,
        established: state == "01",
    })
}

fn decode_endpoint(field: &str) -> Option<(IpAddr, u16)> {
    let (ip_hex, port_hex) = field.rsplit_once(':')?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;
    let ip = if ip_hex.len() == 8 {
        decode_ipv4(ip_hex)?
    } else {
        decode_ipv6(ip_hex)?
    };
    Some((ip, port))
}

/// `/proc/net/tcp` prints each address word with `%08X`, which is the native
/// integer value of a big-endian address. The in-memory bytes are the IP.
fn decode_ipv4(hex: &str) -> Option<IpAddr> {
    if hex.len() != 8 {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some(IpAddr::V4(Ipv4Addr::from(value.to_ne_bytes())))
}

fn decode_ipv6(hex: &str) -> Option<IpAddr> {
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for group in 0..4 {
        let start = group * 8;
        let value = u32::from_str_radix(&hex[start..start + 8], 16).ok()?;
        let offset = group * 4;
        bytes[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }
    Some(IpAddr::V6(Ipv6Addr::from(bytes)))
}

fn unix_socket_inodes(contents: &str, socket_path: &str, home: Option<&str>) -> Vec<u64> {
    let candidates = control_socket_candidates(socket_path, home);
    let mut inodes = Vec::new();
    for line in contents.lines() {
        let mut fields = line.split_whitespace();
        let Some(inode) = fields.nth(6) else {
            continue;
        };
        let path = fields.collect::<Vec<_>>().join(" ");
        if !candidates
            .iter()
            .any(|candidate| socket_path_matches(&path, candidate))
        {
            continue;
        }
        if let Ok(inode) = inode.parse() {
            inodes.push(inode);
        }
    }
    inodes
}

/// The session stores `~/.ssh/<id>`. OpenSSH binds that path under `$HOME` and
/// appends `.<cookie>`, so an exact string compare never sees the socket.
fn control_socket_candidates(socket_path: &str, home: Option<&str>) -> Vec<String> {
    let mut candidates = vec![socket_path.to_string()];
    if let Some(rest) = socket_path.strip_prefix("~/")
        && !rest.is_empty()
        && let Some(home) = home.filter(|home| !home.is_empty())
    {
        candidates.push(Path::new(home).join(rest).to_string_lossy().into_owned());
    }
    candidates
}

fn socket_path_matches(bound: &str, requested: &str) -> bool {
    bound == requested
        || bound
            .strip_prefix(requested)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

fn read_to_string(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn ssh_processes() -> Vec<SshProc> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let pid = entry.file_name().to_str()?.parse::<u32>().ok()?;
            let comm = fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
            if comm.trim_end() != "ssh" {
                return None;
            }
            Some(SshProc {
                cmdline: cmdline(pid),
                inodes: socket_inodes(pid),
            })
        })
        .collect()
}

fn cmdline(pid: u32) -> Vec<String> {
    let Ok(bytes) = fs::read(format!("/proc/{pid}/cmdline")) else {
        return Vec::new();
    };
    bytes
        .split(|byte| *byte == 0)
        .filter(|arg| !arg.is_empty())
        .map(|arg| String::from_utf8_lossy(arg).into_owned())
        .collect()
}

fn socket_inodes(pid: u32) -> Vec<u64> {
    let Ok(entries) = fs::read_dir(format!("/proc/{pid}/fd")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let target = fs::read_link(entry.path()).ok()?;
            let text = target.to_str()?;
            let inode = text.strip_prefix("socket:[")?.strip_suffix(']')?;
            inode.parse().ok()
        })
        .collect()
}

#[cfg(test)]
#[path = "peer_addr_tests.rs"]
mod tests;

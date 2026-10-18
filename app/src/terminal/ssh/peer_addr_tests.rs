use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::*;

const TCP_HEADER: &str = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n";

fn tcp_line(remote: &str, state: &str, inode: u64) -> String {
    format!(
        "   0: 0A00A8C0:C3A1 {remote} {state} 00000000:00000000 00:00000000 00000000 1000 0 {inode} 1 0000000000000000 20 0 0 10 -1\n"
    )
}

fn snapshot(tcp: &str, unix: &str, processes: Vec<SshProc>) -> ProcSnapshot {
    ProcSnapshot {
        tcp: tcp.to_string(),
        tcp6: String::new(),
        unix: unix.to_string(),
        processes,
    }
}

#[test]
fn decodes_ipv4_proc_address() {
    let (ip, port) = decode_endpoint("0501A8C0:0016").unwrap();
    assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5)));
    assert_eq!(port, 22);
    let (loopback, _) = decode_endpoint("0100007F:0050").unwrap();
    assert_eq!(loopback, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
}

#[test]
fn decodes_ipv6_proc_address() {
    let (ip, port) = decode_endpoint("00000000000000000000000001000000:0016").unwrap();
    assert_eq!(ip, IpAddr::V6(Ipv6Addr::LOCALHOST));
    assert_eq!(port, 22);
}

#[test]
fn decodes_ipv4_mapped_ipv6_as_ipv4() {
    let (ip, _) = decode_endpoint("0000000000000000FFFF00000501A8C0:0016").unwrap();
    assert_eq!(format_ip(ip), "192.168.1.5");
}

#[test]
fn control_socket_peer_beats_another_ssh_process() {
    let tcp = format!(
        "{TCP_HEADER}{}{}",
        tcp_line("0100000A:0016", "01", 100),
        tcp_line("0200000A:0016", "01", 200),
    );
    let unix = "Num RefCount Protocol Flags Type St Inode Path\n\
                00000000: 00000002 00000000 00010000 0001 01 999 /tmp/ssh-control\n";
    let processes = vec![
        SshProc {
            cmdline: vec!["ssh".into(), "michael@ThinkPad-X1".into()],
            inodes: vec![999, 100],
        },
        SshProc {
            cmdline: vec!["ssh".into(), "michael@ThinkPad-X1".into()],
            inodes: vec![200],
        },
    ];
    let ip = peer_ip(
        &snapshot(&tcp, unix, processes),
        "ThinkPad-X1",
        None,
        Some("/tmp/ssh-control"),
        None,
    );
    assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
}

#[test]
fn cmdline_match_finds_established_peer() {
    let tcp = format!(
        "{TCP_HEADER}{}{}",
        tcp_line("0501A8C0:0016", "01", 100),
        tcp_line("08080808:0016", "01", 200),
    );
    let processes = vec![
        SshProc {
            cmdline: vec!["ssh".into(), "michael@other-host".into()],
            inodes: vec![200],
        },
        SshProc {
            cmdline: vec![
                "ssh".into(),
                "-o".into(),
                "ControlPath=/tmp/sock".into(),
                "michael@ThinkPad-X1".into(),
            ],
            inodes: vec![100],
        },
    ];
    let ip = peer_ip(
        &snapshot(&tcp, "", processes),
        "thinkpad-x1",
        Some(22),
        None,
        None,
    );
    assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 5))));
}

#[test]
fn ignores_listening_and_unspecified_sockets() {
    let tcp = format!(
        "{TCP_HEADER}{}{}",
        tcp_line("0501A8C0:0016", "0A", 100),
        tcp_line("00000000:0000", "01", 100),
    );
    let processes = vec![SshProc {
        cmdline: vec!["ssh".into(), "ThinkPad-X1".into()],
        inodes: vec![100],
    }];
    assert_eq!(
        peer_ip(
            &snapshot(&tcp, "", processes),
            "ThinkPad-X1",
            None,
            None,
            None,
        ),
        None
    );
}

#[test]
fn prefers_requested_port_and_non_loopback() {
    let tcp = format!(
        "{TCP_HEADER}{}{}{}",
        tcp_line("0100007F:0016", "01", 1),
        tcp_line("0501A8C0:0016", "01", 2),
        tcp_line("08080808:08AE", "01", 3),
    );
    let processes = vec![SshProc {
        cmdline: vec!["ssh".into(), "-p".into(), "2222".into(), "host".into()],
        inodes: vec![1, 2, 3],
    }];
    let ip = peer_ip(
        &snapshot(&tcp, "", processes),
        "host",
        Some(2222),
        None,
        None,
    );
    assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
}

#[test]
fn tilde_control_socket_matches_openssh_suffixed_path() {
    // 192.168.3.225, the peer of a ControlPath that OpenSSH binds as
    // `$HOME/.ssh/<id>.<cookie>` while the session records `~/.ssh/<id>`.
    let tcp = format!("{TCP_HEADER}{}", tcp_line("E103A8C0:0016", "01", 342826));
    let unix = "Num RefCount Protocol Flags Type St Inode Path\n\
                00000000: 00000002 00000000 00010000 0001 01 357395 /home/michael/.ssh/8881242657362942036.AUvnxPLwSHhwazti\n";
    let processes = vec![SshProc {
        cmdline: vec!["ssh".into(), "michael@192.168.3.225".into()],
        inodes: vec![357395, 342826],
    }];
    let ip = peer_ip(
        &snapshot(&tcp, unix, processes),
        "ThinkPad-X1",
        None,
        Some("~/.ssh/8881242657362942036"),
        Some("/home/michael"),
    );
    assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 3, 225))));
}

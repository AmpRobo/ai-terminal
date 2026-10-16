use super::SshCredentialKey;

fn key(user: Option<&str>, host: &str, port: Option<u16>) -> SshCredentialKey {
    SshCredentialKey::new(user.map(str::to_owned), host, port)
}

#[test]
fn key_ignores_host_case_brackets_and_the_default_port() {
    assert_eq!(
        key(Some("root"), "Example.COM", Some(22)),
        key(Some("root"), "example.com", None)
    );
    assert_eq!(key(None, "[fe80::1]", None), key(None, "fe80::1", None));
    assert_eq!(key(Some(""), "host", None), key(None, "host", None));
}

#[test]
fn key_tells_apart_users_hosts_and_ports() {
    let account = key(Some("root"), "10.0.0.1", None);
    assert_ne!(account, key(Some("admin"), "10.0.0.1", None));
    assert_ne!(account, key(None, "10.0.0.1", None));
    assert_ne!(account, key(Some("root"), "10.0.0.2", None));
    assert_ne!(account, key(Some("root"), "10.0.0.1", Some(2222)));
}

#[test]
fn key_displays_as_an_ssh_destination() {
    assert_eq!(
        key(Some("root"), "10.0.0.1", None).to_string(),
        "root@10.0.0.1"
    );
    assert_eq!(
        key(None, "example.com", Some(2222)).to_string(),
        "example.com:2222"
    );
    assert_eq!(
        key(Some("me"), "fe80::1", Some(2222)).to_string(),
        "me@[fe80::1]:2222"
    );
}

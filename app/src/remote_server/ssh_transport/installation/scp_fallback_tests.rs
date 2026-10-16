use remote_server::setup::{RemoteArch, RemoteOs};

use super::*;

fn linux(arch: RemoteArch) -> RemotePlatform {
    RemotePlatform {
        os: RemoteOs::Linux,
        arch,
    }
}

fn write_tarball(bundled_dir: &Path, platform_dir: &str, contents: &[u8]) -> PathBuf {
    let tarball = bundled_dir.join(platform_dir).join("oz.tar.gz");
    std::fs::create_dir_all(tarball.parent().expect("tarball has a parent"))
        .expect("create platform dir");
    std::fs::write(&tarball, contents).expect("write tarball");
    tarball
}

#[tokio::test]
async fn bundled_tarball_matches_the_remote_platform() {
    let bundled_dir = tempfile::tempdir().expect("create temp dir");
    let tarball = write_tarball(bundled_dir.path(), "linux-aarch64", b"tarball");

    assert_eq!(
        bundled_tarball(bundled_dir.path(), &linux(RemoteArch::Aarch64)).await,
        Some(tarball)
    );
    assert_eq!(
        bundled_tarball(bundled_dir.path(), &linux(RemoteArch::X86_64)).await,
        None
    );
}

#[tokio::test]
async fn bundled_tarball_ignores_empty_files() {
    let bundled_dir = tempfile::tempdir().expect("create temp dir");
    write_tarball(bundled_dir.path(), "linux-x86_64", b"");

    assert_eq!(
        bundled_tarball(bundled_dir.path(), &linux(RemoteArch::X86_64)).await,
        None
    );
}

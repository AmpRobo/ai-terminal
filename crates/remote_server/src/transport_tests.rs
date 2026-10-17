use super::*;

#[test]
fn other_errors_are_included_in_the_banner_detail() {
    let error = Error::Other(anyhow::anyhow!(
        "failed to locate the local AI Terminal binary"
    ));
    let facing = error.user_facing_error(SetupStage::InstallBinary);
    assert_eq!(facing.body, "Failed to install SSH extension");
    assert_eq!(
        facing.detail.as_deref(),
        Some("failed to locate the local AI Terminal binary")
    );
}

#[test]
fn other_error_detail_is_truncated() {
    let error = Error::Other(anyhow::anyhow!("x".repeat(MAX_STDERR_DISPLAY_CHARS + 20)));
    let facing = error.user_facing_error(SetupStage::InstallBinary);
    let detail = facing.detail.expect("detail");
    assert!(detail.ends_with('…'));
    assert_eq!(detail.chars().count(), MAX_STDERR_DISPLAY_CHARS + 1);
}

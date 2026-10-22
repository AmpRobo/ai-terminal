use super::*;

#[test]
fn request_rejected_as_malformed_reports_an_outdated_extension() {
    let err = HostRequestError::ServerError {
        code: ErrorCode::InvalidRequest,
        message: "HostScopedRequest had no message variant set".to_string(),
    };

    assert_eq!(
        remote_mutation_error_message("rename", &err),
        "Failed to rename: the SSH extension on this host is out of date."
    );
}

#[test]
fn filesystem_failure_includes_the_server_message() {
    let err = HostRequestError::OperationFailed("Permission denied (os error 13)".to_string());

    assert_eq!(
        remote_mutation_error_message("delete", &err),
        "Failed to delete: Permission denied (os error 13)"
    );
}

#[test]
fn internal_server_error_is_reported_generically() {
    let err = HostRequestError::ServerError {
        code: ErrorCode::Internal,
        message: "boom".to_string(),
    };

    assert_eq!(
        remote_mutation_error_message("delete", &err),
        "Failed to delete."
    );
}

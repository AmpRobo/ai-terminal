use super::should_ignore_shell_title_after_bootstrap;

#[test]
fn keeps_the_working_directory_title_for_a_plain_terminal() {
    assert!(!should_ignore_shell_title_after_bootstrap("bash", "bash"));
    assert!(!should_ignore_shell_title_after_bootstrap("~/src", "~/src"));
    assert!(!should_ignore_shell_title_after_bootstrap("", ""));
}

#[test]
fn keeps_a_conversation_title_ahead_of_the_shell_title() {
    assert!(should_ignore_shell_title_after_bootstrap(
        "New agent conversation",
        "bash",
    ));
    assert!(should_ignore_shell_title_after_bootstrap(
        "New agent conversation",
        "~/src",
    ));
}

use std::process::Command;

#[test]
fn test_chat_subcommand_help_and_discovery() {
    let output = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args(["chat", "--help"])
        .output()
        .expect("failed to execute replayfix chat --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Launch interactive conversational agent workspace"));
    assert!(stdout.contains("--repo"));
    assert!(stdout.contains("--output"));
}

#[test]
fn test_default_no_args_non_interactive_prints_banner_without_hanging() {
    // When stdin is redirected / not a TTY, running without args prints the banner and exits cleanly
    let output = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .output()
        .expect("failed to execute replayfix without arguments");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Deterministic Verification Harness") || stdout.contains("ReplayFix"));
}

#[test]
fn test_interactive_alias_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args(["agent", "--help"])
        .output()
        .expect("failed to execute replayfix agent --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Launch interactive conversational agent workspace"));
}

#[test]
fn test_session_subcommand_list_and_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args(["session", "list", "--json"])
        .output()
        .expect("failed to execute replayfix session list --json");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: serde_json::Value = serde_json::from_str(&stdout).expect("valid json output");
    assert!(val.is_array());
}

#[test]
fn test_session_resume_not_found_fails_closed() {
    let output = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args(["session", "resume", "nonexistent-sess-12345"])
        .output()
        .expect("failed to execute replayfix session resume");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Session not found"));
}

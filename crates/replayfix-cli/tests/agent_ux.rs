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

#[test]
fn test_conversational_session_serialization_and_resumption_roundtrip() {
    use std::io::Write;
    use std::process::Stdio;

    let temp_out =
        std::env::temp_dir().join(format!("replayfix-v14-sess-audit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_out);

    let mut child = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args(["chat", "--output", &temp_out.display().to_string()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn replayfix chat");

    {
        let stdin = child.stdin.as_mut().expect("stdin piped");
        // 1. Triage RF-01 offline in sandbox, confirm plan 'y', reject applying patch to base repo 'n', then '/exit'
        writeln!(stdin, "/repro RF-01").unwrap();
        writeln!(stdin, "y").unwrap();
        writeln!(stdin, "n").unwrap();
        writeln!(stdin, "/exit").unwrap();
    }

    let chat_out = child.wait_with_output().expect("wait on chat");
    assert!(chat_out.status.success());

    // Vector 6.1: Verify `session list --json` and `.replayfix/sessions/<session_id>.json`
    let list_out = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args([
            "session",
            "list",
            "--output",
            &temp_out.display().to_string(),
            "--json",
        ])
        .output()
        .expect("failed to execute session list");
    assert!(list_out.status.success());

    let sessions: serde_json::Value =
        serde_json::from_slice(&list_out.stdout).expect("must parse session list JSON");
    let arr = sessions.as_array().expect("sessions array");
    assert_eq!(arr.len(), 1);

    let session_id = arr[0]["session_id"].as_str().expect("session_id string");
    let session_file = temp_out.join("sessions").join(format!("{session_id}.json"));
    assert!(session_file.exists(), "session file must exist on disk");

    let file_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&session_file).expect("read session file"))
            .expect("valid session JSON");

    assert_eq!(file_json["active_task"].as_str(), Some("RF-01"));
    assert!(!file_json["turns"].as_array().unwrap().is_empty());
    assert!(!file_json["cached_aria_snapshots"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(file_json["verified_gate_state"].as_str().is_some());
    assert!(!file_json["candidate_patches"]
        .as_array()
        .unwrap()
        .is_empty());

    // Vector 6.2: Verify `session resume <session_id>` loads the persisted state and history
    let mut resume_child = Command::new(env!("CARGO_BIN_EXE_replayfix"))
        .args([
            "session",
            "resume",
            session_id,
            "--output",
            &temp_out.display().to_string(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn session resume");

    {
        let stdin = resume_child.stdin.as_mut().expect("stdin piped");
        writeln!(stdin, "/history").unwrap();
        writeln!(stdin, "/exit").unwrap();
    }

    let resume_out = resume_child.wait_with_output().expect("wait on resume");
    assert!(resume_out.status.success());
    let resume_stdout = String::from_utf8_lossy(&resume_out.stdout);
    assert!(resume_stdout.contains("Successfully verified repair for RF-01"));

    let _ = std::fs::remove_dir_all(&temp_out);
}

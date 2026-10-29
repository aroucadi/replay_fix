use replayfix_core::digest::sha256_hex;
use replayfix_core::error::{CoreError, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalMode {
    ManualAffirmative,
    AutoBypassFlag,
    NonInteractiveRejected,
    ManualNegative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalAudit {
    pub timestamp: String,
    pub patch_sha256: String,
    pub files_modified: Vec<String>,
    pub insertions: usize,
    pub deletions: usize,
    pub decision: ApprovalDecision,
    pub mode: ApprovalMode,
    pub tty_detected: bool,
}

/// Highlight unified diff syntax using standard ANSI escape codes.
#[must_use]
pub fn highlight_diff(diff: &str) -> String {
    let mut out = String::with_capacity(diff.len() + 1024);

    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            out.push_str("\x1b[1;37m"); // Bold White
            out.push_str(line);
            out.push_str("\x1b[0m\n");
        } else if line.starts_with("diff --git") || line.starts_with("index ") {
            out.push_str("\x1b[1;34m"); // Bold Blue
            out.push_str(line);
            out.push_str("\x1b[0m\n");
        } else if line.starts_with("@@") {
            out.push_str("\x1b[36m"); // Cyan
            out.push_str(line);
            out.push_str("\x1b[0m\n");
        } else if line.starts_with('+') {
            out.push_str("\x1b[32m"); // Green
            out.push_str(line);
            out.push_str("\x1b[0m\n");
        } else if line.starts_with('-') {
            out.push_str("\x1b[31m"); // Red
            out.push_str(line);
            out.push_str("\x1b[0m\n");
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    out
}

/// Parse diff statistics: list of modified files, insertions, deletions.
#[must_use]
pub fn parse_diff_stats(diff: &str) -> (Vec<String>, usize, usize) {
    let mut files = Vec::new();
    let mut insertions = 0;
    let mut deletions = 0;

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            let file = rest.trim().to_string();
            if !files.contains(&file) {
                files.push(file);
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            insertions += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }

    (files, insertions, deletions)
}

/// Evaluation result of human confirmation gate.
#[derive(Debug)]
pub struct GateResult {
    pub decision: ApprovalDecision,
    pub audit: ApprovalAudit,
}

/// Request human approval for candidate patch.
///
/// Invariants:
/// - If `auto_confirm` is true (`--yes`), returns `Approved` with mode `AutoBypassFlag`.
/// - If stdin is not a TTY and `auto_confirm` is false, returns `Rejected` with mode `NonInteractiveRejected` (fail-closed).
/// - If stdin is a TTY, displays syntax-highlighted diff and prompts user `[y/N]`.
pub fn evaluate_approval_gate(
    patch_content: &str,
    auto_confirm: bool,
    force_tty: Option<bool>,
    reader: Option<&mut dyn BufRead>,
    mut writer: Option<&mut dyn Write>,
) -> Result<GateResult> {
    let (files_modified, insertions, deletions) = parse_diff_stats(patch_content);
    let patch_sha256 = sha256_hex(patch_content.as_bytes());
    let tty_detected = force_tty.unwrap_or_else(|| io::stdin().is_terminal());

    if auto_confirm {
        let audit = ApprovalAudit {
            timestamp: chrono::Utc::now().to_rfc3339(),
            patch_sha256,
            files_modified,
            insertions,
            deletions,
            decision: ApprovalDecision::Approved,
            mode: ApprovalMode::AutoBypassFlag,
            tty_detected,
        };
        return Ok(GateResult {
            decision: ApprovalDecision::Approved,
            audit,
        });
    }

    if !tty_detected {
        let audit = ApprovalAudit {
            timestamp: chrono::Utc::now().to_rfc3339(),
            patch_sha256,
            files_modified,
            insertions,
            deletions,
            decision: ApprovalDecision::Rejected,
            mode: ApprovalMode::NonInteractiveRejected,
            tty_detected: false,
        };
        return Ok(GateResult {
            decision: ApprovalDecision::Rejected,
            audit,
        });
    }

    // Interactive confirmation flow
    if let Some(w) = writer.as_mut() {
        let highlighted = highlight_diff(patch_content);
        let _ = writeln!(w, "\n--- Candidate Patch Preview ({}) ---", patch_sha256);
        let _ = write!(w, "{}", highlighted);
        let _ = writeln!(
            w,
            "Files: {} | Insertions (+): {} | Deletions (-): {}",
            files_modified.join(", "),
            insertions,
            deletions
        );
        let _ = write!(w, "\nApply candidate patch to base repository? [y/N]: ");
        let _ = w.flush();
    } else {
        let highlighted = highlight_diff(patch_content);
        println!("\n--- Candidate Patch Preview ({}) ---", patch_sha256);
        print!("{}", highlighted);
        println!(
            "Files: {} | Insertions (+): {} | Deletions (-): {}",
            files_modified.join(", "),
            insertions,
            deletions
        );
        print!("\nApply candidate patch to base repository? [y/N]: ");
        let _ = io::stdout().flush();
    }

    let mut user_input = String::new();
    let read_result = if let Some(r) = reader {
        r.read_line(&mut user_input)
    } else {
        io::stdin().lock().read_line(&mut user_input)
    };

    match read_result {
        Ok(0) => {
            // EOF reached without affirmative answer -> reject (fail-closed)
            let audit = ApprovalAudit {
                timestamp: chrono::Utc::now().to_rfc3339(),
                patch_sha256,
                files_modified,
                insertions,
                deletions,
                decision: ApprovalDecision::Rejected,
                mode: ApprovalMode::ManualNegative,
                tty_detected: true,
            };
            Ok(GateResult {
                decision: ApprovalDecision::Rejected,
                audit,
            })
        }
        Ok(_) => {
            let choice = user_input.trim().to_lowercase();
            let approved = choice == "y" || choice == "yes";
            let decision = if approved {
                ApprovalDecision::Approved
            } else {
                ApprovalDecision::Rejected
            };
            let mode = if approved {
                ApprovalMode::ManualAffirmative
            } else {
                ApprovalMode::ManualNegative
            };

            let audit = ApprovalAudit {
                timestamp: chrono::Utc::now().to_rfc3339(),
                patch_sha256,
                files_modified,
                insertions,
                deletions,
                decision,
                mode,
                tty_detected: true,
            };

            Ok(GateResult { decision, audit })
        }
        Err(e) => Err(CoreError::Execution(format!(
            "Failed reading approval input: {}",
            e
        ))),
    }
}

/// Persist approval audit record to file.
pub fn write_approval_audit(audit: &ApprovalAudit, output_dir: impl AsRef<Path>) -> Result<()> {
    let p = output_dir.as_ref();
    let artifacts_dir = if p.ends_with("artifacts") {
        p.to_path_buf()
    } else if p.ends_with(".replayfix") {
        p.join("artifacts")
    } else {
        p.join(".replayfix").join("artifacts")
    };
    let _ = std::fs::create_dir_all(&artifacts_dir);
    let target = artifacts_dir.join("approval_audit.json");
    let json_bytes = serde_json::to_vec_pretty(audit)?;
    std::fs::write(&target, json_bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = r#"--- a/src/App.tsx
+++ b/src/App.tsx
@@ -10,3 +10,3 @@
-  const count = state + 1;
+  const count = state;
   return <div>{count}</div>;
"#;

    #[test]
    fn parses_stats_and_highlights_diff() {
        let (files, ins, dels) = parse_diff_stats(SAMPLE_DIFF);
        assert_eq!(files, vec!["src/App.tsx"]);
        assert_eq!(ins, 1);
        assert_eq!(dels, 1);

        let highlighted = highlight_diff(SAMPLE_DIFF);
        assert!(highlighted.contains("\x1b[32m")); // Green for +
        assert!(highlighted.contains("\x1b[31m")); // Red for -
        assert!(highlighted.contains("\x1b[36m")); // Cyan for @@
    }

    #[test]
    fn auto_bypass_approves() {
        let res = evaluate_approval_gate(SAMPLE_DIFF, true, None, None, None).unwrap();
        assert_eq!(res.decision, ApprovalDecision::Approved);
        assert_eq!(res.audit.mode, ApprovalMode::AutoBypassFlag);
    }

    #[test]
    fn non_interactive_without_yes_rejects_fail_closed() {
        let res = evaluate_approval_gate(SAMPLE_DIFF, false, Some(false), None, None).unwrap();
        assert_eq!(res.decision, ApprovalDecision::Rejected);
        assert_eq!(res.audit.mode, ApprovalMode::NonInteractiveRejected);
    }

    #[test]
    fn interactive_affirmative_y_approves() {
        let mut input = io::Cursor::new(b"y\n");
        let mut output = Vec::new();
        let res = evaluate_approval_gate(
            SAMPLE_DIFF,
            false,
            Some(true),
            Some(&mut input),
            Some(&mut output),
        )
        .unwrap();

        assert_eq!(res.decision, ApprovalDecision::Approved);
        assert_eq!(res.audit.mode, ApprovalMode::ManualAffirmative);
    }

    #[test]
    fn interactive_negative_n_rejects() {
        let mut input = io::Cursor::new(b"n\n");
        let mut output = Vec::new();
        let res = evaluate_approval_gate(
            SAMPLE_DIFF,
            false,
            Some(true),
            Some(&mut input),
            Some(&mut output),
        )
        .unwrap();

        assert_eq!(res.decision, ApprovalDecision::Rejected);
        assert_eq!(res.audit.mode, ApprovalMode::ManualNegative);
    }

    #[test]
    fn interactive_default_enter_rejects() {
        let mut input = io::Cursor::new(b"\n");
        let mut output = Vec::new();
        let res = evaluate_approval_gate(
            SAMPLE_DIFF,
            false,
            Some(true),
            Some(&mut input),
            Some(&mut output),
        )
        .unwrap();

        assert_eq!(res.decision, ApprovalDecision::Rejected);
        assert_eq!(res.audit.mode, ApprovalMode::ManualNegative);
    }
}

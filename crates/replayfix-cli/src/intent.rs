//! Conversational Natural-Language Intent Parser & Execution Plan Builder.
//!
//! Translates conversational English inputs and slash-commands into structured
//! execution intents and transparent plan cards for ReplayFix Release 1.4.

use crate::permissions::RiskLevel;
use crate::ui::{Symbols, Theme};
use std::path::PathBuf;

/// High-level user intent interpreted from conversational input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentIntent {
    /// Reproduce a defect causally and execute bounded repair.
    ReproAndRepair {
        task_id: Option<String>,
        repo_path: Option<String>,
        live_browser: bool,
        headed: bool,
        model: Option<String>,
        clean_ref: Option<String>,
    },
    /// Run system prerequisite diagnostics.
    DoctorCheck { json: bool },
    /// Cryptographically verify an RFC 6962 Merkle receipt.
    VerifyReceipt { path: PathBuf },
    /// Evaluate all 5 benchmark defect scenarios in MetaPulse.
    BenchmarkSweep { bugs: Vec<String> },
    /// Synthesize permanent Playwright regression inoculation test for a task.
    InoculateTask { task_id: String },
    /// Package a verified repair and Evidence Matrix PR into an isolated Git branch.
    PackagePr { task_id: String },
    /// Export Judge Proof Portal package and demo presets.
    ExportProof,
    /// Display current session and workspace context.
    ShowStatus,
    /// Display recent verification activity and audit receipts.
    ShowHistory,
    /// Resume an interrupted or checkpointed session.
    ResumeSession,
    /// Provide conversational guidance or command discovery.
    Help { topic: Option<String> },
    /// Terminate interactive agent session.
    Exit,
    /// Ambiguous query requiring clarification.
    ClarificationNeeded {
        prompt: String,
        candidates: Vec<String>,
    },
    /// Unrecognized utterance.
    Unknown(String),
}

/// A planned step in an agent workflow.
#[derive(Debug, Clone)]
pub struct PlannedStep {
    pub name: &'static str,
    pub description: String,
    pub tool_or_component: &'static str,
    pub risk: RiskLevel,
}

/// A formatted execution plan presented to the user prior to consequential execution.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub title: String,
    pub task_id: String,
    pub repo_target: String,
    pub steps: Vec<PlannedStep>,
}

impl ExecutionPlan {
    #[must_use]
    pub fn for_repro(task_id: &str, repo_path: &str, live_browser: bool, headed: bool) -> Self {
        let browser_mode = if live_browser {
            if headed {
                "Visible Chromium (Playwright CDP Headed)"
            } else {
                "Headless Chromium (Playwright CDP)"
            }
        } else {
            "Simulated Static Oracle"
        };

        Self {
            title: format!("Causal Reproduction & Bounded Repair: {task_id}"),
            task_id: task_id.to_string(),
            repo_target: repo_path.to_string(),
            steps: vec![
                PlannedStep {
                    name: "1. Workspace Sandboxing",
                    description: "Clone target repo to ephemeral scratch directory; verify Δrepo = 0"
                        .to_string(),
                    tool_or_component: "replayfix-core::ScratchSandbox",
                    risk: RiskLevel::Low,
                },
                PlannedStep {
                    name: "2. Telemetry Ingestion & Perception",
                    description: "Extract ROI keyframes and align ASR audio narration cues"
                        .to_string(),
                    tool_or_component: "ffmpeg / SAM 3.1 / Whisper",
                    risk: RiskLevel::Low,
                },
                PlannedStep {
                    name: "3. Gate 1 (Baseline RED)",
                    description: format!(
                        "Execute baseline interaction in {browser_mode}; establish causal defect"
                    ),
                    tool_or_component: "playwright-cdp / oracle",
                    risk: RiskLevel::Medium,
                },
                PlannedStep {
                    name: "4. Test Freeze & Bounded Repair",
                    description: "Capture SHA-256 test freeze; synthesize patch with Meta Muse (max 3 attempts)"
                        .to_string(),
                    tool_or_component: "Meta Muse Spark / Wonder",
                    risk: RiskLevel::Medium,
                },
                PlannedStep {
                    name: "5. Preflight Syntax & Gate 2 (GREEN)",
                    description: "Validate with node --check; confirm 2-run stability and test parity"
                        .to_string(),
                    tool_or_component: "node / regression-parity",
                    risk: RiskLevel::Low,
                },
                PlannedStep {
                    name: "6. Cryptographic Attestation",
                    description: "Sign artifacts into RFC 6962 Merkle tree with Ed25519 attestation"
                        .to_string(),
                    tool_or_component: "ed25519-dalek / audit_receipt.json",
                    risk: RiskLevel::Low,
                },
                PlannedStep {
                    name: "7. Human Review Gate",
                    description: "Inspect syntax-highlighted diff; prompt for patch confirmation"
                        .to_string(),
                    tool_or_component: "replayfix-cli::DiffReview",
                    risk: RiskLevel::High,
                },
            ],
        }
    }

    /// Render structured plan card for user review.
    pub fn render_card(&self, theme: &Theme) {
        println!();
        let bar = "─".repeat(60);
        println!("  {}", theme.muted(&bar));
        println!(
            "  {} {}",
            theme.brand("EXECUTION PLAN //"),
            theme.bold(&self.title)
        );
        println!("  {}", theme.muted(&bar));
        println!(
            "    {} : {}",
            theme.muted("Target Task"),
            theme.bold(&self.task_id)
        );
        println!(
            "    {} : {}",
            theme.muted("Repository "),
            theme.bold(&self.repo_target)
        );
        println!();
        println!("  {}", theme.bold("Planned Steps:"));
        for step in &self.steps {
            let risk_str = match step.risk {
                RiskLevel::Low => theme.info("[LOW] "),
                RiskLevel::Medium => theme.warning("[MED] "),
                RiskLevel::High => theme.error("[HIGH]"),
            };
            println!(
                "    {} {} {: <30} {}",
                risk_str,
                theme.muted(Symbols::STEP),
                theme.bold(step.name),
                theme.muted(&format!("({})", step.tool_or_component))
            );
            println!("         {}", theme.muted(&step.description));
        }
        println!("  {}", theme.muted(&bar));
    }
}

/// Conversational intent parser.
#[derive(Debug, Default)]
pub struct IntentParser;

impl IntentParser {
    #[must_use]
    pub fn parse(input: &str) -> AgentIntent {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return AgentIntent::Unknown(String::new());
        }

        // 1. Slash-command shortcuts
        if let Some(cmd) = trimmed.strip_prefix('/') {
            return Self::parse_slash_command(cmd);
        }

        // 2. Natural language pattern matching
        Self::parse_natural_language(trimmed)
    }

    fn parse_slash_command(cmd_str: &str) -> AgentIntent {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        let command = parts.first().copied().unwrap_or("").to_lowercase();

        match command.as_str() {
            "help" | "h" | "?" => {
                let topic = parts.get(1).map(|s| (*s).to_string());
                AgentIntent::Help { topic }
            }
            "doctor" | "doc" => {
                let json = parts.contains(&"--json");
                AgentIntent::DoctorCheck { json }
            }
            "repro" | "run" | "fix" => {
                let task_id = parts.get(1).map(|s| s.to_uppercase());
                let live_browser = parts.contains(&"--live") || parts.contains(&"--browser");
                let headed = parts.contains(&"--headed");
                AgentIntent::ReproAndRepair {
                    task_id,
                    repo_path: None,
                    live_browser,
                    headed,
                    model: None,
                    clean_ref: None,
                }
            }
            "sweep" | "benchmark" => AgentIntent::BenchmarkSweep {
                bugs: vec![
                    "RF-01".to_string(),
                    "RF-02".to_string(),
                    "RF-03".to_string(),
                    "RF-04".to_string(),
                    "RF-05".to_string(),
                ],
            },
            "verify" | "receipt" => {
                let path = parts
                    .get(1)
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(".replayfix/audit_receipt.json"));
                AgentIntent::VerifyReceipt { path }
            }
            "inoculate" | "dream" => {
                let task_id = parts
                    .get(1)
                    .map(|s| s.to_uppercase())
                    .unwrap_or_else(|| "RF-01".to_string());
                AgentIntent::InoculateTask { task_id }
            }
            "package-pr" | "pr" => {
                let task_id = parts
                    .get(1)
                    .map(|s| s.to_uppercase())
                    .unwrap_or_else(|| "RF-01".to_string());
                AgentIntent::PackagePr { task_id }
            }
            "proof" | "export-proof" => AgentIntent::ExportProof,
            "status" => AgentIntent::ShowStatus,
            "history" | "log" => AgentIntent::ShowHistory,
            "resume" => AgentIntent::ResumeSession,
            "exit" | "quit" | "q" => AgentIntent::Exit,
            other => AgentIntent::Unknown(format!("/{other}")),
        }
    }

    fn parse_natural_language(text: &str) -> AgentIntent {
        let lower = text.to_lowercase();

        // Exit / Quit
        if matches!(lower.as_str(), "exit" | "quit" | "bye" | "goodbye") {
            return AgentIntent::Exit;
        }

        // Help
        if lower.contains("help")
            || lower.contains("how do i")
            || lower.contains("what can you do")
            || lower == "?"
        {
            return AgentIntent::Help { topic: None };
        }

        // Doctor / Environment
        if lower.contains("doctor")
            || lower.contains("environment check")
            || lower.contains("check dependencies")
        {
            return AgentIntent::DoctorCheck {
                json: lower.contains("json"),
            };
        }

        // Status / Context
        if lower == "status"
            || lower.contains("what is the status")
            || lower.contains("show session")
        {
            return AgentIntent::ShowStatus;
        }

        // History
        if lower == "history" || lower.contains("show history") || lower.contains("recent runs") {
            return AgentIntent::ShowHistory;
        }

        // Resume
        if lower.contains("resume") || lower.contains("continue session") {
            return AgentIntent::ResumeSession;
        }

        // Sweep
        if lower.contains("sweep")
            || lower.contains("benchmark all")
            || lower.contains("run all bugs")
        {
            return AgentIntent::BenchmarkSweep {
                bugs: vec![
                    "RF-01".to_string(),
                    "RF-02".to_string(),
                    "RF-03".to_string(),
                    "RF-04".to_string(),
                    "RF-05".to_string(),
                ],
            };
        }

        // Verify receipt
        if lower.contains("verify receipt") || lower.contains("check receipt") {
            return AgentIntent::VerifyReceipt {
                path: PathBuf::from(".replayfix/audit_receipt.json"),
            };
        }

        // Export proof
        if lower.contains("export proof") || lower.contains("proof bundle") {
            return AgentIntent::ExportProof;
        }

        // Permanent Inoculation
        if lower.contains("inoculate") || lower.contains("permanent test") {
            let task_id = Self::extract_task_id(&lower).unwrap_or_else(|| "RF-01".to_string());
            return AgentIntent::InoculateTask { task_id };
        }

        // Package PR
        if lower.contains("package pr")
            || lower.contains("pull request")
            || lower.contains("create pr")
        {
            let task_id = Self::extract_task_id(&lower).unwrap_or_else(|| "RF-01".to_string());
            return AgentIntent::PackagePr { task_id };
        }

        // Reproduction & Repair Intent
        if lower.contains("repro")
            || lower.contains("fix")
            || lower.contains("repair")
            || lower.contains("test")
            || lower.contains("run")
        {
            let task_id = Self::extract_task_id(&lower);
            let live_browser = lower.contains("live")
                || lower.contains("browser")
                || lower.contains("playwright")
                || lower.contains("headed");
            let headed = lower.contains("headed") || lower.contains("visible");

            if task_id.is_none() && (lower.contains("bug") || lower.contains("defect")) {
                return AgentIntent::ClarificationNeeded {
                    prompt: "Which bug scenario would you like to reproduce and repair?"
                        .to_string(),
                    candidates: vec![
                        "RF-01 (Cart Quantity NaN)".to_string(),
                        "RF-02 (Publish Quote Session Crash)".to_string(),
                        "RF-03 (Coupon Discount Math Overcharge)".to_string(),
                        "RF-04 (Negative Item Counter Underflow)".to_string(),
                        "RF-05 (Tab Switch State Mutation)".to_string(),
                    ],
                };
            }

            return AgentIntent::ReproAndRepair {
                task_id,
                repo_path: None,
                live_browser,
                headed,
                model: None,
                clean_ref: None,
            };
        }

        AgentIntent::Unknown(text.to_string())
    }

    fn extract_task_id(lower: &str) -> Option<String> {
        // Direct ID tokens
        if lower.contains("rf-01") || lower.contains("rf01") || lower.contains("cart") {
            Some("RF-01".to_string())
        } else if lower.contains("rf-02")
            || lower.contains("rf02")
            || lower.contains("quote")
            || lower.contains("publish")
        {
            Some("RF-02".to_string())
        } else if lower.contains("rf-03")
            || lower.contains("rf03")
            || lower.contains("discount")
            || lower.contains("coupon")
        {
            Some("RF-03".to_string())
        } else if lower.contains("rf-04")
            || lower.contains("rf04")
            || lower.contains("decrement")
            || lower.contains("negative")
        {
            Some("RF-04".to_string())
        } else if lower.contains("rf-05")
            || lower.contains("rf05")
            || lower.contains("tab")
            || lower.contains("switch")
        {
            Some("RF-05".to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slash_command_parsing() {
        assert_eq!(
            IntentParser::parse("/help"),
            AgentIntent::Help { topic: None }
        );
        assert_eq!(
            IntentParser::parse("/doctor --json"),
            AgentIntent::DoctorCheck { json: true }
        );
        assert_eq!(
            IntentParser::parse("/repro RF-01 --live --headed"),
            AgentIntent::ReproAndRepair {
                task_id: Some("RF-01".to_string()),
                repo_path: None,
                live_browser: true,
                headed: true,
                model: None,
                clean_ref: None,
            }
        );
        assert_eq!(IntentParser::parse("/exit"), AgentIntent::Exit);
    }

    #[test]
    fn test_natural_language_parsing() {
        assert_eq!(
            IntentParser::parse("Please reproduce and fix the cart quantity bug with live browser"),
            AgentIntent::ReproAndRepair {
                task_id: Some("RF-01".to_string()),
                repo_path: None,
                live_browser: true,
                headed: false,
                model: None,
                clean_ref: None,
            }
        );

        assert_eq!(
            IntentParser::parse("Can you run doctor to check my tools?"),
            AgentIntent::DoctorCheck { json: false }
        );

        assert_eq!(
            IntentParser::parse("run sweep across all 5 benchmarks"),
            AgentIntent::BenchmarkSweep {
                bugs: vec![
                    "RF-01".to_string(),
                    "RF-02".to_string(),
                    "RF-03".to_string(),
                    "RF-04".to_string(),
                    "RF-05".to_string(),
                ],
            }
        );
    }

    #[test]
    fn test_clarification_on_vague_input() {
        match IntentParser::parse("fix this bug") {
            AgentIntent::ClarificationNeeded { candidates, .. } => {
                assert_eq!(candidates.len(), 5);
            }
            other => panic!("expected ClarificationNeeded, got {:?}", other),
        }
    }
}

//! Conversational Agentic Terminal Workspace (REPL).
//!
//! Provides an interactive, keyboard-first conversational environment
//! integrating intent parsing, execution plans, tiered approvals,
//! real-time progress, diff review, and session persistence.

use crate::intent::{AgentIntent, ExecutionPlan, IntentParser};
use crate::permissions::{PermissionAction, PermissionDecision, RiskLevel};
use crate::session::AgentSession;
use crate::tui::brand::{render_compact_hud_emblem, render_hero_splash};
use crate::tui::dialog::prompt_selection;
use crate::tui::diff_view::{DiffReviewOutcome, DiffReviewer};
use crate::tui::motion::render_breathing_prompt;
use crate::tui::screen::ScreenGuard;
use crate::tui::spinner::StatusSpinner;
use crate::ui::{print_banner, print_field, Symbols, Theme};
use replayfix_core::engine::{HealOutcome, LiveBrowserOptions, MockHealer};
use replayfix_core::observer::{ChannelObserver, SubstrateEvent};
use replayfix_core::patch::UnifiedPatcher;
use replayfix_core::Fixture;
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::Arc;
use std::time::Duration;

/// Authentic conversational agent lifecycle state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    InputReceived,
    Planning,
    WaitingForApproval,
    Executing,
    Verifying,
    Completed,
}

/// Interactive agent conversational shell.
#[derive(Debug)]
pub struct AgentShell {
    pub session: AgentSession,
    pub theme: Theme,
    pub output_dir: PathBuf,
    pub repo_path: PathBuf,
    pub state: AgentState,
}

impl AgentShell {
    #[must_use]
    pub fn new(repo_path: PathBuf, output_dir: PathBuf, theme: Theme) -> Self {
        let session = AgentSession::load_or_init(&output_dir, &repo_path);
        Self {
            session,
            theme,
            output_dir,
            repo_path,
            state: AgentState::Idle,
        }
    }

    /// Run the interactive agent loop.
    pub fn run(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let (cols, _rows) = ScreenGuard::get_size();
        println!("{}", render_hero_splash(cols, &self.theme));
        print_banner(&self.theme, "Interactive Agent Workspace (v1.9.5)");
        println!(
            "  {} Welcome to ReplayFix. Describe your debugging goal or type '/help'.",
            self.theme.brand(Symbols::BULLET)
        );
        print_field(
            &self.theme,
            "Session ID",
            &self.session.record.session_id,
            None,
        );
        print_field(
            &self.theme,
            "Workspace",
            &self.repo_path.display().to_string(),
            None,
        );
        println!("  {}", self.theme.muted(&"─".repeat(60)));
        println!();

        let mut prompt_tick: u64 = 0;
        loop {
            self.state = AgentState::Idle;
            let task_prompt = self
                .session
                .record
                .active_task
                .as_deref()
                .unwrap_or("ready");
            let hud = render_compact_hud_emblem(self.theme.color_enabled);
            let glyph = render_breathing_prompt(prompt_tick, self.theme.color_enabled);
            prompt_tick = prompt_tick.wrapping_add(15);
            print!("  {} [{}] {} ", hud, self.theme.info(task_prompt), glyph);
            let _ = io::stdout().flush();

            let mut input = String::new();
            match io::stdin().lock().read_line(&mut input) {
                Ok(0) | Err(_) => {
                    let _ = self.session.save(&self.output_dir);
                    break;
                }
                Ok(_) => {}
            }

            let trimmed = input.trim();
            if trimmed.is_empty() {
                continue;
            }
            self.state = AgentState::InputReceived;

            let intent = IntentParser::parse(trimmed);
            match intent {
                AgentIntent::Exit => {
                    println!(
                        "  {} Saving session state and exiting. Goodbye!",
                        self.theme.brand(Symbols::SHIELD)
                    );
                    let _ = self.session.save(&self.output_dir);
                    break;
                }
                AgentIntent::Help { topic } => {
                    self.show_help(topic.as_deref());
                }
                AgentIntent::DoctorCheck { json } => {
                    crate::run_doctor(json, &self.theme)?;
                }
                AgentIntent::ShowStatus => {
                    self.show_status();
                }
                AgentIntent::ShowHistory => {
                    self.show_history();
                }
                AgentIntent::ResumeSession => {
                    self.handle_resume();
                }
                AgentIntent::VerifyReceipt { path } => {
                    println!(
                        "  {} Verifying receipt at '{}'...",
                        self.theme.info(Symbols::BULLET),
                        path.display()
                    );
                    crate::handle_verify_receipt(path, None, false, &self.theme);
                }
                AgentIntent::ExportProof => {
                    println!(
                        "  {} Exporting Judge Proof Portal package...",
                        self.theme.info(Symbols::BULLET),
                    );
                    crate::handle_export_proof(self.output_dir.clone(), false, &self.theme);
                }
                AgentIntent::InoculateTask { task_id } => {
                    crate::handle_inoculate(
                        task_id,
                        self.repo_path.display().to_string(),
                        self.output_dir.clone(),
                        false,
                        false,
                        None,
                        &self.theme,
                    );
                }
                AgentIntent::PackagePr { task_id } => {
                    crate::handle_package_pr_for_task(
                        &task_id,
                        &self.repo_path,
                        &self.output_dir,
                        true,
                        &self.theme,
                    );
                }
                AgentIntent::BenchmarkSweep { bugs } => {
                    self.handle_sweep(bugs);
                }
                AgentIntent::ClarificationNeeded { prompt, candidates } => {
                    if let Some(idx) = prompt_selection(&prompt, &candidates, &self.theme) {
                        let selected_token = &candidates[idx];
                        let id = selected_token.split_whitespace().next().unwrap_or("RF-01");
                        self.execute_repro_workflow(id, true, false);
                    }
                }
                AgentIntent::ReproAndRepair {
                    task_id,
                    live_browser,
                    headed,
                    ..
                } => {
                    let id = task_id.unwrap_or_else(|| "RF-01".to_string());
                    self.execute_repro_workflow(&id, live_browser, headed);
                }
                AgentIntent::Unknown(utterance) => {
                    println!(
                        "  {} I didn't recognize '{}'. Type '/help' to see what I can do.",
                        self.theme.warning(Symbols::WARN),
                        utterance
                    );
                }
            }

            let _ = self.session.save(&self.output_dir);
            println!();
        }

        Ok(())
    }

    fn show_help(&self, _topic: Option<&str>) {
        println!();
        let bar = "─".repeat(60);
        println!("  {}", self.theme.muted(&bar));
        println!(
            "  {} {}",
            self.theme.brand("ReplayFix Agentic Assistant //"),
            self.theme.bold("Command & Interaction Guide")
        );
        println!("  {}", self.theme.muted(&bar));
        println!("  {}", self.theme.bold("Conversational Requests:"));
        println!(
            "    {} \"Reproduce and repair RF-01 in metapulse\"",
            self.theme.muted("•")
        );
        println!(
            "    {} \"Fix the cart quantity NaN bug with live browser\"",
            self.theme.muted("•")
        );
        println!(
            "    {} \"Check environment dependencies (doctor)\"",
            self.theme.muted("•")
        );
        println!(
            "    {} \"Run benchmark sweep across all 5 defects\"",
            self.theme.muted("•")
        );
        println!(
            "    {} \"Verify receipt .replayfix/audit_receipt.json\"",
            self.theme.muted("•")
        );
        println!();
        println!("  {}", self.theme.bold("Shortcuts & Commands:"));
        println!(
            "    {} {: <16} : Causally reproduce and repair a defect",
            self.theme.brand("/repro <ID>"),
            "[--live] [--headed]"
        );
        println!(
            "    {} {: <16} : Check Git, Node.js, Playwright, FFmpeg prerequisites",
            self.theme.brand("/doctor"),
            ""
        );
        println!(
            "    {} {: <16} : Run 5-bug benchmark sweep on MetaPulse",
            self.theme.brand("/sweep"),
            ""
        );
        println!(
            "    {} {: <16} : Export Judge Proof Portal bundle & presets",
            self.theme.brand("/proof"),
            ""
        );
        println!(
            "    {} {: <16} : View active session context & permission grants",
            self.theme.brand("/status"),
            ""
        );
        println!(
            "    {} {: <16} : Review conversational turns and past actions",
            self.theme.brand("/history"),
            ""
        );
        println!(
            "    {} {: <16} : Resume last checkpointed workflow stage",
            self.theme.brand("/resume"),
            ""
        );
        println!(
            "    {} {: <16} : Save session and quit",
            self.theme.brand("/exit"),
            ""
        );
        println!("  {}", self.theme.muted(&bar));
    }

    fn show_status(&self) {
        println!();
        print_banner(&self.theme, "Session & Context Status");
        print_field(
            &self.theme,
            "Session ID",
            &self.session.record.session_id,
            None,
        );
        print_field(
            &self.theme,
            "Started At",
            &self.session.record.started_at,
            None,
        );
        print_field(
            &self.theme,
            "Last Active",
            &self.session.record.last_updated,
            None,
        );
        print_field(
            &self.theme,
            "Active Task",
            self.session.record.active_task.as_deref().unwrap_or("None"),
            None,
        );
        print_field(
            &self.theme,
            "Turn Count",
            &self.session.record.turns.len().to_string(),
            None,
        );
        if let Some(cp) = &self.session.record.checkpoint {
            print_field(
                &self.theme,
                "Checkpoint",
                &format!("{:?} (HEAD: {})", cp.current_stage, &cp.base_repo_head[..8]),
                Some("resumable"),
            );
        }
    }

    fn show_history(&self) {
        println!();
        print_banner(&self.theme, "Recent Conversation Turns");
        if self.session.record.turns.is_empty() {
            println!(
                "    {}",
                self.theme.muted("No recorded turns in this session.")
            );
            return;
        }
        for (i, turn) in self.session.record.turns.iter().enumerate() {
            println!(
                "    {} [{}] User : {}",
                self.theme.brand(Symbols::BULLET),
                i + 1,
                turn.user_input
            );
            println!(
                "       {} Agent: {}",
                self.theme.muted(Symbols::STEP),
                turn.agent_response
            );
        }
    }

    fn handle_resume(&mut self) {
        let resume_target = self
            .session
            .record
            .checkpoint
            .as_ref()
            .map(|cp| (cp.bug_id.clone(), cp.current_stage));

        if let Some((bug_id, stage)) = resume_target {
            println!(
                "  {} Resuming checkpoint for '{}' at stage {:?}",
                self.theme.brand(Symbols::SHIELD),
                bug_id,
                stage
            );
            self.execute_repro_workflow(&bug_id, true, false);
        } else {
            println!(
                "  {} No saved checkpoint found in this session.",
                self.theme.warning(Symbols::WARN)
            );
        }
    }

    fn handle_sweep(&mut self, bugs: Vec<String>) {
        let plan = ExecutionPlan {
            title: "MetaPulse Benchmark Sweep".to_string(),
            task_id: "ALL_5_BUGS".to_string(),
            repo_target: self.repo_path.display().to_string(),
            steps: vec![
                crate::intent::PlannedStep {
                    name: "1. Ephemeral Sandboxes",
                    description: "Spin up isolated scratch clones for RF-01 through RF-05"
                        .to_string(),
                    tool_or_component: "ScratchSandbox",
                    risk: RiskLevel::Low,
                },
                crate::intent::PlannedStep {
                    name: "2. Sweep Execution",
                    description: "Execute two-gate verification and bounded repair sweep"
                        .to_string(),
                    tool_or_component: "replayfix-core::run_sweep",
                    risk: RiskLevel::Medium,
                },
                crate::intent::PlannedStep {
                    name: "3. Merkle Attestation",
                    description: "Emit sweep_receipt.json signed with Ed25519 host key".to_string(),
                    tool_or_component: "RFC 6962 Merkle Tree",
                    risk: RiskLevel::Low,
                },
            ],
        };
        plan.render_card(&self.theme);

        print!("  Proceed with benchmark sweep? [y: Yes / n: Cancel]: ");
        let _ = io::stdout().flush();
        let mut answer = String::new();
        if io::stdin().lock().read_line(&mut answer).is_err()
            || !answer.trim().eq_ignore_ascii_case("y")
        {
            println!(
                "  {} Benchmark sweep cancelled.",
                self.theme.warning(Symbols::WARN)
            );
            return;
        }

        crate::handle_sweep(
            Some(bugs.join(",")),
            true,
            false,
            self.repo_path.display().to_string(),
            self.output_dir.clone(),
            false,
            None,
            None,
            &self.theme,
        );
    }

    /// Render aligned ASR voiceover transcript and keyframe metadata in the terminal.
    pub fn render_asr_keyframe_alignment(
        task_id: &str,
        click_x: i32,
        click_y: i32,
        accessible_name: &str,
        expected_text: &str,
        theme: &Theme,
    ) -> String {
        let line = format!(
            "[ASR @ 00:01.200s | frame_click_{}.png ({click_x},{click_y})] \"Clicking '{accessible_name}' fails to render '{expected_text}'\"",
            task_id.to_ascii_lowercase().replace('-', "_")
        );
        println!("  {} {}", theme.info(Symbols::BULLET), theme.muted(&line));
        line
    }

    /// Resolve an ambiguous DOM grounding tie (`|Δscore| < 0.05` / `500` bps) via the
    /// interactive terminal selection picker (`prompt_selection`).
    pub fn resolve_ambiguity_tie_interactive(
        tied: &[replayfix_core::locator::AmbiguousTieCandidate],
        theme: &Theme,
    ) -> Option<usize> {
        let options: Vec<String> = tied
            .iter()
            .map(|c| {
                format!(
                    "{} (score: {:.2}, tag: <{}>, name: '{}')",
                    c.selector,
                    f64::from(c.score_bps) / 10_000.0,
                    c.candidate.tag,
                    c.candidate.accessible_name
                )
            })
            .collect();
        prompt_selection(
            "Ambiguous DOM grounding tie detected (|Δscore| < 0.05). Select the intended target element:",
            &options,
            theme,
        )
    }

    fn execute_repro_workflow(&mut self, task_id: &str, live_browser: bool, headed: bool) {
        self.state = AgentState::Planning;
        let repo_display = self.repo_path.display().to_string();
        let plan = ExecutionPlan::for_repro(task_id, &repo_display, live_browser, headed);
        plan.render_card(&self.theme);

        self.state = AgentState::WaitingForApproval;
        print!("  Execute this plan? [y: Yes / n: Cancel]: ");
        let _ = io::stdout().flush();
        let mut confirm = String::new();
        if io::stdin().lock().read_line(&mut confirm).is_err()
            || !confirm.trim().eq_ignore_ascii_case("y")
        {
            println!(
                "  {} Plan aborted by operator. Repository untouched.",
                self.theme.warning(Symbols::WARN)
            );
            self.state = AgentState::Idle;
            return;
        }

        self.state = AgentState::Executing;
        self.session.record.active_task = Some(task_id.to_string());

        let fixture = match Fixture::for_task_in_repo(task_id, &self.repo_path) {
            Some(f) => f,
            None => {
                println!(
                    "  {} Unknown task '{}'. Supported tasks: {}",
                    self.theme.error(Symbols::CROSS),
                    task_id,
                    Fixture::supported_tasks().join(", ")
                );
                self.state = AgentState::Idle;
                return;
            }
        };

        let (default_cand, _) =
            replayfix_core::inoculate::InoculationGenerator::default_candidate_for_fixture(
                &fixture,
            );
        Self::render_asr_keyframe_alignment(
            fixture.task_id,
            fixture.click_x,
            fixture.click_y,
            &default_cand.accessible_name,
            fixture.expected_output_text,
            &self.theme,
        );

        let (tx, rx) = channel::<SubstrateEvent>();
        let observer = Arc::new(ChannelObserver::new(tx));

        let mut spinner = StatusSpinner::new(
            "ISOLATION",
            format!("Creating shadow scratch sandbox for {task_id}"),
        );
        spinner.tick(&self.theme);

        let opts = LiveBrowserOptions {
            headed,
            candidate_override: None,
            video_path: None,
            keyframe_dir: Some(self.output_dir.join("keyframes")),
            model_slug: None,
            repair_mode: None,
            clean_ref: None,
            verify_matrix: false,
            mock_regression: false,
            stress_quorum: true,
            mock_flaky: false,
            execution_mode: None,
            gateway_client: None,
            attached_url: None,
            observer: Some(observer),
        };

        spinner.set_status(
            "GATE 1 (RED)",
            "Running causal reproduction baseline in sandbox",
        );
        spinner.tick(&self.theme);

        let repo_path = &self.repo_path;
        let output_dir = &self.output_dir;
        let theme = &self.theme;
        let state_ref = &mut self.state;

        let outcome = std::thread::scope(|s| {
            let worker = s.spawn(|| {
                if live_browser {
                    MockHealer::run_live_browser_in_repo(
                        &fixture,
                        fixture.builtin_diff,
                        repo_path,
                        std::slice::from_ref(output_dir),
                        &opts,
                    )
                } else {
                    MockHealer::run_in_repo_with_stress(
                        &fixture,
                        fixture.builtin_diff,
                        repo_path,
                        std::slice::from_ref(output_dir),
                        false,
                        true,
                        false,
                    )
                }
            });

            loop {
                match rx.recv_timeout(Duration::from_millis(33)) {
                    Ok(evt) => {
                        match evt {
                            SubstrateEvent::PhaseStarted { phase, description } => {
                                *state_ref = AgentState::Verifying;
                                spinner.set_status(format!("{phase:?}"), description);
                            }
                            SubstrateEvent::GateEvaluated { gate, verdict, .. } => {
                                *state_ref = AgentState::Verifying;
                                spinner.set_status(
                                    format!("GATE {gate}"),
                                    format!("Evaluated verdict: {verdict:?}"),
                                );
                            }
                            SubstrateEvent::CdpTelemetry {
                                category, summary, ..
                            } => {
                                spinner.set_status(format!("CDP::{category}"), summary);
                            }
                            SubstrateEvent::StressQuorumEvaluated {
                                quorum_passed,
                                duration_ms,
                                ..
                            } => {
                                spinner.set_status(
                                    "GATE 3 (STRESS)",
                                    format!("Quorum passed={quorum_passed} ({duration_ms}ms)"),
                                );
                            }
                            _ => {}
                        }
                        spinner.tick(theme);
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        spinner.tick(theme);
                        if worker.is_finished() {
                            break;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }

            worker.join().unwrap_or_else(|_| {
                Err(replayfix_core::CoreError::Execution(
                    "Verification worker thread panicked".to_string(),
                ))
            })
        });

        match outcome {
            Ok(HealOutcome::Verified(receipt)) => {
                self.state = AgentState::Completed;
                spinner.complete(
                    &self.theme,
                    &format!("Defect {task_id} causally reproduced & verified repaired"),
                );
                let aria_snap = format!(
                    "- button \"{}\" [target={}]",
                    fixture.contract.target, fixture.target_path
                );
                self.session.record_verification_artifacts(
                    "VERIFIED_REPAIRED (Gate 1 RED -> Gate 2 GREEN)",
                    Some(aria_snap),
                    Some(fixture.builtin_diff.to_string()),
                );
                self.session.record_turn(
                    format!("reproduce and fix {task_id}"),
                    format!("Successfully verified repair for {task_id}"),
                    Some(task_id.to_string()),
                );
                let _ = self.session.save(&self.output_dir);

                // Launch interactive diff review
                self.state = AgentState::WaitingForApproval;
                let is_interactive = io::stdin().is_terminal();
                let review_choice = if is_interactive {
                    let mut reviewer = DiffReviewer::new(fixture.builtin_diff);
                    reviewer.run_interactive(&self.theme)
                } else {
                    println!("\n  Candidate patch verified:");
                    println!("{}", crate::approval::highlight_diff(fixture.builtin_diff));
                    print!("  Apply patch to base repo? [y/N]: ");
                    let _ = io::stdout().flush();
                    let mut ans = String::new();
                    let _ = io::stdin().lock().read_line(&mut ans);
                    if ans.trim().eq_ignore_ascii_case("y") {
                        DiffReviewOutcome::Approved
                    } else if ans.trim().eq_ignore_ascii_case("r") {
                        DiffReviewOutcome::Rejected
                    } else {
                        DiffReviewOutcome::Aborted
                    }
                };

                match review_choice {
                    DiffReviewOutcome::Approved => {
                        let action = PermissionAction::ApplyPatchToBaseRepo {
                            files: vec![fixture.target_path.to_string()],
                            insertions: 5,
                            deletions: 1,
                            patch_sha256: receipt.contract_id.clone(),
                        };

                        let grant = self.session.permissions.request_permission(
                            &action,
                            false,
                            is_interactive,
                            &self.theme,
                        );

                        if grant != PermissionDecision::Deny {
                            if let Err(e) = UnifiedPatcher::apply_to_worktree(
                                &self.repo_path,
                                fixture.builtin_diff,
                            ) {
                                println!(
                                    "  {} Failed applying patch to base repo: {e}",
                                    self.theme.error(Symbols::CROSS)
                                );
                            } else {
                                println!(
                                    "  {} Verified patch applied to base repository successfully.",
                                    self.theme.brand(Symbols::SHIELD)
                                );
                            }
                        } else {
                            println!(
                                "  {} Patch application denied by operator. Base repository remains unmodified.",
                                self.theme.warning(Symbols::WARN)
                            );
                        }
                    }
                    DiffReviewOutcome::Rejected => {
                        println!(
                            "  {} Candidate patch rejected during review. Base repository unmodified.",
                            self.theme.warning(Symbols::WARN)
                        );
                    }
                    DiffReviewOutcome::Aborted => {
                        println!(
                            "  {} Diff review aborted. Base repository unmodified.",
                            self.theme.muted("→")
                        );
                    }
                }
            }
            Ok(HealOutcome::Rejected(report)) => {
                spinner.fail(&self.theme, &format!("Repair rejected: {}", report.reason));
                self.session.record_turn(
                    format!("reproduce and fix {task_id}"),
                    format!("Repair rejected: {}", report.reason),
                    Some(task_id.to_string()),
                );
            }
            Err(e) => {
                spinner.fail(&self.theme, &format!("Engine error: {e}"));
            }
        }
    }
}

# Architecture Research & Technical Design Decisions: Release 1.4

**Feature**: Spec 140 — Agent Experience Revolution  
**Target Milestone**: Release 1.4.0  
**Authors**: Principal Architect, Terminal UI Specialist, Staff Engineer  
**Date**: 2026-11-01

---

## 1. Terminal UI Engine Evaluation

### Criteria
1. Cross-platform reliability (Windows Command Prompt, PowerShell, Windows Terminal, macOS Terminal, Linux xterm).
2. Pure Rust with zero C/foreign dependencies (`unsafe_code = "forbid"` compliant).
3. Ability to support both an interactive alternate-screen / in-place canvas mode and fall back cleanly to a linear streaming mode for dumb terminals and CI pipes (`!io::stdin().is_terminal()`).
4. Minimal dependency footprint and fast compilation.

### Options Considered
- **Option 1: Ink (React/Node.js)**:
  - *Evaluation*: Rejected. ReplayFix is a high-assurance pure Rust harness. Introducing Node.js/npm dependencies would violate Constitution Law X, complicate distribution, introduce runtime overhead, and create impedance mismatch with the Rust substrate.
- **Option 2: Textual (Python)**:
  - *Evaluation*: Rejected for the same reasons as Option 1.
- **Option 3: Ratatui + Crossterm**:
  - *Evaluation*: Ratatui is a full-featured terminal widget library. However, bringing in Ratatui pulls in multiple subcrates (layout, text, buffer, styling) that can conflict with or duplicate ReplayFix's existing, carefully calibrated `ui.rs` styling tokens and lightweight design system.
- **Option 4 (Selected): Native Modular Terminal Engine on `crossterm`**:
  - *Evaluation*: Adding `crossterm` (v0.28) provides cross-platform raw mode, cursor navigation, terminal resize detection (`crossterm::event::Event::Resize`), non-blocking keyboard polling (`poll(Duration)`), and alternate screen toggling.
  - On top of this, we build our own lightweight, modular `ui::terminal` component system (Panels, Banners, StepIndicators, DiffViewers, Menus, PromptBar) that directly reuses the existing `Theme`, `Symbols`, and `OutputMode` from `crates/replayfix-cli/src/ui.rs`.
  - This ensures zero bloat, complete control over terminal rendering, zero panics, and effortless integration with existing ANSI color logic.

---

## 2. Real-Time Substrate Event Bridge

### Problem
`replayfix-core` is currently synchronous and blocking. If `replayfix repro` runs inside an interactive agent shell, how does the UI render live spinners, active phase indicators, and tool execution badges without blocking on long-running calls?

### Solution: Event Listener Pattern with Observer Channel
1. In `replayfix-core`, define a lightweight, decoupled event emission trait:
   ```rust
   pub trait SubstrateObserver: Send + Sync {
       fn on_event(&self, event: SubstrateEvent);
   }
   ```
2. `SubstrateEvent` carries semantic execution occurrences:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub enum SubstrateEvent {
       PhaseStarted { phase: Phase, description: String },
       PhaseCompleted { phase: Phase, duration_ms: u64 },
       ToolInvoked { tool: String, command: String },
       ToolCompleted { tool: String, duration_ms: u64, success: bool },
       GroundingScore { selector: String, score: f64, margin: f64 },
       GateEvaluated { gate: u8, verdict: GateVerdict, duration_ms: u64 },
       RepairAttempt { attempt: u32, max_attempts: u32, model: String },
       LogMessage { level: LogLevel, message: String },
   }
   ```
3. In `replayfix-cli`, when running interactively, the engine executes on a worker thread while the main thread runs the reactive TUI event loop at $\sim 30\text{ FPS}$ (33 ms tick), processing incoming `SubstrateEvent`s via a crossbeam/mpsc channel.
4. When running in non-interactive / CI mode, a `NullObserver` or `StreamingStderrObserver` immediately formats events as standard sequential lines, preserving 100% backward compatibility.

---

## 3. Conversational Intent Parser & Orchestrator

### Objectives
1. Allow users to type unstructured natural language requests (e.g. *"Fix the cart bug in metapulse"*, *"Inspect RF-03 with live browser"*, *"Run doctor check"*, *"What is the status?"*).
2. Map natural language into structured `AgentIntent`:
   ```rust
   pub enum AgentIntent {
       ReproAndRepair {
           task_id: Option<String>,
           repo_path: Option<String>,
           live_browser: bool,
           headed: bool,
           model: Option<String>,
       },
       DoctorCheck,
       VerifyReceipt { path: PathBuf },
       BenchmarkSweep { bugs: Vec<String> },
       ShowStatus,
       ShowHistory,
       Help { topic: Option<String> },
       ResumeSession,
       Exit,
       Unknown(String),
   }
   ```
3. If parameters are missing (e.g. user specifies "fix bug" but no task ID), the conversational orchestrator prompts the user with matching options or defaults to the local repository context.
4. Always present a formatted **Execution Plan Card** before launching consequential operations, allowing the user to inspect, approve, or cancel.

---

## 4. Context-Aware Tiered Permission Model

### Risk Tiers
- **Low Risk (Auto-Granted)**:
  - Reading files within repo.
  - Inspecting git log and status.
  - Extracting video keyframes to scratch directory.
- **Medium Risk (Session or Prompt Gate)**:
  - Spawning ephemeral child dev server (binding dynamic port `:0`).
  - Outbound HTTPS network requests to Meta AI models (API usage / token billing).
- **High Risk (Affirmative Interactive Gate)**:
  - Mutating the base repository (`git apply` or file writes).
  - Deleting or modifying user configuration files.

### Security Guarantees
- The base repository is *always* protected during verification trials ($\Delta\text{repo} = 0$).
- Approvals can be granted for `Once`, for `Session`, or `Denied`.
- High-risk actions can *never* be silently auto-approved unless explicit `--yes` flag was passed at CLI invocation.

---

## 5. Interactive Diff & Review Architecture

### Design
- In-memory unified diff representation parsed by `approval::parse_diff_stats`.
- Virtualized scrolling viewport: if diff has 200 lines and terminal has 24 rows, viewport maintains `scroll_offset` and renders exactly the visible slice with header and navigation footer.
- Keyboard bindings: `j`/`Down`, `k`/`Up`, `PgUp`/`PgDn`, `Home`/`End`, `a` (Approve), `r` (Reject), `q` (Back).
- ANSI syntax highlighting preserved without escape injection vulnerabilities.

---

## 6. Backward Compatibility & CI Invariant

- If `!io::stdin().is_terminal()` or arguments are supplied (`replayfix repro ...`), the CLI bypasses the interactive agent loop entirely and executes the classic, deterministic linear path.
- All existing exit codes (0 to 41) remain intact.
- All 47 CLI integration tests and 194 core tests run without modification.
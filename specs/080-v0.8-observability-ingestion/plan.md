# 📐 Plan: ReplayFix v0.8.0 — Observability, GitHub Ingestion & Human Confirmation Gate

**Feature Identity**: `080-v0.8-observability-ingestion`  
**Target Release**: `v0.8.0`  
**Dependencies**: `replayfix-core`, `replayfix-cli`, `reqwest` (blocking/async), `regex`, `colored` or ANSI styling, `crossterm`/`dialoguer` or standard terminal TTY checks (`atty` / `std::io::IsTerminal`).

---

## 1. Architectural Strategy

ReplayFix v0.8.0 introduces three primary capabilities and one structural artifact enhancement:

```
                                      +------------------------------------+
                                      |  CLI: `replayfix repro / run`     |
                                      |  --issue <url> --trace --yes       |
                                      +-----------------+------------------+
                                                        |
                         +------------------------------+-------------------------------+
                         |                                                              |
                         v                                                              v
      +-------------------------------------+                        +-------------------------------------+
      |   GitHub Ingestion Module           |                        |   Playwright Trace Controller       |
      |   `replayfix-core::ingestion`       |                        |   `replayfix-core::trace`           |
      |   - Regex URL validator             |                        |   - Trace recording config          |
      |   - HTTP client (with rate-limit)   |                        |   - .zip artifact capture           |
      |   - Untrusted body framing          |                        |   - Auto-launcher (TTY-aware)       |
      |   - Step-to-reproduce parser        |                        +-------------------------------------+
      +-------------------------------------+                                           |
                         |                                                              |
                         v                                                              v
      +---------------------------------------------------------------------------------+
      |                           FSM Execution Substrate                               |
      |   Init -> Gate1Red -> Repair -> DiffGuard -> RegParity -> Gate2Green (x2)       |
      +---------------------------------------------------------------------------------+
                                                        |
                                                        v
                                      +-------------------------------------+
                                      |   Human Approval Gate               |
                                      |   `replayfix-cli::approval`         |
                                      |   - ANSI Syntax-highlighted Diff    |
                                      |   - TTY interactive prompt [y/N]    |
                                      |   - Fail-closed in non-TTY (no -y)  |
                                      |   - Approval audit logger           |
                                      +-----------------+-------------------+
                                                        |
                                                        v
                                      +-------------------------------------+
                                      |   Disaggregated Artifacts Store     |
                                      |   `.replayfix/artifacts/`           |
                                      |   - candidate.patch                 |
                                      |   - differential_diagnosis.json     |
                                      |   - approval_audit.json             |
                                      |   - issue_ingestion.json            |
                                      |   - playwright.trace.zip            |
                                      +-------------------------------------+
```

---

## 2. Component Design & Changes

### A. Ingestion Engine (`crates/replayfix-core/src/ingestion.rs`)
- Struct `GitHubIssueRef`: `owner: String`, `repo: String`, `issue_number: u64`.
- Struct `ParsedIssue`:
  - `url: String`
  - `title: String`
  - `body: String`
  - `author: String`
  - `labels: Vec<String>`
  - `extracted_steps: Vec<String>`
  - `untrusted_context_block: String`
- Validation logic:
  - Regex: `^https://github\.com/(?P<owner>[a-zA-Z0-9_.-]+)/(?P<repo>[a-zA-Z0-9_.-]+)/issues/(?P<number>\d+)/?$`
  - Rejects URLs pointing to `/pull/`, `/commit/`, non-https, or non-GitHub hosts.
- Retrieval logic:
  - HTTPS request to `https://api.github.com/repos/{owner}/{repo}/issues/{number}`.
  - Custom User-Agent: `ReplayFix-Harness/0.8.0`.
  - Timeout: 10 seconds. Response size capped at 512 KB.
  - Maps HTTP 403 / 429 into `CoreError::GitHubRateLimitExceeded`.
  - Maps HTTP 404 into `CoreError::GitHubIssueNotFound`.
- Step extraction:
  - Looks for sections matching `#+ *(Steps to Reproduce|Reproduction Steps|How to Reproduce|Repro)`.
  - Parses numbered list items (`1. ...`, `2. ...`) or bullet points (`- ...`, `* ...`).

### B. Trace Collector & Auto-Launcher (`crates/replayfix-core/src/trace.rs` & `crates/replayfix-core/src/browser.rs`)
- Struct `TraceConfig`:
  - `enabled: bool`
  - `output_path: PathBuf`
  - `screenshots: bool`
  - `snapshots: bool`
  - `sources: bool`
- Trace lifecycle:
  - In `BrowserController::launch()`: if trace enabled, set up tracing flags.
  - When test runs complete: write trace archive to `.replayfix/artifacts/<task_id>.trace.zip`.
- Auto-launcher function:
  - `pub fn launch_trace_viewer(trace_path: &Path, is_interactive: bool, is_ci: bool) -> Result<Option<u32>>`
  - If `!is_interactive || is_ci`: logs notice and returns `Ok(None)`.
  - If interactive: spawns `npx playwright show-trace <trace_path>` with `Stdio::null()` so it doesn't block the parent process or hang on exit.

### C. Human Approval Gate & Diff Highlighter (`crates/replayfix-cli/src/approval.rs`)
- Struct `ApprovalGate`:
  - `is_interactive: bool`
  - `auto_confirm: bool` (`--yes`)
- Functions:
  - `pub fn highlight_diff(diff_content: &str) -> String`: Formats diff lines with terminal color codes:
    - `---` and `+++` in bold
    - `@@ ... @@` in cyan
    - `+...` in green
    - `-...` in red
  - `pub fn request_approval(patch_content: &str, patch_hash: &str, files_changed: &[String], is_tty: bool, auto_confirm: bool) -> Result<ApprovalDecision, CliError>`
  - Evaluates:
    - If `auto_confirm`: returns `Approved { mode: AutoBypass }`
    - If `!is_tty`: returns error `NonInteractiveRefusal` (exit code 11)
    - If `is_tty`: prompts user `Apply patch to base repository? [y/N]: `
      - If input is `y` or `yes`: returns `Approved { mode: ManualAffirmative }`
      - Otherwise: returns `Rejected` (exit code 10)

### D. Disaggregated Artifacts Storage (`crates/replayfix-core/src/artifacts.rs`)
- Ensures directory `.replayfix/artifacts/` exists.
- Provides typed artifact writers:
  - `write_candidate_patch(dir, content)`
  - `write_approval_audit(dir, audit_record)`
  - `write_issue_ingestion(dir, issue_record)`
  - `write_differential_diagnosis(dir, diag_record)`
  - `write_regression_report(dir, reg_record)`

---

## 3. Risk & Blast Radius Analysis

1. **Risk: Trace Viewer hanging automated CI runners.**
   - *Mitigation*: Fail-safe detection of `CI=true` and non-TTY stdin/stdout. Never launch external processes unless explicitly interactive.
2. **Risk: Malicious code injection via GitHub Issue titles or bodies.**
   - *Mitigation*: Untrusted context framing (`<UNTRUSTED_ISSUE_BODY>`). The issue text is used purely as advisory natural language input for model guidance, never shell executed or evaluated.
3. **Risk: Patch applied to base repo without human consent.**
   - *Mitigation*: Default fail-closed prompt requiring affirmative input (`y`/`yes`). Unattended runs fail with exit code 11 unless explicit `--yes` flag is passed.

# Agent Living State — ReplayFix

## 1. Project Phase & Active Task
- **Active Milestone**: `v0.3.3` (v0.3 Release Quality Gate, Workspace Version Harmonization, BeautifulRust Verified)
- **Status**: Qualified & shipped; baseline locked for Spec 040 (`040-v0.4-playwright-cdp-grounding`)
- **Release process**: verify repo → specify → plan/tasks → implement → test → review → QA → security → docs/changelog/release notes → ship → verify remote. Release notes live in `docs/release-notes/vX.Y.Z.md`; every tag needs a changelog entry and release notes.

## 2. Verified Milestones & Evidence
- [x] **v0.0.0 Genesis**: two-crate workspace, platform abstraction, toolchain lock-in.
- [x] **v0.1.0 Walking Skeleton**: `doctor` (Git, Node.js, Playwright, FFmpeg), manifest ingestion, path-traversal rejection.
- [x] **v0.1.1 Hardening**: UNC, DOS-device and ADS path defenses, backslash normalization, platform-aware doctor advice, native test execution (no Cargo runner).
- [x] **v0.2.0 / v0.2.1 Mock Healer & Clean Architecture**:
  - Phase machine `Init → Gate1Red → Repair → Gate2Green` (+ `Rejected`), illegal transitions fail closed.
  - Gates evaluate the same contract (SHA-256 of canonical JSON); mismatch aborts.
  - Strict unified-diff patcher; anti-no-op guard; receipts (`audit_receipt.json` / `triage_report.json`) with stale-artifact removal.
  - Codified 10 Rust Engineering Principles (`docs/engineering-principles-rust.md`) with `unsafe_code = "forbid"` and `missing_debug_implementations = "warn"`.
- [x] **v0.3.0 Ephemeral Scratch Isolation**:
  - `WorktreeManifest`: dual-layer workspace integrity snapshot (Layer 1 recursive SHA-256 content tree + Layer 2 `git status --porcelain=v1 -uall`).
  - `ScratchSandbox`: clones `--repo` into an isolated OS temp directory, runs Gate 1 -> disk patch -> Gate 2 inside the sandbox, verifies $\Delta \text{repo} = 0$ (`BASE_REPO_DIRTY_MUTATION_DETECTED`, exit code 11 on violation), and tears down child process trees and read-only files with retry backoff.
  - `EphemeralPort::allocate()` (`127.0.0.1:0`) and `Platform::terminate_process_tree(pid)`.
  - Receipt schema `"0.3"` (`base_tree_sha256`, `sandbox_tree_sha256`, `base_repo_unmodified_verified`, `ephemeral_port`).
  - **Honesty note**: the gate oracle is still simulated (static inspection of `src/cart-summary.js` inside the scratch sandbox for `RF-01`). Real browser gates arrive in Spec 040.
- [x] **v0.3.1 & v0.3.2 Terminal UX & Interaction Engineering**:
  - Semantic styling, accessible symbols, output modes (`Human`, `Quiet`, `Machine`), deterministic color precedence (`--color auto|always|never`).
  - Security boundary trust cards ($\Delta \text{repo} = 0$, scratch isolation, test freeze), structured doctor diagnostics with remediation, and `--dry-run`.
- [x] **v0.3.3 Release Quality Gate & Version Harmonization**:
  - Workspace crate versions synchronized to `0.3.3` across `replayfix-core` and `replayfix-cli`.
  - Windows `.cmd` batch probe resolution for `npx` and `playwright`, resulting in 100% green `replayfix doctor` (Git 2.55, Node 24.19, Playwright 1.64.1, FFmpeg 15.2).
  - 96 automated tests passing (77 core, 17 integration, 2 UI); zero Clippy warnings; zero `.unwrap()` or `.expect()` in production source.

## 3. Toolchain & Environment
- Windows 11 (MSVC native), Rust 1.99.1, Node.js 24, Git 2.55, FFmpeg 15.2, Playwright 1.64.1 present and verified compatible by `replayfix doctor`.
- Windows Smart App Control can block fresh build outputs: use `scripts/sign-build-outputs.ps1` (see `docs/windows-code-signing.md`).

## 4. Deferred Items (documented, not hidden)
- `tracing_subscriber` is initialised but nothing logs yet; kept because later specs add structured logging.
- Patches touching test files are not rejected yet (Spec 070). Receipts are unsigned (Spec 090).
- Only `RF-01` has a built-in mock scenario; real browser scenarios arrive with Playwright CDP grounding (Spec 040).

## 5. v1.0.0 Readiness
- **Shipping status**: BLOCKED until Specs 040–090 are implemented and verified.

## 6. Next Immediate Task
- Advance to Spec 040: `040-v0.4-playwright-cdp-grounding`.

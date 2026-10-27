# Agent Living State — ReplayFix

## 1. Project Phase & Active Task
- **Active Milestone**: `v0.4.1` (`040-v0.4-playwright-cdp-grounding` — High-Risk Systems Gate Adversarial Hardening)
- **Status**: Qualified & shipped (`GO` after remediating 7 adversarial findings); baseline locked for Spec 050 (`050-v0.5-video-telemetry-ingestion`)
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
- [x] **v0.3.0 – v0.3.3 Ephemeral Scratch Isolation & Terminal UX**:
  - `WorktreeManifest`: dual-layer workspace integrity snapshot (Layer 1 recursive SHA-256 content tree + Layer 2 `git status --porcelain=v1 -uall`).
  - `ScratchSandbox`: clones `--repo` into an isolated OS temp directory, runs Gate 1 -> disk patch -> Gate 2 inside the sandbox, verifies $\Delta \text{repo} = 0$ (`BASE_REPO_DIRTY_MUTATION_DETECTED`, exit code 11 on violation), and tears down child process trees and read-only files with retry backoff.
  - `EphemeralPort::allocate()` (`127.0.0.1:0`) and `Platform::terminate_process_tree(pid)`.
  - Terminal UX (`OutputMode`, `--color`, `--quiet`, `--dry-run`, trust boundary cards, structured `doctor` diagnostics).
- [x] **v0.4.0 & v0.4.1 Playwright CDP Grounding, Live Browser Verification & Adversarial Systems Hardening**:
  - `LocatorResolver`: spatial-semantic candidate scoring in integer basis points (`0..=10_000` bps), **DOM Grounding Invariant** cross-checked against Playwright `ariaSnapshot`, `.first()` / `:nth-match` ban, duplicate-selector detection across all page candidates, `getByTestId` fallback when `aria_verified == false`, and **Fail-Closed Ambiguity Guard** ($|\Delta S| < 0.25$ / `< 2_500` bps -> `AMBIGUOUS_GROUNDING`, exit code `22`).
  - `SandboxServer` & `PlaywrightBridge`: serves `ScratchSandbox` over `127.0.0.1:<ephemeral_port>` with concurrent per-connection HTTP handling, path-traversal/drive-prefix/symlink defenses, and drives real Chromium over line-delimited JSON-lines stdio IPC with monotonic `request_id` correlation, non-blocking `stdout` reader thread (`recv_timeout`), continuous `stderr` drain ring buffer, browser WebSocket/target crash detection, explicit pre-click TOCTOU DOM verification (`DOM_NODE_DETACHED`), and idempotent single-termination process tree reaping.
  - `replayfix repro --live-browser` (and `--headed`): executes Gate 1 (RED) -> disk patch -> Gate 2 (GREEN) in live Chromium inside `ScratchSandbox` with $\Delta \text{repo} = 0$ verification and receipt schema `"0.4"` (`substrate: "playwright-cdp"`, `grounding_telemetry`).
  - 112 automated tests passing (91 core unit tests, 19 CLI integration tests, 2 CLI UI unit tests); zero Clippy warnings; zero `.unwrap()` or `.expect()` in production source.

## 3. Toolchain & Environment
- Windows 11 (MSVC native), Rust 1.99.1, Node.js 24, Git 2.55, FFmpeg 15.2, Playwright 1.64.1 present and verified compatible by `replayfix doctor`.
- Windows Smart App Control can block fresh build outputs: use `scripts/sign-build-outputs.ps1` (see `docs/windows-code-signing.md`).

## 4. Deferred Items (documented, not hidden)
- `tracing_subscriber` is initialised but nothing logs yet; kept because later specs add structured logging.
- Click hotspot coordinates `(160, 120)` in v0.4.x come from the `RF-01` fixture metadata; FFmpeg video keyframe extraction and SAM 3.1 perception arrive in Spec 050.
- Patches touching test files are not rejected yet (Spec 070). Receipts are unsigned (Spec 090).

## 5. v1.0.0 Readiness
- **Shipping status**: BLOCKED until Specs 050–090 are implemented and verified.

## 6. Next Immediate Task
- Advance to Spec 050: `050-v0.5-video-telemetry-ingestion`.

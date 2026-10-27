# Agent Living State — ReplayFix

## 1. Project Phase & Active Task
- **Active Milestone**: `v0.2.0` (Dual-gate mock healer, simulated oracle)
- **Status**: Ready for Spec 030 (`030-v0.3-ephemeral-scratch-isolation`)
- **Release process**: verify repo → specify → plan/tasks → implement → test → review → QA → security → docs/changelog/release notes → ship → verify remote. Release notes live in `docs/release-notes/vX.Y.Z.md`; every tag needs a changelog entry and release notes.

## 2. Verified Milestones & Evidence
- [x] **v0.0.0 Genesis**: two-crate workspace, platform abstraction, toolchain lock-in.
- [x] **v0.1.0 Walking Skeleton**: `doctor` (Git, Node.js, Playwright, FFmpeg), manifest ingestion, path-traversal rejection.
- [x] **v0.1.1 Hardening**: UNC, DOS-device and ADS path defenses, backslash normalization, platform-aware doctor advice, native test execution (no Cargo runner).
- [x] **v0.2.0 Mock Healer**:
  - Phase machine `Init → Gate1Red → Repair → Gate2Green` (+ `Rejected`), illegal transitions fail closed.
  - Gates evaluate the same contract (SHA-256 of canonical JSON); mismatch aborts.
  - Strict in-memory unified-diff patcher; anti-no-op guard; receipts (`audit_receipt.json` / `triage_report.json`) with stale-artifact removal.
  - `replayfix repro --mock-heal [--mock-patch builtin|noop|FILE] [--output-dir DIR]`.
  - 66 core tests + 15 CLI integration tests passing; clippy and fmt clean.
  - **Honesty note**: the oracle is simulated (static fixture inspection for `RF-01`). No browser, video, model or repository access yet.

## 3. Toolchain & Environment
- Windows 11 (MSVC native), Rust 1.99.1, Node.js 24, Git 2.55, FFmpeg present. Playwright not installed on the dev machine (`doctor` reports it as missing, which is expected).
- Windows Smart App Control can block fresh build outputs: use `scripts/sign-build-outputs.ps1` (see `docs/windows-code-signing.md`).

## 4. Deferred Items (documented, not hidden)
- `tracing_subscriber` is initialised but nothing logs yet; kept because later specs add structured logging.
- Patches touching test files are not rejected yet (Spec 070). Receipts are unsigned (Spec 090).
- Only `RF-01` has a built-in mock scenario; real scenarios arrive with browser gates (Spec 040).

## 5. v1.0.0 Readiness
- **Shipping status**: BLOCKED until Specs 030–090 are implemented and verified.

## 6. Next Immediate Task
- Advance to Spec 030: `030-v0.3-ephemeral-scratch-isolation`.

# Agent Living State — ReplayFix

## 1. Project Phase & Active Task
- **Active Milestone**: `v0.6.1` (`v0.6.1-patch-adversarial-hardening` — Preflight Reliability, Windows Reparse Point Security, Enclosing Scope Verification, and Lexical AST Panic Defense)
- **Status**: Qualified & ready for release tag `v0.6.1`; baseline locked for Spec 070 (`070-v0.7-differential-diagnosis-anticheat`)
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
  - `SandboxServer` & `PlaywrightBridge`: serves `ScratchSandbox` over `127.0.0.1:<ephemeral_port>` with concurrent per-connection HTTP handling, path-traversal/drive-prefix/symlink defenses, and drives real Chromium over line-delimited JSON-lines stdio IPC.
- [x] **v0.5.0 Multimodal Video Telemetry, SAM 3.1 & ASR Alignment**:
  - `run_bounded` subprocess supervisor, `VideoPreprocessor` (`ffprobe`, grayscale frames, keyframes, 24 kHz mono PCM WAV), `GeometricCalibration`, `RoiDiffer` ($120 \times 120\text{px}$ $L_1$ optical flow), `SamSegmenter` (`sam-3.1` `<|box;...|>` wire-token centroid snapping), and `AsrTranscriber` (`muse-voice-transcribe-1.0` `audioProcessedMs` locking + Zero-Audio Rule on `muse-spark-1.3`).
- [x] **v0.6.0 Meta Model Bounded Repair, Anti-Cheating Test Freeze & Deliberation Continuity**:
  - `TestFreezeGuard`: SHA-256 pre-repair freeze of all test/config files (`tests/**`, `*.spec.*`, `*.test.*`, `playwright.config.*`, `package.json`), rejecting any patch or disk mutation touching protected paths across case variants or `..` traversal with `TEST_MUTATION_PROHIBITED` (exit code `11`).
  - `MuseModelConfig`: validates `muse-spark-1.3` (`xhigh`), `muse-wonder-1.0` (`high`), `muse-spark-1.3-contributor` (`xhigh`), and fallback `meta-models/muse-spark-open-1026`, rejecting `muse-wonder-full-1.0` and `effort: "max"`.
  - `PromptBuilder` & `RepairRequest`: 24h static prefix caching (`replayfix-v0.6-harness-prefix-v1`), `<UNTRUSTED_CONTEXT>` tag escaping, and Meta Superpower #3 encrypted deliberation continuity (`reasoning.encrypted_content` + `previous_response_id`).
  - `StructuredRepairPatch`, `BlockPatcher`, & `SyntaxPreflight`: strict 5-field JSON Schema (`deny_unknown_fields`), exact single-match uniqueness (`0` -> `PATCH_TARGET_NOT_FOUND`, $\ge 2$ -> `AMBIGUOUS_PATCH`), `node --input-type=module --check` syntax preflight, 2-run consecutive Gate 2 stability (`gate2_runs_completed: 2`), bounded 3-attempt quarantine (`QUARANTINE_REPAIR_FAILED`, exit code `21`), and integer micro-dollar `TokenAccounting`.
  - 151 automated tests passing (125 core unit tests, 24 CLI integration tests, 2 CLI UI unit tests); zero Clippy warnings; zero `.unwrap()` or `.expect()` in production source.
- [x] **v0.6.1 Adversarial Security Hardening & Preflight Reliability**:
  - Windows `FILE_ATTRIBUTE_REPARSE_POINT` (NTFS junctions `mklink /J`) and canonical root containment verification in `write_file`.
  - DOS device stem isolation (`CON.txt.bak`) and trailing dot/space sanitization in `manifest.rs`.
  - `BlockPatcher` overlapping match detection and enclosing declaration boundary scope verification for `touched_symbols`.
  - Exact nano-dollar rate cards across multi-model fallback sequences in `TokenAccounting`.
  - Dynamic `syntax_preflight_passed` state reflection and exhaustive error matching in `engine.rs`.
  - CLI validation rejecting `--reasoning-effort max` with exit code `2`.
  - Lexical tokenizer for `check-panics.ps1` with self-test suite and crate-level AST denial lints.

## 3. Toolchain & Environment
- Windows 11 (MSVC native), Rust 1.99.1, Node.js 24.19.0, Git 2.55, FFmpeg 15.2, Playwright 1.64.1 present and verified compatible by `replayfix doctor`.
- Windows Smart App Control can block fresh build outputs: use `scripts/sign-build-outputs.ps1` (see `docs/windows-code-signing.md`).

## 4. Deferred Items (documented, not hidden)
- `tracing_subscriber` is initialised but nothing logs yet; kept because later specs add structured logging.
- Clean-branch (`clean_ref`) differential diagnosis and full pre-existing regression suite parity arrive in Spec 070.
- Receipts are unsigned until Spec 090 (RFC 6962 Merkle tree + Ed25519 signature).

## 5. v1.0.0 Readiness
- **Shipping status**: BLOCKED until Specs 070–090 are implemented and verified.

## 6. Next Immediate Task
- Advance to Spec 070: `070-v0.7-differential-diagnosis-anticheat`.

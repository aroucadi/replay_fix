# Agent Living State — ReplayFix

## 1. Project Phase & Active Task
- **Active Milestone**: `v0.8.0` (`080-v0.8-observability-ingestion` — Observability, GitHub Ingestion, Human Confirmation Gate & Disaggregated Artifacts)
- **Status**: Release Qualification Complete; ready for merge to `main` and tag `v0.8.0`.
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
- [x] **v0.6.2 Packaging Integrity, Test Resiliency & Sandbox Seeding Hardening**:
  - Git packaging: un-ignored `.specify/scripts/` in `.gitignore` and added root `scripts/check-panics.ps1` with dynamic repo root discovery.
  - Windows test runner: replaced fragile `.expect()` in `tests/cli.rs::code()` with `unwrap_or(-1)`.
  - Sandbox seeding: `collect_entries` and `copy_tree` in `worktree.rs` inspect `is_symlink_or_reparse_point(&symlink_meta)` to skip NTFS directory junctions (`mklink /J`).
  - Block patcher hygiene: `strip_comments_and_strings` in `repair.rs` blanks comments (`//`, `/* */`) and string literals before scope extraction.
- [x] **v0.7.0 Differential Diagnosis, Cryptographic Anti-Cheat, Regression Parity & Resumable Checkpoints**:
  - 6-state differential diagnosis matrix (`CAUSAL_DEFECT_CONFIRMED`, `INVALID_TEST_OVERFIT`, `QUARANTINE_INVERTED_BASELINE`, `UNATTRIBUTED_SYMPTOM`, `CLEAN_PASS_UNREPRODUCED`, `FLAKY_NON_DETERMINISTIC`).
  - Surgical `package.json` test script locking (`"scripts.test*"` locked byte-by-byte while allowing dependency updates; violations throw exit `11`).
  - Structured regression parity engine enforcing $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$, test suite preservation, and zero test skip inflation.
  - Durable resumable checkpoints (`session_state.json`) with git HEAD, manifest hash, and TTL staleness rejection.
  - Schema bumped to `"0.7"` with disaggregated stage artifacts (`candidate.patch`, `repro_spec.json`, `differential_diagnosis.json`, `regression_parity.json`).
  - New subcommand `freeze-spec --bug <ID>` and CLI flags `--clean-ref <REF>`, `--verify-matrix`, `--mock-pkg-test-mutation`.
  - 168 automated tests passing (139 core unit tests, 27 CLI integration tests, 2 CLI unit tests); zero panic violations; zero clippy warnings.
- [x] **v0.7.1 Spec 070 Hardening, Active Regression Parity & Inoculation**:
  - Dedicated `DiffGuard` module (`crates/replayfix-core/src/diffguard.rs`) enforcing runner configuration immutability and structural `package.json` script locking.
  - Active `RegressionEvaluator::evaluate` and test snapshot discovery wiring into `engine.rs`, rejecting test regressions with exit code 31 (`REGRESSION_DETECTED`).
  - `CheckpointManager` orchestrating monotonic stage advancement, durable disk persistence of `session_state.json`, and staleness invalidation.
  - `replayfix freeze-spec --bug <ID>` generating permanent Playwright regression tests (`tests/repro/rf_{id}_regression.spec.ts`) with immutable cryptographic aggregate SHA-256 headers.
  - Complete numbered fact artifacts suite (`01_candidate.diff`, `02_preflight.json`, `03_differential_diagnosis.json`, `04_regression_parity.json`, `04_audit_receipt.json`).
  - Real git ref resolution via `resolve_git_ref`.
  - Added CLI flag `--mock-regression` and integration test asserting exit code 31.
  - 174 automated tests passing (144 core unit tests, 28 CLI integration tests, 2 CLI unit tests); zero panic violations; zero clippy warnings.
- [x] **v0.8.0 Observability Suite, GitHub Issue Ingestion & Human Confirmation Gate**:
  - Playwright Trace Auto-Launcher (`--trace`) capturing `playwright.trace.zip` and spawning `npx playwright show-trace` detached, safely skipping in CI and non-interactive sessions.
  - Live GitHub Issue Ingestion (`--issue <url>`) via strict regex, HTTPS curl/gh CLI fetch, untrusted `<UNTRUSTED_ISSUE_METADATA>` isolation, and automated reproduction step extraction into task manifests.
  - Interactive Human Approval Gate (`--apply` / `--yes`) previewing syntax-highlighted ANSI unified diffs (+green, -red, @@ cyan) and prompting `[y/N]`, failing closed with exit code 13 (`NON_INTERACTIVE_REFUSAL`) in non-TTY sessions unless `--yes` is provided.
  - Base worktree mutation (`UnifiedPatcher::apply_to_worktree`) applying verified and approved patches with path traversal defenses.
  - Disaggregated fact artifacts persisted in `.replayfix/artifacts/` (`candidate.patch`, `approval_audit.json`, `issue_ingestion.json`, `differential_diagnosis.json`, `regression_parity.json`, `playwright.trace.zip`).
  - 185 automated tests passing (152 core unit tests, 33 CLI integration tests); zero Clippy warnings; zero panics in production source.

## 3. Toolchain & Environment
- Windows 11 (MSVC native), Rust 1.99.1, Node.js 24.19.0, Git 2.55, FFmpeg 15.2, Playwright 1.64.1 present and verified compatible by `replayfix doctor`.
- Windows Smart App Control can block fresh build outputs: use `scripts/sign-build-outputs.ps1` (see `docs/windows-code-signing.md`).

## 4. Deferred Items (documented, not hidden)
- `tracing_subscriber` is initialised but nothing logs yet; kept because later specs add structured logging.
- Receipts are unsigned until Spec 090 (RFC 6962 Merkle tree + Ed25519 signature).

## 5. v1.0.0 Readiness
- **Shipping status**: Progressing through roadmap. Next is Spec 090 (RFC 6962 Merkle receipt & benchmark suite).

## 6. Next Immediate Task
- Advance to Spec 090: `090-v0.9-merkle-receipt-benchmark-suite`.

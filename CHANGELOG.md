# Changelog

All notable changes to the **ReplayFix** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased] - Roadmap to v1.0.0

The full production release **v1.0.0** requires completing and verifying the following sequential architectural specifications:
- **Spec 060 (v0.6.0)**: Meta Muse Wonder bounded AST repair and localized patch synthesis.
- **Spec 070 (v0.7.0)**: Differential diagnosis, SHA-256 test freezing, and 2-run stability pass.
- **Spec 080 (v0.8.0)**: Observability suite, headed replay mode (`--headed`), and trace visualization.
- **Spec 090 (v0.9.0 & v1.0.0)**: RFC 6962 Merkle tree audit receipt generation, Ed25519 signing, and 5-bug benchmark sweep on `MetaPulse`.

---

## [0.8.1] - 2026-10-29

Security hardening, terminal injection defense, authenticated issue ingestion, and specification harmonization for v0.8.x baseline qualification freeze. Full notes: [docs/release-notes/v0.8.1.md](docs/release-notes/v0.8.1.md).

### Added
- **Adversarial Prompt Injection Defense (`replayfix-core::ingestion`)**: Entity-escapes embedded `<UNTRUSTED_ISSUE_METADATA>` and `</UNTRUSTED_ISSUE_METADATA>` delimiters within issue titles, reproduction steps, and raw markdown bodies.
- **Authenticated GitHub Issue Retrieval (`replayfix-core::ingestion`)**: Supports `GH_TOKEN` / `GITHUB_TOKEN` environment variables to prevent rate limiting (60/hr) and access private issues without credential leakage.
- **Terminal Injection Sanitization (`replayfix-cli::approval`)**: Neutralizes raw ANSI control characters (`\x1b` -> `^[`) in diff previews, preventing terminal hijack vulnerabilities.
- **Interactive EOF Fail-Closed Handling (`replayfix-cli::approval`)**: Ensures non-affirmative EOF triggers fail-closed `ManualNegative` rejection.

### Changed
- **Spec 080 Harmonization**: Synchronized acceptance criteria exit codes in `specs/080-v0.8-observability-ingestion/spec.md` with Section 4 mapping (12, 13, 23, 24).

---

## [0.8.0] - 2026-10-29

Observability suite, live GitHub issue ingestion, interactive human confirmation gate, and disaggregated evidence artifacts for Spec 080. Full notes: [docs/release-notes/v0.8.0.md](docs/release-notes/v0.8.0.md).

### Added
- **Playwright Trace Auto-Launcher (`--trace`)**: Captures complete Playwright execution traces and launches the interactive visual timeline scrubber (`npx playwright show-trace`) upon run completion, automatically skipping GUI spawn in headless CI and non-interactive sessions.
- **Live GitHub Issue Ingestion (`--issue <url>`)**: Secure retrieval of public GitHub issues via HTTPS curl or gh CLI, extraction of reproduction steps into task manifests, and enclosing untrusted descriptions in `<UNTRUSTED_ISSUE_METADATA>` blocks.
- **Interactive Human Approval Gate (`replayfix_cli::approval`)**: Syntax-highlighted ANSI candidate diff preview requiring affirmative operator confirmation (`[y/N]`) before modifying base working trees, failing closed with exit code 13 (`NON_INTERACTIVE_REFUSAL`) in non-TTY environments unless `--yes` is provided.
- **Worktree Patch Application (`replayfix_core::patch`)**: Added `UnifiedPatcher::apply_to_worktree` to apply approved candidate patches directly to repository working trees.
- **Disaggregated Fact Artifacts**: Persists `candidate.patch`, `approval_audit.json`, and `issue_ingestion.json` alongside `differential_diagnosis.json` and `regression_parity.json` in `.replayfix/artifacts/`.

---

## [0.7.1] - 2026-10-29

Security hardening, regression wiring, and spec alignment for Spec 070. Full notes: [docs/release-notes/v0.7.1.md](docs/release-notes/v0.7.1.md).

### Added
- **Dedicated DiffGuard Module (replayfix_core::diffguard)**: Formalized DiffGuard with structural AST package.json test script inspection and test runner configuration protection (CONFIG_MUTATION_PROHIBITED, exit code 11).
- **Active Regression Parity Engine (replayfix_core::regression)**: Wired RegressionEvaluator::evaluate and capture_suite_snapshot into the repair engine, enforcing F_patched \setminus F_baseline = \emptyset and rejecting regressions with exit code 31 (REGRESSION_DETECTED).
- **Resumable State CheckpointManager (replayfix_core::checkpoint)**: Implemented CheckpointManager orchestrating stage progression, disk persistence of session_state.json, and staleness invalidation.
- **Codebase Inoculation Generation (replayfix-cli freeze-spec)**: Generates permanent Playwright regression tests at tests/repro/rf_{id}_regression.spec.ts with cryptographic aggregate SHA-256 headers.
- **Disaggregated Fact Artifact Suite (replayfix_core::receipt)**: Emits numbered fact artifacts (01_candidate.diff, 02_preflight.json, 03_differential_diagnosis.json, 04_regression_parity.json, 04_audit_receipt.json).
- **New CLI Flag --mock-regression**: Simulates candidate patch introducing regressions and asserts exit code 31.

---

## [0.7.0] - 2026-10-28

Spec-driven release for Spec 070 (`070-v0.7-differential-diagnosis-anticheat`). Full notes: [docs/release-notes/v0.7.0.md](docs/release-notes/v0.7.0.md).

### Added
- **Formal 6-State Differential Diagnosis Matrix (`replayfix_core::fsm::matrix`)**:
  - Implemented `DifferentialMatrix` evaluating baseline, clean reference, and 2-run patch stability.
  - Distinguishes `CAUSAL_DEFECT_CONFIRMED` (exit 0), `INVALID_TEST_OVERFIT` (exit 10), `QUARANTINE_INVERTED_BASELINE` (exit 32), `UNATTRIBUTED_SYMPTOM` (exit 21), `CLEAN_PASS_UNREPRODUCED` (exit 10), and `FLAKY_NON_DETERMINISTIC` (exit 21).
- **Cryptographic Anti-Cheat & Surgical `package.json` DiffGuard (`replayfix_core::repair`)**:
  - SHA-256 pre/post freeze on all test files and runner configs.
  - Byte-level surgical immutability on `package.json` `"scripts.test*"` keys with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED")` (exit 11) while allowing safe non-test dependency/script edits.
- **Structured Regression Parity Engine (`replayfix_core::regression`)**:
  - `RegressionEvaluator` enforcing $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$, $T_{\text{baseline}} \setminus T_{\text{patched}} = \emptyset$, and non-increasing test skip counts with `REGRESSION_DETECTED` (exit 31).
- **Resumable Session Checkpointing (`replayfix_core::checkpoint`)**:
  - Checkpoint state model conforming to `session_state.schema.json` with git HEAD, manifest hash, and TTL staleness rejection.
- **Receipt Schema 0.7 & Disaggregated Artifacts (`replayfix_core::receipt`)**:
  - Bumped schema version to `"0.7"` incorporating `differential_diagnosis` and `regression_parity`.
  - Automatically writes `candidate.patch`, `repro_spec.json`, `differential_diagnosis.json`, and `regression_parity.json` to `<output_dir>/artifacts/`.
- **CLI Subcommands & Verification Flags (`replayfix-cli`)**:
  - Added `replayfix freeze-spec --bug <ID>` subcommand.
  - Added `--clean-ref <REF>`, `--verify-matrix`, and `--mock-pkg-test-mutation` flags.

---

## [0.6.2] - 2026-10-28

Surgical packaging, test runner resiliency, and sandbox seeding patch for Spec 060 (`060-v0.6-meta-model-bounded-repair`). Full notes: [docs/release-notes/v0.6.2.md](docs/release-notes/v0.6.2.md).

### Packaging & Hardening
- **Git Packaging & CI Tracking (`.gitignore`, `scripts/check-panics.ps1`)**:
  - Un-ignored `.specify/scripts/` in `.gitignore` and added root `scripts/check-panics.ps1` for clean CI reproducibility.
  - Dynamically detects the workspace root by walking parent directories looking for `Cargo.toml`.
- **Windows Test Runner Resiliency (`crates/replayfix-cli/tests/cli.rs`)**:
  - Replaced `.expect()` in test process exit code helper with `out.status.code().unwrap_or(-1)` to eliminate false runner panics on Windows.
- **NTFS Directory Junction Exclusion in Sandbox Seeding (`replayfix_core::worktree`)**:
  - In `collect_entries` and `copy_tree`, evaluated `is_symlink_or_reparse_point` to guarantee directory junctions (`mklink /J`) are excluded during initial workspace cloning.
- **Comment & String Literal Stripping in Scope Parser (`replayfix_core::repair::BlockPatcher`)**:
  - Implemented `strip_comments_and_strings` in `BlockPatcher::open_enclosing_declarations` to blank out comments and string literals, preventing false enclosing scope detection.

---

## [0.6.1] - 2026-10-28

Adversarial security hardening and preflight reliability patch for Spec 060 (`060-v0.6-meta-model-bounded-repair`). Full notes: [docs/release-notes/v0.6.1.md](docs/release-notes/v0.6.1.md).

### Security & Hardening
- **Windows Reparse Points & NTFS Junction Defense (`replayfix_core::worktree`)**:
  - Implemented `is_symlink_or_reparse_point` via `FILE_ATTRIBUTE_REPARSE_POINT` (`0x0400`) on Windows to block sandbox escapes via directory junctions (`mklink /J`).
  - Added canonical sandbox root containment verification in `write_file`.
- **DOS Device & Trailing Dot/Space Path Sanitization (`replayfix_core::manifest`)**:
  - Hardened `assert_safe_path` against multi-dot DOS reserved device patterns (e.g., `CON.txt.bak`) and stripped trailing dots/spaces from path components.
- **Enclosing Declaration Verification & Overlapping Matches (`replayfix_core::repair::BlockPatcher`)**:
  - Validated that `touched_symbols` are either defined in `replace_block` or belong to the active enclosing declaration scope.
  - Added overlapping match detection to guarantee `search_block` matches uniquely without ambiguous multi-match overlaps.
- **Fail-Closed Fallback Token Accounting (`replayfix_core::repair::TokenAccounting`)**:
  - Corrected nano-dollar cost accumulation across mixed model fallback attempts, preserving micro-USD integrity without integer rounding error.
- **Preflight & Engine Fail-Closed Error Handling (`replayfix_core::engine`)**:
  - Dynamic `syntax_preflight_passed` state reflection in telemetry.
  - Replaced partial `if let` blocks with exhaustive `match` arms for security violation reporting.
- **CLI & Zero-Panic Invariants (`replayfix-cli`)**:
  - Added CLI argument validation rejecting `--reasoning-effort max` with exit code `2`.
  - Enforced `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::unreachable))]` across crates.
  - Built lexical tokenizer into `.specify/scripts/check-panics.ps1` with `-SelfTest` suite.

---

## [0.6.0] - 2026-10-28

Delivers **Spec 060 (`060-v0.6-meta-model-bounded-repair`)**: Bounded autonomous Meta Muse repair (`muse-spark-1.3`, `muse-wonder-1.0`, `muse-spark-1.3-contributor`, and open-weights fallback `meta-models/muse-spark-open-1026`), anti-cheating SHA-256 test freeze (`TestFreezeGuard`), `<UNTRUSTED_CONTEXT>` prompt-injection data fencing with 24-hour static prefix caching (`replayfix-v0.6-harness-prefix-v1`), Meta Superpower #3 encrypted deliberation continuity (`reasoning.encrypted_content` + `previous_response_id`), strict 5-field JSON Schema output validation (`StructuredRepairPatch`), ambiguity-safe source-block patching (`BlockPatcher`), `node --input-type=module --check` syntax preflight (`SyntaxPreflight`), 2-run consecutive Gate 2 stability verification, bounded 3-attempt quarantine (`QUARANTINE_REPAIR_FAILED`, exit code `21`), and integer micro-dollar token accounting (`TokenAccounting`). Full notes: [docs/release-notes/v0.6.0.md](docs/release-notes/v0.6.0.md).

### Added
- **Anti-Cheating Test Freeze Guard (`replayfix_core::repair::TestFreezeGuard`)**:
  - Pre-repair SHA-256 capture of all protected test and runner config files (`tests/**`, `__tests__/**`, `*.spec.*`, `*.test.*`, `playwright.config.*`, `jest.config.*`, `vitest.config.*`, `tsconfig*.json`, `package.json`).
  - Rejects any patch targeting protected paths across case variants (`TESTS\Foo.SPEC.ts`) or path traversal (`src/../../tests/foo.spec.ts`) with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED")` (exit code `11`), and re-verifies SHA-256 parity on disk after every patch.
- **Meta Muse Model Routing, Prompt Isolation & Deliberation Continuity (`replayfix_core::repair`)**:
  - `MuseModelSlug`, `ReasoningEffort`, and `MuseModelConfig` supporting `muse-spark-1.3` (`effort: "xhigh"`), `muse-wonder-1.0` (`effort: "high"`), `muse-spark-1.3-contributor` (`effort: "xhigh"`), and `meta-models/muse-spark-open-1026`, while explicitly rejecting `muse-wonder-full-1.0` and `effort: "max"`.
  - `PromptBuilder`: Static system prefix (`replayfix-v0.6-harness-prefix-v1`, `24h` retention) and `<UNTRUSTED_CONTEXT>` escaping.
  - `RepairRequest` & `BoundedRepairClient`: Requests `include: ["reasoning.encrypted_content"]` and threads `previous_response_id` + `encrypted_content` across attempts 1..=3.
- **Strict JSON Schema, Ambiguity-Safe Block Patcher & Syntax Preflight**:
  - `StructuredRepairPatch::decode_strict` (`#[serde(deny_unknown_fields)]`, rejecting markdown code fences).
  - `BlockPatcher::apply`: Exact single-match uniqueness check (`0` matches -> `PATCH_TARGET_NOT_FOUND`, $\ge 2$ matches -> `AMBIGUOUS_PATCH`) and `touched_symbols` verification.
  - `SyntaxPreflight::check_file`: Pipes patched target source to `node --input-type=module --check` via `run_bounded` inside `ScratchSandbox`.
- **2-Run Gate 2 Stability, Quarantine Exit Code `21` & Receipt Schema `"0.6"`**:
  - Bumped `SCHEMA_VERSION` to `"0.6"`, added `TriageStatus::QuarantineRepairFailed` (`"QUARANTINE_REPAIR_FAILED"`, exit code `21`), and added `repair_telemetry: Option<RepairTelemetry>` and `token_accounting: Option<TokenAccounting>` to `AuditReceipt` and `TriageReport`.
  - Added `--model <MODEL>`, `--mock-failing-repair`, and `--mock-syntax-retry` flags to `replayfix repro` / `replayfix run`.
  - Remediated SpecKit workflow artifacts for `050` and generated full SpecKit suite for `060` (`spec.md`, `plan.md`, `tasks.md`, `research.md`, `data-model.md`, `quickstart.md`, `checklists/requirements.md`, `contracts/repair_response.schema.json`).
  - Expanded automated test suite to 151 tests (125 core unit tests + 24 CLI integration tests + 2 CLI UI unit tests).

---

## [0.5.0] - 2026-10-27

Delivers **Spec 050 (`050-v0.5-video-telemetry-ingestion`)**: Multimodal video telemetry ingestion, bounded FFmpeg/FFprobe subprocess execution, cursor-bounded $120 \times 120\text{px}$ ROI $L_1$ optical-flow frame differencing, geometric calibration across resolution/letterbox/DPR, Meta Superpower #1 (`sam-3.1`) wire-token centroid snapping, Meta Superpower #2 (`muse-voice-transcribe-1.0`) 24 kHz mono PCM `audioProcessedMs` narration alignment with strict Zero-Audio enforcement on `muse-spark-1.3`, and end-to-end CDP grounding integration. Full notes: [docs/release-notes/v0.5.0.md](docs/release-notes/v0.5.0.md).

### Added
- **Bounded Subprocess Supervisor (`replayfix_core::process`)**:
  - `run_bounded` and `BoundedOutput` enforcing wall-clock deadlines via `mpsc::Receiver::recv_timeout`, stdout size caps, stderr tail ring drains, and automatic process-tree termination (`Platform::terminate_process_tree`) on timeout or overflow.
- **FFmpeg Video Prober, Geometric Calibration & Cursor-Bounded ROI Differencer (`replayfix_core::video`)**:
  - `VideoPreprocessor`: Container safety validation (`.mp4`, `.webm`, `.mov`, $\le 120\text{s}$), `ffprobe` JSON metadata inspection (`VideoMetadata`), bounded grayscale frame extraction, keyframe PNG writer (`MAX_KEYFRAMES = 10` to `.replayfix/keyframes/`), 24 kHz mono 16-bit PCM WAV extraction (`pcm_s16le`), and deterministic demo MP4 synthesis (`synthesize_demo_video`).
  - `GeometricCalibration`: Integer-math coordinate transformation from video pixel space to CSS logical pixel space accounting for resolution scaling, letterbox/pillarbox padding, and device-pixel-ratio (`dpr_milli`).
  - `RoiDiffer`: $120 \times 120\text{px}$ cursor-bounded ROI $L_1$ frame differencer (`InteractionEvidence`) isolating the click timestamp `t_ms` and rough hotspot `(x_video, y_video)` while failing closed (`AMBIGUOUS_GROUNDING`, exit code `22`) on static/noisy frames or competing peaks within `< 2_500` bps ($|\Delta S| < 0.25$).
- **Meta Multimodal Perception Engine (`replayfix_core::perception`)**:
  - **Meta Superpower #1 (`sam-3.1` - `SamSegmenter`)**: Parses `<|box;x1=...;y1=...;x2=...;y2=...|><|mask;...|>` wire tokens, computes contour centroids $\left(\frac{x_1+x_2}{2}, \frac{y_1+y_2}{2}\right)$, enforces the `< 2_500` bps fail-closed ambiguity guard, and projects centroids into CSS logical pixels (`SamSnapResult`).
  - **Meta Superpower #2 (`muse-voice-transcribe-1.0` - `AsrTranscriber`)**: Enforces the strict **Zero-Audio Rule** (blocking `muse-spark-1.3` from receiving audio with `CoreError::SecurityViolation`), validates 24 kHz mono PCM streaming config (`PCM_24KHZ`, `ENDPOINTING`, `DELTA`, `emit_audio_progress: true`), and aligns developer narration using cumulative `audioProcessedMs` (`NarrationCue`).
  - `VideoTelemetry::ingest`: End-to-end multimodal ingestion pipeline wired into `MockHealer::run_live_browser_in_repo` and CLI `replayfix run --video assets/fixtures/rf01_demo.mp4 --repo .`.
- **Receipt Schema `"0.5"` & Demo Video Fixture**:
  - Bumped `SCHEMA_VERSION` to `"0.5"` and added `video_telemetry: Option<VideoTelemetry>` to `AuditReceipt` (100% float-free for RFC 8785 canonical JSON).
  - Added deterministic `assets/fixtures/rf01_demo.mp4` (`1280x720`, 24 kHz mono audio, UI click transition at `(100, 100)..(220, 140)`).
  - Expanded automated test suite to 138 tests (115 core unit tests + 21 CLI integration tests + 2 CLI UI unit tests).

---

## [0.4.1] - 2026-10-27

High-Risk Systems Gate adversarial hardening release for Playwright CDP grounding, child process timeout enforcement, protocol correlation, and DOM TOCTOU safety. Full notes: [docs/release-notes/v0.4.1.md](docs/release-notes/v0.4.1.md).

### Fixed & Hardened
- **Deadline-Bounded Non-Blocking Stdout Reader (`PlaywrightBridge`)**: Replaced blocking `BufReader::read_line` on the child's OS pipe with a dedicated reader thread (`spawn_stdout_reader`), `read_bounded_line` (`4 MiB` cap), and `mpsc::Receiver::recv_timeout` (`DEFAULT_BRIDGE_STARTUP_TIMEOUT = 20s`, `DEFAULT_BRIDGE_COMMAND_TIMEOUT = 15s`, `SHUTDOWN_COMMAND_TIMEOUT = 1.5s`), immediately poisoning and terminating the child process tree on timeout.
- **Continuous Background `stderr` Drain (`spawn_stderr_drainer`)**: Eliminated OS pipe buffer backpressure deadlocks when Node.js/Chromium writes >64 KiB to `stderr` and surfaced captured `stderr` context in all timeout, EOF, and protocol errors.
- **Monotonic Request Correlation (`request_id`)**: Added `request_id: u64` envelope correlation to `PlaywrightBridge` and `PLAYWRIGHT_BRIDGE_JS`, discarding late responses from prior requests (`resp_id < req_id`) and failing closed on mismatched future IDs (`resp_id > req_id`) or malformed JSON.
- **Browser WebSocket Disconnect, Target Crash & CDP Timeouts (`PLAYWRIGHT_BRIDGE_JS`)**: Added `browser.on('disconnected')`, `page.on('crash')`, `page.on('close')` tracking, `ensureActivePage()` health assertions, and `withTimeout` promise race guards across all Playwright/CDP calls.
- **DOM Node TOCTOU & Duplicate-Selector Ambiguity Guard (`LocatorResolver` & `CLICK_AND_OBSERVE`)**:
  - `LocatorResolver::resolve` now rejects selectors that match `> 1` grounded DOM candidates on the page (`Ambiguous grounding ... matches N distinct DOM elements`).
  - `LocatorResolver::synthesize_selector_for_grounding` and `execute_browser_gate` only synthesize/use `page.getByRole(...)` when `aria_verified == true` and fall back to `page.getByTestId(...)` otherwise.
  - `CLICK_AND_OBSERVE` verifies `locator.count() === 1` and non-zero `locator.boundingBox()` before clicking, surfacing `DOM_NODE_DETACHED` or `AMBIGUOUS_GROUNDING` immediately.
- **Idempotent Single-Termination Shutdown (`PlaywrightBridge::shutdown`)**: Added `shut_down` and `terminated` state guards and bounded `try_wait` reaping (`500ms`) so `Platform::terminate_process_tree` is called at most once per child instance.
- **Concurrent Per-Connection HTTP Handling (`SandboxServer`)**: Spawned short-lived per-connection threads with a 250ms loopback read timeout and 32 MiB file size cap (`MAX_HTTP_FILE_BYTES`) so idle TCP sockets cannot stall concurrent asset requests or server shutdown.

---

## [0.4.0] - 2026-10-27

Delivers **Spec 040 (`040-v0.4-playwright-cdp-grounding`)**: Playwright CDP spatial-semantic grounding, accessibility tree (`ariaSnapshot`) DOM Grounding Invariant verification, fail-closed ambiguity abstention (`AMBIGUOUS_GROUNDING`, exit code `22`), line-delimited JSON-lines stdio IPC (`PlaywrightBridge`), and live Chromium two-gate verification (`--live-browser` and `--headed`). Full notes: [docs/release-notes/v0.4.0.md](docs/release-notes/v0.4.0.md).

### Added
- **Spatial-Semantic Locator Grounding (`replayfix_core::locator`)**:
  - `BoundingBox`, `DomCandidate`, `GroundedTarget`, `GroundingTelemetry`, and `LocatorResolver`.
  - Integer basis-point composite scoring (`0..=10_000` bps) combining spatial containment (`4000` bps), semantic accessibility identity (`4000` bps), and DOM hierarchy specificity (`0..=2000` bps) for full RFC 8785 canonical JSON compatibility.
  - **DOM Grounding Invariant**: Cross-checks every synthesized semantic locator (`getByRole` / `getByTestId`) against Playwright's live `ariaSnapshot` YAML before execution.
  - **Strict `.first()` Ban**: Rejects positional disambiguators (`.first()`, `.nth()`, `:nth-match`).
  - **Fail-Closed Ambiguity Guard**: Abstains when the top two candidates score within $|\Delta S| < 0.25$ (`< 2500` bps), emitting `triage_report.json` with `status: "AMBIGUOUS_GROUNDING"` (exit code `22`).
- **Ephemeral Sandbox HTTP Server & Playwright Bridge (`replayfix_core::browser`)**:
  - `SandboxServer`: Serves isolated `ScratchSandbox` files over `127.0.0.1:<ephemeral_port>` with path-traversal, Windows drive-letter prefix, and symlink defenses (`403 Forbidden`).
  - `PlaywrightBridge`: Spawns a Node.js child process speaking flushed line-delimited JSON (`\n`) over `stdin`/`stdout` (`START_BROWSER`, `PROBE_PAGE`, `CLICK_AND_OBSERVE`, `SHUTDOWN`), automatically resolving Playwright from local `node_modules` or npm `_npx` caches and registering its process tree for deterministic cleanup.
- **Live Browser Two-Gate Verification (`--live-browser` & `--headed`)**:
  - `MockHealer::run_live_browser_in_repo` and `LiveBrowserOptions` in `replayfix_core::engine` driving Gate 1 (RED runtime `TypeError` in Chromium) -> disk patch in `ScratchSandbox` -> Gate 2 (GREEN DOM assertion in Chromium) under identical SHA-256 behavioral contracts and $\Delta \text{repo} = 0$ verification.
  - Added `--live-browser` and `--mock-ambiguous` flags to `replayfix repro` (with `--headed` launching visible Chromium).
- **Receipt Schema `"0.4"`**:
  - Bumped `SCHEMA_VERSION` to `"0.4"`, added `SUBSTRATE_LIVE_BROWSER` (`"playwright-cdp"`), `TriageStatus::AmbiguousGrounding` (exit code `22`), `TriageStatus::BridgeExecutionFailed` (exit code `20`), and `grounding_telemetry: Option<GroundingTelemetry>` on `AuditReceipt`.
- **Spec Kit Artifacts & Expanded Test Suite**:
  - Complete specification, plan, research, data model, quickstart, checklist, and task breakdown under `specs/040-v0.4-playwright-cdp-grounding/`.
  - Expanded automated test suite to 107 tests (86 core unit tests + 19 CLI integration tests + 2 CLI UI unit tests).

### Changed
- Bumped workspace crate versions in `replayfix-core` and `replayfix-cli` from `0.3.3` to `0.4.0`.
- Extended `Fixture::rf01()` with `index_html`, `click_x: 160`, `click_y: 120`, `viewport_width: 1280`, `viewport_height: 720`, and `expected_output_text: "Items: 0"`.

### Security
- `SandboxServer` binds strictly to `127.0.0.1` loopback and rejects path traversal (`../`), Windows drive-letter prefixes (`GET /C:/...`), and symlinks with `403 Forbidden`.
- `LocatorResolver` enforces the DOM Grounding Invariant against `ariaSnapshot` and bans `.first()` / `:nth-match` selectors.

---

## [0.3.3] - 2026-10-27

v0.3 Release Quality Gate qualification, workspace version harmonization, and BeautifulRust verification. Full notes: [docs/release-notes/v0.3.3.md](docs/release-notes/v0.3.3.md).

### Changed
- **Workspace Version Harmonization**: Synchronized crate versions to `v0.3.3` across `replayfix-core` and `replayfix-cli`.
- **Windows Command Resolution in `doctor`**: Resolved Windows batch wrapper probes (`.cmd`) for npm/npx-managed tools (`npx`, `playwright`), ensuring 100% accurate environment compatibility reporting.

### Verified
- **Full Test Parity**: 96 passing automated tests (77 core unit tests, 17 CLI integration tests, 2 UI painter tests) with 0 failures.
- **BeautifulRust Invariants**: Zero `.unwrap()` and zero `.expect()` across all production source files; `unsafe_code = "forbid"` and `missing_debug_implementations = "warn"` enforced.
- **Strict Clippy Cleanliness**: `cargo clippy --workspace --all-targets --all-features -- -D warnings` passing with zero warnings.
- **Interactive & Non-Interactive UX**: Verified `--dry-run`, deterministic color precedence (`--color auto|always|never`), structured diagnostic cards, and quiet mode.

---

## [0.3.2] - 2026-10-27

Production-grade terminal UX and interaction engineering milestone.

### Added
- **Output Modes & Capability Precedence**: Added --color <auto|always|never> and --quiet (OutputMode::Human, OutputMode::Quiet, OutputMode::Machine).
- **Security & Trust Boundaries**: User-visible verification boundary cards declaring repository immutability ($\\Delta\\text{repo} = 0$), scratch isolation status, and SHA-256 test freeze.
- **Actionable Diagnostic Architecture**: Overhauled replayfix doctor with structured findings, impacts, concrete remediation, and evidence.
- **Dry-Run & Preview**: Added --dry-run flag to replayfix repro.
- **Deterministic Operational Steps**: Labeled phase indicators ([RUN], [PASS], [FAIL], [SKIP]) with contextual metadata and audit summaries.

---

## [0.3.1] - 2026-10-27

Terminal UX and Interaction Engineering patch. Full notes: [docs/release-notes/v0.3.1.md](docs/release-notes/v0.3.1.md).

### Added
- Dedicated Terminal UI Module (replayfix_cli::ui).
- Semantic styling painter (Theme) and accessible symbols (Symbols: check, cross, warn, arrow).
- Standard color policy detection honoring NO_COLOR, CLICOLOR_FORCE, TERM=dumb, and std::io::IsTerminal.
- Doctor diagnostic badges and repro step progress indicators.

---

## [0.3.0] - 2026-10-27

Delivers **Spec 030 (`030-v0.3-ephemeral-scratch-isolation`)**: Ephemeral scratch workspace isolation, dual-layer repository integrity ($\Delta \text{repo} = 0$), dynamic ephemeral port allocation (`127.0.0.1:0`), and cross-platform process tree supervision with Windows file-lock defense. Full notes: [docs/release-notes/v0.3.0.md](docs/release-notes/v0.3.0.md).

### Added
- **Dual-Layer Workspace Integrity (`replayfix_core::worktree::WorktreeManifest`)**:
  - **Layer 1**: Recursive filesystem walk computing SHA-256 over every non-excluded regular file (`FileEntry { path, size, sha256 }`) and hashing the RFC 8785 canonical JSON array into `tree_sha256`.
  - **Layer 2**: Automatic capture and verification of `git status --porcelain=v1 -uall` when `.git` is present in the target repository.
  - Built-in exclusion of `.git`, `.replayfix`, `node_modules`, `target`, and custom `--output-dir` directories.
- **Ephemeral Scratch Sandbox (`replayfix_core::worktree::ScratchSandbox`)**:
  - Clones target repository working files into a unique OS temporary directory (`replayfix-sandbox-<task>-<pid>-<seq>`).
  - Executes Gate 1, disk-backed patch application, and Gate 2 strictly inside the scratch sandbox while keeping the user's base repository untouched ($\Delta \text{repo} = 0$).
  - Automatic `Drop` and `teardown()` lifecycle that terminates registered child process trees, clears read-only file attributes, and retries `remove_dir_all` with bounded backoff (`10ms`â€“`200ms`) to defeat Windows file locks (`os error 5` / `os error 32`).
- **Dynamic Ephemeral Port Allocation (`replayfix_core::platform::EphemeralPort`)**:
  - Binds `TcpListener::bind("127.0.0.1:0")` to allocate a guaranteed non-zero OS ephemeral port (`NonZeroU16`) per run, preventing `EADDRINUSE` collisions across concurrent sessions.
- **Cross-Platform Process Tree Termination (`Platform::terminate_process_tree`)**:
  - Terminates spawned child process trees via `taskkill /F /T /PID <pid>` on Windows and `pkill -KILL -P <pid>` / `kill -9 <pid>` on Unix, with self/zero PID safety guards and idempotent handling of already-exited processes.
- **Receipt Schema `"0.3"` & Base-Repo Mutation Triage**:
  - Added `TriageStatus::BaseRepoDirtyMutationDetected` (`"BASE_REPO_DIRTY_MUTATION_DETECTED"`, exit code `11`).
  - Added `base_tree_sha256`, `sandbox_tree_sha256`, `base_repo_unmodified_verified`, and `ephemeral_port` to `AuditReceipt`.
- **Engineering Principles Codification**:
  - Codified the 10 Rust Engineering Principles in [docs/engineering-principles-rust.md](docs/engineering-principles-rust.md) and enforced workspace-wide `unsafe_code = "forbid"` and `missing_debug_implementations = "warn"` lints.
- **Spec Kit Artifacts & Expanded Test Suite**:
  - Complete specification, plan, research, data model, quickstart, checklist, and task breakdown under `specs/030-v0.3-ephemeral-scratch-isolation/`.
  - Expanded automated test suite to 94 tests (77 core unit tests + 17 CLI integration tests).

### Changed
- `MockHealer` now executes Gate 1, patch application, and Gate 2 against real files on disk inside `ScratchSandbox` rather than purely in-memory strings, and verifies pre-run vs. post-run `WorktreeManifest` parity on `--repo`.
- `replayfix repro` now validates that `--repo` exists and is a directory (exiting with code `2` if missing) and prints scratch sandbox, ephemeral port, and tree digest telemetry during `--mock-heal`.
- Bumped `SCHEMA_VERSION` in `replayfix_core::receipt` from `"0.2"` to `"0.3"`.

### Security
- Added symlink escape detection and relative path enforcement inside `ScratchSandbox` (`resolve_safe_path` and `reject_symlink_chain`).
- Added a 32 MiB per-file safety limit (`MAX_WORKSPACE_FILE_BYTES`) during workspace snapshot and sandbox cloning to prevent memory exhaustion DoS.
- Unauthorized mutation of the base repository during a run fails closed with `BASE_REPO_DIRTY_MUTATION_DETECTED` (exit code `11`).

---

## [0.2.1] - 2026-10-27

Maintenance and clean-architecture refactoring release following the comprehensive engineering confidence audit. Full notes: [docs/release-notes/v0.2.1.md](docs/release-notes/v0.2.1.md).

### Added
- **SSOT String Representation for `TriageStatus`**: Added `as_str()` and `Display` implementation directly on `TriageStatus` in `replayfix_core::receipt`, eliminating runtime serde string extraction overhead.
- **SSOT CLI Exit Codes**: Introduced `replayfix_cli::exit_codes` (`SUCCESS = 0`, `DOCTOR_FAILED = 1`, `USAGE_OR_INPUT_ERROR = 2`, `INTERNAL_OR_SYSTEM_ERROR = 10`, `SECURITY_VIOLATION = 11`, `REPAIR_REJECTED = 30`) centralizing all CLI termination pathways.
- **Root Crate Re-exports**: Re-exported `TriageStatus`, `AUDIT_RECEIPT_FILE`, and `TRIAGE_REPORT_FILE` from `replayfix_core` crate root.
- **Automated Property Tests**: Added `test_triage_status_as_str_and_display` and `test_triage_status_serde_parity` ensuring exact equivalence between serde serialization and string representation.

### Changed
- Refactored CLI triage report output to use native `Display` formatting directly on `report.status`.
- Replaced magic exit code numbers across CLI handlers with named `exit_codes` constants.

### Removed
- Debloated redundant `status_name` helper in `replayfix-cli`.

---

## [0.2.0] - 2026-10-27

First release of the dual-gate loop. It runs against a **simulated oracle**: no browser, video, model or repository access is involved yet. Full notes: [docs/release-notes/v0.2.0.md](docs/release-notes/v0.2.0.md).

### Added
- **Dual-gate state machine** (`replayfix_core::fsm`): `Init` -> `Gate1Red` -> `Repair` -> `Gate2Green`, plus a terminal `Rejected`. Illegal transitions fail closed.
- **Mock healer engine** (`replayfix_core::engine`): Gate 1 must be RED before repair; Gate 2 re-evaluates the same behavioral contract, compared by SHA-256 of its canonical JSON. RED is valid only when setup, target and action pass before a failed assertion.
- **In-memory unified-diff patcher** (`replayfix_core::patch`): single file, exact offsets and context, no fuzz; rejects bad headers, mismatched hunk counts, overlaps, multi-file patches, creation and rename headers, and unsafe header paths.
- **Anti-no-op guard**: empty, whitespace-only and net-zero patches end as `REPAIR_NOOP_REJECTED`.
- **Receipts**: `audit_receipt.json` (`VERIFIED_REPAIRED`) or `triage_report.json`. A rejected run removes any stale receipt in the same folder.
- **CLI**: `replayfix repro --mock-heal`, with `--mock-patch <builtin|noop|FILE>` and `--output-dir <DIR>`. Exit codes: `0` verified, `2` usage or input error, `10` Gate 1 not RED or contract mismatch, `11` security violation, `30` repair rejected.
- Built-in scenario for task `RF-01` (unguarded `cart.items.length`).
- `scripts/sign-build-outputs.ps1` and a troubleshooting section for Smart App Control blocks on proc-macro DLLs and test executables.
- Spec Kit artifacts for Spec 020 (`specs/020-v0.2-mock-healer-two-gates/`).
- 66 core unit tests and 15 CLI integration tests.

### Changed
- `replayfix --version` and the banner now read the crate version instead of a hardcoded string.
- The `repro` success line without `--mock-heal` now says what to do next instead of promising Gate 1 execution.
- `doctor` result lines were reworded in plain language (exit codes unchanged).
- README rewritten around what is implemented and verified, with the simulated-oracle limit stated up front.

### Removed
- `scripts/cargo-runner.ps1`: orphaned since v0.1.1 removed the Cargo runner.
- Unused dependencies: `tokio`, `tracing`, `walkdir` and `async-trait` from `replayfix-core`; `serde` and `tracing` from `replayfix-cli`; `tokio` now enables only the features the CLI uses.
- Mojibake in `docs/windows-code-signing.md`.

### Security
- Patch header paths, `--mock-patch` and `--output-dir` go through the same path checks as manifests (traversal, UNC, DOS device, ADS), exit code 11.
- Patch errors truncate echoed content so a wrong `--mock-patch` file cannot leak its contents into logs or reports.
- `--mock-patch` files are limited to 1 MiB.

### Breaking changes
- None. Without `--mock-heal`, `repro` behaves as in v0.1.1 apart from the reworded status line. Library users who match `CoreError` exhaustively must handle the new `CoreError::Patch` variant.

### Known limitations
- Only `RF-01` has a built-in scenario; the oracle inspects a bundled snippet, so receipts prove the state machine and receipt format, not real browser behavior (real gates: v0.4).
- No repository isolation yet (v0.3), no rejection of patches that touch test files and no two-run stability pass (v0.7).
- Receipts are not signed or Merkle-bound (v0.9).
- On Windows with Smart App Control, new builds may need `scripts/sign-build-outputs.ps1` before tests can launch the CLI.

---

## [0.1.1] - 2026-10-26

### Security Hardening (CSO Comprehensive Audit)
- **UNC Network Injection Defense**:
  - Rejection of UNC network paths (`\\host\share` and `//host/share`) across repository, recording, and manifest file paths (`TaskManifest::from_file`) to prevent SMB credential coercion and NTLM reflection attacks.
- **Multi-Component DOS Device Sanitization**:
  - Rejection of Windows reserved device filenames (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`) across all path components (including subdirectories like `projects/CON/repo`), eliminating descriptor hijacking and process hangs.
- **Alternate Data Stream (ADS) Defense**:
  - Rejection of unauthorized NTFS stream delimiters (`:`) in paths outside of valid Windows drive prefixes, blocking hidden payloads and stream smuggling.
- **Environment Adapter Sanitization**:
  - Strict regex enforcement on environment variable names (`port_env_var` matching `^[a-zA-Z_][a-zA-Z0-9_]*$`).
  - Validation of `healthcheck_path` against CRLF injection, control characters, and whitespace to prevent HTTP header injection.
  - Bounded verification of `startup_timeout_ms` (100ms to 600,000ms).
- **Regex DoS & Churn Elimination**:
  - Transitioned all regex patterns in `doctor` and `manifest` parsing to static `std::sync::OnceLock` singletons, eliminating runtime regex recompilation overhead.
- **Cross-Platform Path Normalization & Portability**:
  - Normalized Windows backslashes (`\`) to forward slashes (`/`) during path decomposition in `assert_safe_path`, ensuring path traversal and DOS device checks execute identically across Linux, macOS, and Windows.
  - Refined `OsFamily` taxonomy with discrete `MacOS`, `Linux`, `OtherUnix`, and `Windows` variants, along with `is_windows()` and `is_unix()` helpers.
  - Implemented platform-adaptive installation suggestions in `replayfix doctor` (`brew` for macOS, `apt`/`dnf` for Linux, `winget` for Windows).
- **Windows Code Signing & SAC Architecture**:
  - Replaced machine-specific Cargo runner overrides with native execution and standalone developer scripts (`scripts/setup-dev-cert.ps1` and `scripts/sign-binary.ps1`).
  - Eliminated PowerShell execution wrapper overhead and associated Smart App Control toast warnings during local testing.
  - Added comprehensive architectural and troubleshooting documentation in `docs/windows-code-signing.md`.
- **Automated Regression Tests**:
  - Added comprehensive automated security and cross-platform tests (including `test_security_violation_on_unc_path`, `test_security_violation_on_dos_device`, `test_security_violation_on_intermediate_dos_device`, `test_security_violation_on_ads_colon`, `test_from_file_rejects_unc_manifest_path`, `test_environment_adapter_security_validation`, `test_windows_backslash_path_normalization`, `test_security_violation_on_backslash_path_traversal`, `test_security_violation_on_backslash_dos_device`, `test_platform_os_family`, and `test_platform_remediation_suggestions`). All 17 workspace tests pass cleanly.

---

## [0.1.0] - 2026-10-26

### Added
- **Multi-Dependency Environment Diagnostics (`replayfix doctor`)**:
  - Deterministic probing and semantic version validation for Git ($\ge 2.30.0$), Node.js ($\ge 18.0.0$), Playwright ($\ge 1.40.0$), and FFmpeg ($\ge 4.4.0$).
  - Actionable remediation advice and installation commands on missing or incompatible dependencies.
  - Machine-readable structured JSON output (`--json`) for CI pipelines.
- **Canonical Task Manifest Ingestion (`replayfix repro`)**:
  - Strongly typed domain entities (`TaskManifest`, `EnvironmentAdapter`) strictly conforming to `task_manifest.schema.json`.
  - Dual ingestion modes: direct command-line arguments (`--task-id`, `--title`, `--repo`, `--bug-ref`, `--clean-ref`, `--video`, `--server-cmd`, etc.) or manifest file ingestion via `--manifest <path>`.
  - Visible browser replay flag (`--headed`) and trace capture flag (`--trace`).
- **Security Boundary Enforcement**:
  - Path canonicalization and path traversal detection on `--repo`, `--video`, and manifest paths, terminating attempts with `SECURITY_VIOLATION` (exit code 11).
- **Core Library Expansion**:
  - `replayfix_core::doctor` module with version comparison and dependency checkers.
  - `replayfix_core::manifest` module with serde schemas and validation rules.
  - Extended error taxonomy in `replayfix_core::error::CoreError`.

### Known Boundaries & Limitations
- Gate 1 browser reproduction and dual-worktree isolation are scheduled for upcoming vertical micro-slices (`020` / `030`). In this release, `replayfix repro` validates and hydrates the canonical task manifest, confirming readiness for baseline reproduction.

---

## [0.0.0] - 2026-10-26

### Added
- Pristine 2-crate root Cargo workspace: `crates/replayfix-core` (library) and `crates/replayfix-cli` (binary).
- Cross-platform native toolchain lock-in (`replayfix_core::Platform`) supporting Windows MSVC, macOS, and Linux.
- Structured error handling with `thiserror` (`CoreError`).
- Spec Kit governance configuration and baseline state journal.

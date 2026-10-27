# Changelog

All notable changes to the **ReplayFix** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased] - Roadmap to v1.0.0

The full production release **v1.0.0** requires completing and verifying the following sequential architectural specifications:
- **Spec 040 (v0.4.0)**: Playwright CDP spatial grounding, `ariaSnapshot` inspection, and coordinate snapping.
- **Spec 050 (v0.5.0)**: Video telemetry ingestion, FFmpeg keyframe extraction, and SAM 3.1 centroid alignment.
- **Spec 060 (v0.6.0)**: Meta Muse Wonder bounded AST repair and localized patch synthesis.
- **Spec 070 (v0.7.0)**: Differential diagnosis, SHA-256 test freezing, and 2-run stability pass.
- **Spec 080 (v0.8.0)**: Observability suite, headed replay mode (`--headed`), and trace visualization.
- **Spec 090 (v0.9.0 & v1.0.0)**: RFC 6962 Merkle tree audit receipt generation, Ed25519 signing, and 5-bug benchmark sweep on `MetaPulse`.

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
  - Automatic `Drop` and `teardown()` lifecycle that terminates registered child process trees, clears read-only file attributes, and retries `remove_dir_all` with bounded backoff (`10ms`–`200ms`) to defeat Windows file locks (`os error 5` / `os error 32`).
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

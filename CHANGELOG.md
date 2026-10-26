# Changelog

All notable changes to the **ReplayFix** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased] - Roadmap to v1.0.0

The full production release **v1.0.0** requires completing and verifying the following sequential architectural specifications:
- **Spec 020 (v0.2.0)**: Dual-gate FSM state machine (`Init` $\rightarrow$ `Gate1Red` $\rightarrow$ `Repair` $\rightarrow$ `Gate2Green`).
- **Spec 030 (v0.3.0)**: Ephemeral dual-worktree repository isolation ($\Delta\text{repo} = 0$) and dynamic port allocation (`:0`).
- **Spec 040 (v0.4.0)**: Playwright CDP spatial grounding, `ariaSnapshot` inspection, and coordinate snapping.
- **Spec 050 (v0.5.0)**: Video telemetry ingestion, FFmpeg keyframe extraction, and SAM 3.1 centroid alignment.
- **Spec 060 (v0.6.0)**: Meta Muse Wonder bounded AST repair and localized patch synthesis.
- **Spec 070 (v0.7.0)**: Differential diagnosis, SHA-256 test freezing, and 2-run stability pass.
- **Spec 080 (v0.8.0)**: Observability suite, headed replay mode (`--headed`), and trace visualization.
- **Spec 090 (v0.9.0 & v1.0.0)**: RFC 6962 Merkle tree audit receipt generation, Ed25519 signing, and 5-bug benchmark sweep on `MetaPulse`.

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
- **Windows Enterprise Code Integrity / SAC Compatibility**:
  - Added `.cargo/config.toml` target runner configuration to support local development code-signing on systems enforcing Windows Smart App Control (SAC).
- **Automated Regression Tests**:
  - Added 6 automated security tests (`test_security_violation_on_unc_path`, `test_security_violation_on_dos_device`, `test_security_violation_on_intermediate_dos_device`, `test_security_violation_on_ads_colon`, `test_from_file_rejects_unc_manifest_path`, and `test_environment_adapter_security_validation`). All 13 workspace tests pass cleanly.

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

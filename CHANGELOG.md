# Changelog

All notable changes to the **ReplayFix** project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

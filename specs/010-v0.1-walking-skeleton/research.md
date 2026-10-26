# Research & Architecture Decisions: 010-v0.1-walking-skeleton

**Feature**: Environment Diagnostics & Repro Manifest Ingestion  
**Date**: 2026-10-26  

---

## 1. Multi-Dependency Probing Architecture

- **Decision**: Implement dependency probes in `replayfix_core::doctor` executing each binary via `Platform::create_command` with `--version` flags:
  - **Git**: `git --version` $\rightarrow$ parse `git version X.Y.Z` (minimum 2.30.0).
  - **Node.js**: `node --version` $\rightarrow$ parse `vX.Y.Z` (minimum 18.0.0).
  - **Playwright**: Check CLI / npx resolution via `npx --no-install playwright --version` or probing local `node_modules` (minimum 1.40.0).
  - **FFmpeg**: `ffmpeg -version` $\rightarrow$ parse `ffmpeg version X.Y.Z` (minimum 4.4.0).
- **Rationale**:
  - Deterministic subprocess execution avoids guessing or environment variable inspection.
  - Fail-closed evaluation ensures that if a tool fails to execute or times out, it is treated as `NotFound` or `Incompatible` with exact remediation advice.
- **Alternatives Considered**:
  - *Hardcoding assumption that dependencies exist*: Rejected; violated Law I and caused flaky reproduction in unfamiliar environments.

---

## 2. Canonical Task Manifest Mapping

- **Decision**: Model `TaskManifest` in `replayfix_core::manifest` strictly matching `contracts/task_manifest.schema.json`:
  - Required fields: `task_id`, `title`, `repo_path`, `bug_ref`, `video_path`.
  - Optional fields: `clean_ref`, `audio_path`, `expected_behavior`, `headed` (bool, default false).
  - Nested `environment_adapter`: `server_cmd`, `port_env_var` (default "PORT"), `healthcheck_path` (default "/api/health"), `startup_timeout_ms` (default 10000).
- **Rationale**:
  - Dual ingestion path: The CLI can hydrate a manifest directly from CLI arguments (`--task-id`, `--repo`, `--bug-ref`, `--video`, etc.) OR load an existing JSON file via `--manifest <path>`.
  - Serde deserialization guarantees schema validation before any child processes or browsers are spawned.

---

## 3. Path Sanitization & Traversal Defense

- **Decision**: Sanitize all incoming paths (`repo_path`, `video_path`, `manifest`) by canonicalizing and verifying that relative path components do not contain parent directory traversal escapes (`..`) outside authorized boundaries.
- **Rationale**:
  - Satisfies Law III (Workspace Isolation) and passes adversarial test scenarios for path traversal containment.

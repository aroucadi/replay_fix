# Feature Specification: 010-v0.1-walking-skeleton — Environment Diagnostics & Repro Manifest Ingestion

**Feature Branch**: `release/v0.1.0-walking-skeleton`  
**Created**: 2026-10-26  
**Status**: Ready for Planning  
**Input**: Multi-dependency environment diagnostics (`replayfix doctor`) covering Git, Node.js, Playwright, and FFmpeg; initial `replayfix repro` CLI flags and parameters strictly adhering to canonical `task_manifest.schema.json`.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Multi-Dependency Environment Diagnostics (Priority: P1)

As an engineer preparing to reproduce visual defects or run ReplayFix in CI, I want to execute `replayfix doctor` to verify that all required host tools (Git, Node.js, Playwright, and FFmpeg) are installed, compatible, and ready, so that reproduction runs do not fail mid-flight due to missing system binaries.

**Why this priority**: Without verified system dependencies, reproduction, browser control, and video keyframe extraction cannot run deterministically. Early fail-closed diagnostics prevent opaque downstream failures.

**Independent Test**: Execute `replayfix doctor`. The command inspects each required tool, parses installed versions, compares them against minimum criteria, prints a diagnostic matrix, and exits with 0 if all dependencies are satisfied or non-zero if critical dependencies are missing, displaying actionable install commands.

**Acceptance Scenarios**:
1. **Given** a host with Git, Node.js, Playwright, and FFmpeg installed and accessible in PATH, **When** the user runs `replayfix doctor`, **Then** the terminal renders a structured table displaying each dependency with its resolved version, a green `OK` status, and exits with code 0.
2. **Given** a host missing FFmpeg or Playwright, **When** the user runs `replayfix doctor`, **Then** the missing tool is clearly flagged with status `MISSING`, the output includes exact platform-specific remediation commands (e.g. package manager installation steps), and the process exits with code 1.
3. **Given** an automated pipeline requesting JSON output, **When** the user runs `replayfix doctor --json`, **Then** the diagnostic report is emitted to stdout as machine-readable JSON matching the diagnostic schema.

---

### User Story 2 - Canonical Task Manifest & CLI Ingestion (Priority: P2)

As a developer or automated evaluation harness, I want to invoke `replayfix repro` with explicit CLI arguments (`--video`, `--repo`, `--task-id`, `--bug-ref`, `--headed`, `--server-cmd`) or a pre-formulated `--manifest <path>`, so that task definitions strictly validate against the canonical `TaskManifest` schema before any browser or process execution begins.

**Why this priority**: Canonical schema adherence ensures consistent input structure across local interactive debugging and automated benchmark sweeps.

**Independent Test**: Invoke `replayfix repro` with valid and invalid CLI flags, verifying that valid configurations hydrate a verified `TaskManifest` instance, while missing required fields or invalid paths fail immediately with structured error messages.

**Acceptance Scenarios**:
1. **Given** valid reproduction parameters (`--task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --repo ./target-app`), **When** the user runs `replayfix repro`, **Then** the harness parses and hydrates an active `TaskManifest`, validating all required fields and confirming readiness for Gate 1 baseline reproduction.
2. **Given** a direct path to a task manifest JSON file, **When** the user runs `replayfix repro --manifest task.json`, **Then** the manifest is parsed and validated against the canonical schema.
3. **Given** missing mandatory arguments (e.g. omitting `--video` and `--manifest`), **When** the user invokes `replayfix repro`, **Then** the CLI rejects the command with a descriptive error listing the missing required inputs and exits with code 2.

---

### User Story 3 - Path Traversal & Security Boundary Sanitization (Priority: P3)

As a security-conscious engineer, I want the harness to reject malicious or out-of-bounds filesystem paths in `--repo` and `--video` arguments, so that reproduction execution cannot escape the designated workspace boundary or access sensitive host files.

**Why this priority**: Adheres to Law III (Workspace Isolation) and prevents path traversal vulnerabilities (`test_16_path_traversal_is_blocked`).

**Independent Test**: Supply arguments containing path traversal sequences (e.g. `../../etc/passwd` or absolute root overrides) and verify the harness immediately aborts before any filesystem operations occur.

**Acceptance Scenarios**:
1. **Given** a `--repo` or `--video` argument pointing outside allowed boundaries via traversal sequences (`../`), **When** `replayfix repro` is executed, **Then** the harness rejects the path with `SECURITY_VIOLATION` and exits with code 11.

---

### Edge Cases

- **Missing Executable Permissions**: What happens when a tool binary exists in PATH but cannot be executed due to OS security or file permissions?
  - *Behavior*: `replayfix doctor` catches the execution failure, classifies it as `UNAVAILABLE (Permission Denied)`, and suggests checking execution policies.
- **Malformed Version Strings**: What happens if an installed tool prints a custom or non-standard version output (e.g. a Git fork or custom Node build)?
  - *Behavior*: The version parser extracts semantic version digits using resilient regex pattern matching without panicking.
- **Target Repository Not a Git Repo**: What happens if `--repo` points to a directory without a `.git` folder?
  - *Behavior*: The harness rejects the target path with an informative error stating that the target repository must be an initialized Git repository.
- **Port Environment Variable Customization**: What happens when the target application uses an unconventional port variable (e.g. `SERVER_PORT` instead of `PORT`)?
  - *Behavior*: The `environment_adapter` config cleanly accepts `--port-env-var SERVER_PORT`, defaulting to `PORT` if unspecified.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a `doctor` command (`replayfix doctor`) that deterministically checks the presence and versions of Git, Node.js, Playwright, and FFmpeg.
- **FR-002**: System MUST verify that detected versions meet minimum requirements: Git $\ge 2.30.0$, Node.js $\ge 18.0.0$, Playwright $\ge 1.40.0$, FFmpeg $\ge 4.4.0$.
- **FR-003**: System MUST provide human-readable formatted table output and machine-readable JSON output (`--json`) for `doctor` diagnostics.
- **FR-004**: System MUST emit actionable installation commands when one or more required dependencies are missing or incompatible.
- **FR-005**: System MUST provide a `repro` command (`replayfix repro`) that accepts parameters mapping directly to `TaskManifest`: `--task-id`, `--repo`, `--bug-ref`, `--clean-ref`, `--video`, `--audio`, `--server-cmd`, `--port-env-var`, `--healthcheck-path`, `--startup-timeout-ms`, `--headed`, and `--trace`.
- **FR-006**: System MUST accept a `--manifest <path>` argument to ingest an existing JSON file conforming to `task_manifest.schema.json`.
- **FR-007**: System MUST validate that all required manifest fields (`task_id`, `title`, `repo_path`, `bug_ref`, `video_path`) are present before proceeding to reproduction dispatch.
- **FR-008**: System MUST sanitize and canonicalize all supplied paths, rejecting paths containing illegal characters or unauthorized path traversal escapes.
- **FR-009**: System MUST resolve OS-specific binary executable conventions transparently through the platform abstraction layer without leaking shell-specific script extensions.
- **FR-010**: System MUST support visible browser mode (`--headed`) and trace capture (`--trace`) without modifying the deterministic execution flow.

### Key Entities *(data models)*

- **DependencyCheck**: Represents a single tool dependency evaluation, containing tool name, executable path, detected version, minimum version, status (`Compatible`, `IncompatibleVersion`, `NotFound`), and remediation command.
- **DoctorReport**: Aggregation of all tool checks, host OS family, overall pass/fail status, and timestamp.
- **TaskManifest**: Canonical benchmark reproduction specification containing task identifier, defect metadata, target repo path, video evidence path, audio path, expected behavior, headed preference, and environment adapter configuration.
- **EnvironmentAdapter**: Configuration defining how to spawn and monitor the target web application: server launch command, port environment variable name, healthcheck endpoint path, and readiness timeout.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: `replayfix doctor` completes multi-tool diagnostics across all 4 dependencies in under 1,500 milliseconds on standard hardware.
- **SC-002**: Diagnostic failure rate on valid environments is 0% (zero false positives when tools are present in PATH).
- **SC-003**: Ingestion validation of `TaskManifest` guarantees 100% adherence to `task_manifest.schema.json` with zero unvalidated fields passing into the runtime harness.
- **SC-004**: Path sanitization catches 100% of tested directory traversal sequences (`../`, absolute escaping) before any filesystem operations occur.

---

## Assumptions

- Target applications run on Node.js / modern web stacks compatible with Playwright CDP instrumentation.
- Users executing `doctor` have basic privileges to inspect installed developer binaries.
- Video inputs are encoded in standard container formats (MP4, WebM) decodable by FFmpeg.
- When `--clean-ref` is omitted, the harness assumes Mode B (Invariant Defect Mode) as per constitutional principles.

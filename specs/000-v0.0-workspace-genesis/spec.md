# Feature Specification: 000-v0.0-workspace-genesis — Clean-Room Workspace Genesis & Toolchain Lock-In

**Feature Branch**: `main` (initial Day-0 baseline commit `v0.0.0`)  
**Created**: 2026-10-26  
**Status**: Ready for Planning  
**Input**: Clean-room workspace genesis, Cargo 2-crate workspace structure (`replayfix-core` + `replayfix-cli`), cross-platform host toolchain verification, and living state baseline lock-in.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Pristine 2-Crate Workspace Architecture (Priority: P1)

As a systems engineer building ReplayFix from scratch under the clean-room protocol, I want a standardized 2-crate Cargo workspace linking `replayfix-core` and `replayfix-cli`, so that core domain abstractions, platform handlers, and verification oracles remain strictly decoupled from CLI presentation and interactive commands.

**Why this priority**: Core library and CLI boundaries must be established at Day-0. A clean 2-crate architecture eliminates compile latency and crate-boundary overhead while preventing circular dependencies.

**Independent Test**: Execute `cargo check --workspace` and `cargo test --workspace`. Both crates must compile cleanly and run basic unit tests without errors.

**Acceptance Scenarios**:
1. **Given** an initialized workspace with root `Cargo.toml`, **When** `cargo check --workspace` is executed, **Then** both `replayfix-core` and `replayfix-cli` compile successfully with exit code 0.
2. **Given** basic domain and platform modules in `replayfix-core`, **When** `cargo test --workspace` is executed, **Then** all workspace tests pass with 0 failures.
3. **Given** a built CLI binary, **When** executing the CLI with `--version`, **Then** it prints `replayfix 0.1.0` and exits with code 0.

---

### User Story 2 - Host Toolchain & Platform Abstraction (Priority: P2)

As a developer running on native Windows MSVC (or Unix in CI), I want the core crate to expose a cross-platform abstraction (`Platform`) that detects OS family and canonicalizes paths, so that OS-specific commands and paths do not leak into business logic or cause runtime incompatibilities.

**Why this priority**: Satisfies Law VII (Cross-Platform Native Substrate). Platform differences must be encapsulated from Day-0.

**Independent Test**: Run unit tests on `Platform::os_family()` and verify that the current operating system is correctly detected and reported.

**Acceptance Scenarios**:
1. **Given** a Windows host, **When** `Platform::os_family()` is evaluated, **Then** it returns `OsFamily::Windows`.
2. **Given** a path to resolve, **When** `Platform::canonicalize_path` is called, **Then** it normalizes the path and verifies existence.

---

### User Story 3 - Living Agent State & Reproducibility Provenance (Priority: P3)

As an adversarial reviewer or hackathon judge, I want a living state journal (`docs/agent-state.md`) and version tag (`v0.0.0`) establishing verifiable Day-0 baseline milestones, so that clean-room provenance and progressive micro-slices are empirically auditable.

**Why this priority**: Transparency and empirical reproducibility are non-negotiable hackathon requirements.

**Independent Test**: Inspect `docs/agent-state.md` to verify recorded compiler outputs, timestamps, and active spec pointers.

**Acceptance Scenarios**:
1. **Given** successful compilation and testing, **When** `docs/agent-state.md` is examined, **Then** it details the exact toolchain versions, verified test outcomes, and active next milestone.

---

### Edge Cases

- **Platform Security & Binary Execution Policies**: What happens if host security (such as Windows WDAC or Application Control) restricts unsigned debug binaries?
  - *Behavior*: Build profiles (`--release` with LTO) and test runner harnesses execute cleanly, ensuring development verification is uninterrupted.
- **Path Separation Across OS Families**: What happens when paths with forward slashes or backslashes are passed across platform boundaries?
  - *Behavior*: `Platform::canonicalize_path` safely handles platform-native representations without breaking path equality checks.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a root Cargo workspace manifest (`Cargo.toml`) with members `crates/replayfix-core` and `crates/replayfix-cli` using Cargo edition 2021 and resolver 2.
- **FR-002**: `crates/replayfix-core` MUST be configured as a library crate providing core error handling (`error::CoreError`) and platform detection (`platform::Platform`).
- **FR-003**: `crates/replayfix-cli` MUST be configured as a binary crate depending directly on `replayfix-core` and `clap` for command-line parsing.
- **FR-004**: System MUST provide a CLI entrypoint supporting `--version`, a basic `doctor` command, and a stubbed `repro` command.
- **FR-005**: System MUST establish a `.gitignore` policy that keeps the repository clean of build artifacts (`target/`), caches, and ephemeral test runs.
- **FR-006**: System MUST maintain an auditable state document (`docs/agent-state.md`) capturing empirical verification records for each milestone.
- **FR-007**: System MUST compile cleanly via `cargo check --workspace` and pass all unit tests via `cargo test --workspace` with exit code 0.

### Key Entities *(data models)*

- **Platform**: Cross-platform abstraction utility encapsulating host OS family detection and path canonicalization.
- **OsFamily**: Enumeration representing host operating systems (`Windows`, `Unix`).
- **CoreError**: Central error enum leveraging `thiserror` for structured error handling across I/O, serialization, and execution.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Workspace compilation (`cargo check --workspace`) completes in under 60 seconds with 0 warnings/errors.
- **SC-002**: Workspace unit tests (`cargo test --workspace`) pass with 100% success rate (0 failures).
- **SC-003**: CLI binary executes cleanly and reports accurate version and host detection in under 100 milliseconds.
- **SC-004**: Zero extraneous or pre-built code is copied wholesale; 100% of workspace files are cleanly synthesized and verifiable.

---

## Assumptions

- Rust toolchain (rustc and cargo $\ge 1.80$) is installed and configured on the host machine.
- Native build environment (MSVC on Windows, GCC/Clang on Unix) is available for native linking.
- Spec Kit is initialized and governs subsequent vertical micro-slice progression.

# Research & Architecture Decisions: 000-v0.0-workspace-genesis

**Feature**: Clean-Room Workspace Genesis & Toolchain Lock-In  
**Date**: 2026-10-26  

---

## 1. Pristine 2-Crate Architecture vs. Multi-Crate Proliferation

- **Decision**: Adopt a strict 2-crate Cargo workspace consisting solely of `crates/replayfix-core` (library) and `crates/replayfix-cli` (binary).
- **Rationale**:
  - Eliminates circular dependency hazards and crate-boundary compilation overhead.
  - Keeps compilation latency under 60 seconds on initial build and sub-second for incremental checks.
  - Preserves clean encapsulation: CLI parsing and progress rendering stay in `replayfix-cli`; platform abstractions, verification state machine, and error models stay in `replayfix-core`.
- **Alternatives Considered**:
  - *Multi-crate micro-decomposition (e.g. `replayfix-common`, `replayfix-oracle`, `replayfix-verifier`, `replayfix-receipt`)*: Rejected during Day-0 Working Backwards audit due to excessive compile times, trait overhead, and interface friction during rapid hackathon iteration. Heavy cryptographic receipts are deferred to P2 stretch status.
  - *Single monolithic crate*: Rejected because separating library from CLI binary enables clean integration testing and MCP server embedding.

---

## 2. Cross-Platform Abstraction & Host Toolchain Support

- **Decision**: Encapsulate all operating system interactions behind `replayfix_core::platform::Platform`, detecting `OsFamily` at compile/runtime and normalizing paths without shell-specific assumptions.
- **Rationale**:
  - Windows environments require handling `.exe` suffixes, backslash path normalization, and specific execution security policies (such as WDAC / Smart App Control).
  - Unix environments require POSIX process spawning and signal handling.
  - Leaking `.cmd` or hardcoded paths causes immediate failure across diverse judge evaluation machines.
- **Alternatives Considered**:
  - *Ad-hoc `cfg!(windows)` scattered throughout codebase*: Rejected due to high risk of regressions and maintenance burden.

---

## 3. Error Handling Architecture

- **Decision**: Use `thiserror` (v2.0) in `replayfix-core` to provide an exhaustive, strongly-typed `CoreError` enum, and standard `Result<T, Box<dyn std::error::Error>>` in `replayfix-cli`.
- **Rationale**:
  - Strongly typed errors allow caller functions and tests to pattern-match on exact failure reasons (e.g. `CoreError::Environment`, `CoreError::Io`, `CoreError::Verification`).
  - `thiserror` provides zero-overhead `#[derive(Error)]` without dynamic dispatch cost.

---

## 4. CLI Framework & Subcommand Routing

- **Decision**: Standardize on `clap` (v4.5) with `derive` and `env` features.
- **Rationale**:
  - Declarative struct-based argument parsing provides automatic `--help`, `--version`, subcommands, and environment variable fallbacks with compile-time safety.
  - Zero manual argument parsing boilerplate.

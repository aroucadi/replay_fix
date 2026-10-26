# Implementation Plan: 000-v0.0-workspace-genesis — Clean-Room Workspace Genesis & Toolchain Lock-In

**Branch**: `main` (initial Day-0 baseline commit `v0.0.0`) | **Date**: 2026-10-26 | **Spec**: [spec.md](spec.md)

**Input**: Clean-room workspace genesis, Cargo 2-crate workspace structure (`replayfix-core` + `replayfix-cli`), cross-platform host toolchain verification, and living state baseline lock-in.

---

## Summary

Establish a pristine, compliant, zero-prior-code Git repository and 2-crate Cargo workspace (`replayfix-core` library and `replayfix-cli` binary) on the native Windows MSVC toolchain. Configure root manifest dependencies, structured error taxonomies, cross-platform host abstraction, and verify clean compilation (`cargo check`) and unit test execution (`cargo test`).

---

## Technical Context

**Language/Version**: Rust 1.99.1 (Edition 2021)  
**Primary Dependencies**:
- `replayfix-core`: `serde` (1.0), `serde_json` (1.0), `thiserror` (2.0), `tokio` (1.43), `tracing` (0.1), `walkdir` (2.5), `chrono` (0.4), `async-trait` (0.1)
- `replayfix-cli`: `clap` (4.5), `tokio` (1.43), `tracing` (0.1), `tracing-subscriber` (0.3)  
**Storage**: N/A for Slice 000 (Local filesystem configuration)  
**Testing**: `cargo test --workspace` (built-in Rust test harness)  
**Target Platform**: Native Windows (`x86_64-pc-windows-msvc`), macOS, Linux  
**Project Type**: 2-crate Cargo Workspace (Core Library + CLI Binary)  
**Performance Goals**: Sub-60s clean compilation; sub-second incremental check  
**Constraints**: Zero pre-built code; read-only base repository isolation ($\Delta \text{repo} = 0$); fail-closed verification  
**Scale/Scope**: Day-0 workspace baseline foundation  

---

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Constitutional Invariant | Status | Alignment & Evidence |
| :--- | :---: | :--- |
| **Law I: Substrate Primacy & Fail-Closed Authority** | **PASS** | Pure Rust implementation; deterministic verification substrate holds sole authority. |
| **Law II: Behavioral Contract Invariance** | **PASS** | Dependencies on `serde` and `serde_json` established for canonical contract deserialization. |
| **Law III: Workspace Isolation** | **PASS** | `Platform` path canonicalization prevents path escapes; base repository remains read-only. |
| **Law VII: Cross-Platform Native Substrate** | **PASS** | `replayfix_core::Platform` cleanly abstracts OS family differences. |
| **Law X: Elephant Carpaccio & Vertical Micro-Slicing** | **PASS** | Slice 000 is the first, thinnest vertical slice (`v0.0.0`), compiling and testable standalone. |

---

## Project Structure

### Documentation (this feature)

```text
specs/000-v0.0-workspace-genesis/
├── spec.md              # Feature specification
├── plan.md              # This implementation plan
├── research.md          # Architecture decisions (2-crate layout, platform abstraction)
├── data-model.md        # Domain entities (Platform, OsFamily, CoreError)
├── quickstart.md        # Verification and run guide
├── checklists/
│   └── requirements.md  # Spec quality checklist
└── tasks.md             # Actionable task list (generated in next phase)
```

### Source Code Layout

```text
Cargo.toml               # Workspace root manifest (members: replayfix-core, replayfix-cli)
crates/
├── replayfix-core/
│   ├── Cargo.toml       # Core library dependencies
│   └── src/
│       ├── lib.rs       # Library root exposing error and platform modules
│       ├── error.rs     # CoreError enum (thiserror)
│       └── platform.rs  # Platform struct and OsFamily enum
└── replayfix-cli/
    ├── Cargo.toml       # CLI dependencies (replayfix-core, clap)
    └── src/
        └── main.rs      # Main CLI entrypoint with doctor and repro commands
```

**Structure Decision**: A 2-crate workspace structure cleanly separates the testable domain logic and platform abstractions (`replayfix-core`) from the CLI argument parsing and terminal presentation (`replayfix-cli`).

---

## Complexity Tracking

Zero constitutional violations or unwarranted abstractions. No database, network services, or heavy dependencies added in this initial slice.

# Implementation Plan: 010-v0.1-walking-skeleton — Environment Diagnostics & Repro Manifest Ingestion

**Branch**: `release/v0.1.0-walking-skeleton` | **Date**: 2026-10-26 | **Spec**: [spec.md](spec.md)

**Input**: Multi-dependency environment diagnostics (`replayfix doctor`) covering Git, Node.js, Playwright, and FFmpeg; initial `replayfix repro` CLI flags and parameters strictly adhering to canonical `task_manifest.schema.json`.

---

## Summary

Implement comprehensive multi-dependency environment diagnostics in `replayfix_core::doctor` with semantic version verification and actionable remediation advice. Implement canonical `TaskManifest` data structures and deserializers in `replayfix_core::manifest` with dual CLI/JSON ingestion, path canonicalization, and path-traversal security verification. Connect both capabilities to `replayfix-cli` with human and JSON output modes.

---

## Technical Context

**Language/Version**: Rust 1.99.1 (Edition 2021)  
**Primary Dependencies**:
- `replayfix-core`: `serde`, `serde_json`, `thiserror`, `tokio`, `tracing`, `walkdir`, `chrono`, `regex`
- `replayfix-cli`: `clap` (4.5), `tokio` (1.43), `tracing`, `tracing-subscriber`  
**Storage**: Ephemeral session state and structured stdout  
**Testing**: `cargo test --workspace` with automated unit and integration tests  
**Target Platform**: Native Windows (`x86_64-pc-windows-msvc`), macOS, Linux  
**Project Type**: 2-crate Cargo Workspace (Core Library + CLI Binary)  
**Performance Goals**: `doctor` execution $< 1,500$ms; manifest validation $< 50$ms  
**Constraints**: Fail-closed verification; canonical schema compliance; path traversal immunity; zero absolute reference paths in documentation  

---

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Constitutional Invariant | Status | Alignment & Evidence |
| :--- | :---: | :--- |
| **Law I: Substrate Primacy & Fail-Closed Authority** | **PASS** | Rust harness deterministically inspects host binaries and strictly validates manifests. Missing dependencies fail closed. |
| **Law II: Behavioral Contract Invariance** | **PASS** | `TaskManifest` models strictly enforce canonical contract schema attributes. |
| **Law III: Workspace Isolation** | **PASS** | Path canonicalization and traversal checks reject out-of-boundary paths (`SECURITY_VIOLATION`, code 11). |
| **Law VII: Cross-Platform Native Substrate** | **PASS** | Probes use `Platform::create_command` without leaking shell scripts or OS quirks. |
| **Law X: Elephant Carpaccio & Vertical Micro-Slicing** | **PASS** | Milestone `v0.1.0` delivers testable diagnostics and manifest ingestion in a bounded, reviewable slice. |

---

## Project Structure

### Documentation (this feature)

```text
specs/010-v0.1-walking-skeleton/
├── spec.md              # Feature specification
├── plan.md              # This implementation plan
├── research.md          # Architecture decisions (probing strategy, manifest mapping)
├── data-model.md        # Domain entities (DependencyCheck, DoctorReport, TaskManifest)
├── quickstart.md        # Verification and run guide
├── checklists/
│   └── requirements.md  # Spec quality checklist
└── tasks.md             # Actionable task breakdown
```

### Source Code Layout

```text
crates/
├── replayfix-core/
│   ├── Cargo.toml       # Added regex dependency for version parsing
│   └── src/
│       ├── lib.rs       # Expose doctor, manifest, error, platform
│       ├── doctor.rs    # Multi-tool environment probes and report generator
│       ├── manifest.rs  # TaskManifest, EnvironmentAdapter, and path validation
│       ├── error.rs     # Extended error taxonomy for doctor and manifest failures
│       └── platform.rs  # Cross-platform command spawning and path checks
└── replayfix-cli/
    └── src/
        └── main.rs      # Wire doctor (--json) and repro (--manifest / flags) commands
```

---

## Complexity Tracking

Zero constitutional violations. Simple regex-based version extraction avoids heavy external crates.

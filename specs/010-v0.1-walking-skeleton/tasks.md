# Tasks: 010-v0.1-walking-skeleton — Environment Diagnostics & Repro Manifest Ingestion

**Feature**: Environment Diagnostics & Repro Manifest Ingestion  
**Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)  
**Status**: Completed  

---

## Phase 1: Setup & Core Dependencies

- [x] T001 Add `regex` dependency to `crates/replayfix-core/Cargo.toml` for resilient semantic version parsing

---

## Phase 2: Foundational Error & Domain Taxonomy

- [x] T002 Extend `CoreError` with manifest and security variants in `crates/replayfix-core/src/error.rs`

---

## Phase 3: User Story 1 — Multi-Dependency Environment Diagnostics (Priority: P1)

*Goal*: Implement deterministic diagnostic probes for Git, Node.js, Playwright, and FFmpeg with actionable remediation advice.  
*Independent Test*: `cargo test -p replayfix-core -- doctor` and `cargo run --release -p replayfix-cli -- doctor`.

- [x] T003 [P] [US1] Define `DependencyStatus`, `DependencyCheck`, and `DoctorReport` structs in `crates/replayfix-core/src/doctor.rs`
- [x] T004 [US1] Implement dependency probing and semantic version comparison in `crates/replayfix-core/src/doctor.rs`
- [x] T005 [P] [US1] Add unit tests for version parsing and missing-tool remediation in `crates/replayfix-core/src/doctor.rs`
- [x] T006 [US1] Wire `replayfix doctor` command and `--json` flag with human/JSON table rendering in `crates/replayfix-cli/src/main.rs`

---

## Phase 4: User Story 2 — Canonical Task Manifest & Repro CLI Ingestion (Priority: P2)

*Goal*: Implement `TaskManifest` and `EnvironmentAdapter` strictly conforming to `task_manifest.schema.json` with dual CLI/JSON file ingestion.  
*Independent Test*: `cargo test -p replayfix-core -- manifest` and `cargo run --release -p replayfix-cli -- repro --help`.

- [x] T007 [P] [US2] Define `EnvironmentAdapter` and `TaskManifest` structs with serde validation in `crates/replayfix-core/src/manifest.rs`
- [x] T008 [US2] Implement manifest JSON file parser and validation logic in `crates/replayfix-core/src/manifest.rs`
- [x] T009 [P] [US2] Add unit tests for manifest deserialization and required field validation in `crates/replayfix-core/src/manifest.rs`
- [x] T010 [US2] Wire `replayfix repro` CLI flags matching `task_manifest.schema.json` and `--manifest` in `crates/replayfix-cli/src/main.rs`

---

## Phase 5: User Story 3 — Path Traversal & Security Boundary Defense (Priority: P3)

*Goal*: Enforce path canonicalization and directory traversal rejection on target repo and video paths.  
*Independent Test*: `cargo test -p replayfix-core -- security` verifying path traversal attempts fail with `SECURITY_VIOLATION`.

- [x] T011 [US3] Implement path sanitization and directory traversal detection in `crates/replayfix-core/src/manifest.rs`
- [x] T012 [US3] Add security unit tests asserting path traversal sequences are rejected in `crates/replayfix-core/src/manifest.rs`
- [x] T013 [US3] Enforce path validation in CLI repro execution handler in `crates/replayfix-cli/src/main.rs`

---

## Phase 6: Documentation, Release Notes & Verification

*Goal*: Validate release candidate, enrich README, update CHANGELOG, and publish `v0.1.0`.

- [x] T014 Run full workspace test suite `cargo test --workspace` and release compilation
- [x] T015 Enrich `README.md` with doctor diagnostic output, repro CLI parameters, and manifest examples
- [x] T016 Establish `CHANGELOG.md` with Keep a Changelog format, documenting `v0.0.0` and `v0.1.0` releases
- [x] T017 Execute CLI smoke tests on live host (`doctor`, `doctor --json`, `repro --help`, `repro` with manifest)
- [x] T018 Commit changes, create release tag `v0.1.0`, merge to `main`, and push to remote

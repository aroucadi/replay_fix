# Implementation Plan: 030-v0.3-ephemeral-scratch-isolation — Ephemeral Scratch Isolation & Dual-Layer Integrity

**Branch**: `release/v0.3.0-scratch-isolation` | **Date**: 2026-10-27 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/030-v0.3-ephemeral-scratch-isolation/spec.md`

---

## Summary

Add `crates/replayfix-core/src/worktree.rs` implementing `WorktreeManifest` (dual-layer recursive SHA-256 file tree manifest + `git status --porcelain` guard) and `ScratchSandbox` (ephemeral scratch workspace in `std::env::temp_dir()` with boundary-checked file operations, child process cleanup, read-only attribute clearing, and Windows file-lock retry backoff). Extend `Platform` in `crates/replayfix-core/src/platform.rs` with `EphemeralPort::allocate()` (`127.0.0.1:0`) and `Platform::terminate_process_tree(pid)`. Wire `MockHealer` and `replayfix repro --mock-heal` to execute Gate 1, patch application, and Gate 2 on disk inside `ScratchSandbox`, verify $\Delta \text{repo} = 0$ on the base repository (`BASE_REPO_DIRTY_MUTATION_DETECTED` on any drift), and emit `base_tree_sha256`, `sandbox_tree_sha256`, `base_repo_unmodified_verified`, and `ephemeral_port` in `audit_receipt.json` (schema `"0.3"`).

---

## Technical Context

**Language/Version**: Rust 1.99.1 (Edition 2021)
**New Dependencies**: None. Uses `std::fs`, `std::net::TcpListener`, `std::process::Command`, and existing `sha2` / `serde_json`.
**Storage**: Ephemeral scratch directories in `std::env::temp_dir()` (automatically removed on completion/drop); JSON receipts in `--output-dir` (default `.replayfix/`).
**Testing**: `cargo test --workspace` — unit tests in `worktree.rs`, `platform.rs`, `engine.rs`, `receipt.rs` and end-to-end CLI integration tests in `crates/replayfix-cli/tests/cli.rs`.
**Target Platform**: Windows (MSVC), macOS, Linux.
**Constraints**: Zero external crate additions; fail-closed on any base-repository mutation or path escape; deterministic cleanup on Windows even with read-only files.

---

## Constitution Check

| Constitutional Invariant | Status | Alignment & Evidence |
| :--- | :---: | :--- |
| **Law I: Substrate Primacy** | **PASS** | `PhaseMachine` and `WorktreeManifest` in Rust hold sole authority over gate verdicts and isolation checks. |
| **Law II: Contract Invariance** | **PASS** | Canonical SHA-256 contract digest check preserved across Gate 1 and Gate 2. |
| **Law III: Workspace Isolation ($\Delta \text{repo} = 0$)** | **PASS** | Enforced via pre/post `WorktreeManifest` (recursive file SHA-256 + `git status --porcelain`) and `ScratchSandbox` execution in `std::env::temp_dir()`. |
| **Law IV: Ephemeral Provenance** | **PASS** | `EphemeralPort::allocate()` binds `127.0.0.1:0` and `Platform::terminate_process_tree` supervises child PIDs. |
| **Law VI: Monotonic Ordering** | **PASS** | `SETUP -> TARGET -> ACTION -> ASSERT` event ordering preserved on sandboxed files. |
| **Law VII: Model Honesty** | **PASS** | Receipt records `mock-static-oracle` and explicit disclaimer. |
| **Law X: BeautifulRust & Vertical Micro-Slicing** | **PASS** | `unsafe_code = "forbid"`, zero `.unwrap()` in production code, `Debug` derived on all public types, zero new dependencies. |

---

## Project Structure

```text
specs/030-v0.3-ephemeral-scratch-isolation/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md  tasks.md
└── checklists/requirements.md

crates/replayfix-core/src/
├── lib.rs        # export worktree types and EphemeralPort
├── error.rs      # + Worktree error variant
├── platform.rs   # + EphemeralPort::allocate, Platform::terminate_process_tree
├── worktree.rs   # NEW: FileEntry, WorktreeManifest, ScratchSandbox
├── engine.rs     # Sandbox-backed MockHealer::run_in_repo + fault-injection hook for tests
└── receipt.rs    # Schema "0.3", + BaseRepoDirtyMutationDetected, + isolation fields on AuditReceipt

crates/replayfix-cli/
├── src/main.rs   # Pass validated --repo path into MockHealer::run_in_repo; display isolation summary
└── tests/cli.rs  # End-to-end tests for repo isolation, custom repo file, missing repo, and sandbox cleanup
```

---

## Complexity Tracking

Zero new crate dependencies added. `std::fs` recursive walk and `std::net::TcpListener` provide everything needed with full control over exclusions, path normalization, and Windows read-only permission clearing.

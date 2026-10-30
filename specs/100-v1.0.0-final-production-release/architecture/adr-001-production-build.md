# ADR-001: Production Build Profile & `panic = "abort"` Safety Decision

## Status
Accepted

## Context
ReplayFix v1.0.0 requires an optimized production binary (`[profile.release]`) with `opt-level = 3`, `lto = true`, `codegen-units = 1`, and `panic = "abort"`. We must verify that `panic = "abort"` does not endanger repository immutability ($\Delta	ext{repo}=0$) or leave orphaned child processes or corrupted artifacts.

## Decision
Approve `opt-level = 3`, `lto = true`, `codegen-units = 1`, and `panic = "abort"` in the workspace root `Cargo.toml` for three reasons:
1. **Zero Non-Test Panics**: Every production module in `replayfix-core` and `replayfix-cli` compiles under `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::unreachable))]` and is verified by `scripts/check-panics.ps1` (`Total non-test violations: 0`).
2. **Two-Phase Atomic File Persistence**: `ArtifactStore` and `atomic_write_file` write to `.tmp.<pid>.<counter>` and atomically rename to the target filename, so even an OS-level abort cannot leave truncated JSON receipts.
3. **Ephemeral Scratch Isolation**: All candidate edits happen inside `%TEMP%/replayfix-scratch-*`, never in the user's base repository (`WorktreeManifest` verified).

# Research & Decisions: 030-v0.3-ephemeral-scratch-isolation

## D1 — How is $\Delta \text{repo} = 0$ proven without trusting Git index commands alone?
**Decision**: Dual-layer manifest verification in `replayfix_core::worktree::WorktreeManifest`:
1. **Layer 1 (Recursive Filesystem Walk)**: Recursively walk the working tree, hash every regular file's bytes with SHA-256, record `(relative_path, size, sha256)` sorted by normalized forward-slash relative path, and hash the canonical JSON array into `tree_sha256`.
2. **Layer 2 (Git Working Tree Porcelain Guard)**: When `.git` exists in the target directory, run `git -C <repo> status --porcelain=v1 -uall` and store the exact output alongside the file manifest.
**Rationale**: `git write-tree` or index-only checks miss unstaged working-tree edits or untracked files created outside the index, whereas `git status --porcelain` alone does not produce a single cryptographic digest when the target directory is not a Git repository (such as a standalone fixture directory in unit tests). Combining both catches every file creation, deletion, or byte modification regardless of whether `.git` is present.
**Alternatives considered**: Relying solely on `git status --porcelain` (rejected: fails on non-Git fixture directories and does not yield a cryptographic tree digest for `base_tree_sha256` / `sandbox_tree_sha256`).

## D2 — Which directories are excluded from `WorktreeManifest` and `ScratchSandbox` copying?
**Decision**: Always exclude `.git`, `.replayfix`, `node_modules`, and `target` at the top level, plus any caller-specified relative `--output-dir` path when it resolves inside `--repo`.
**Rationale**:
- `.git` internal object/index state is covered by Layer 2 (`git status --porcelain`) without copying megabytes of packfiles into `temp_dir()`.
- `.replayfix` (or custom `--output-dir`) is where ReplayFix writes `audit_receipt.json` / `triage_report.json`; excluding it ensures that writing the receipt or cleaning stale receipts never triggers a false positive on base-repo mutation.
- `node_modules` and `target` are build caches that should never be recursively hashed or copied per run.

## D3 — How do we prevent Windows `os error 5` (Access Denied) during scratch sandbox cleanup?
**Decision**: `ScratchSandbox::cleanup` performs three steps in order:
1. Terminate any registered child process IDs via `Platform::terminate_process_tree(pid)`.
2. Walk the sandbox directory and clear read-only attributes (`permissions.set_readonly(false)`) on all files and directories.
3. Retry `std::fs::remove_dir_all` with bounded backoff (up to 5 attempts: `10ms`, `25ms`, `50ms`, `100ms`, `200ms`).
**Rationale**: On Windows, `std::fs::remove_dir_all` fails with `ERROR_ACCESS_DENIED (os error 5)` if any file inside the tree is marked read-only or briefly held open by an exiting child process or Windows Defender scan. Clearing read-only flags and retrying makes cleanup deterministic on Windows, macOS, and Linux.
**Alternatives considered**: Adding the `tempfile` crate (rejected by Scope Discipline: `std::fs` + a 40-line `ScratchSandbox` with explicit read-only clearing and retry backoff solves the exact Windows lock problem without adding a new crate dependency).

## D4 — How is dynamic ephemeral port allocation implemented?
**Decision**: `EphemeralPort::allocate()` binds `std::net::TcpListener::bind(("127.0.0.1", 0))`, reads `local_addr()?.port()`, and returns the OS-assigned non-zero `u16` port.
**Rationale**: Uses the Rust standard library (`std::net::TcpListener`) with zero external dependencies, works identically across Windows, macOS, and Linux, and avoids Next.js / Vite `PORT=0` stdout-parsing pitfalls noted in Spec 040 by obtaining a concrete port number from the OS kernel.

## D5 — How does `--mock-heal` behave when `--repo` does not contain `src/cart-summary.js`?
**Decision**: If the copied scratch sandbox does not already contain the fixture's `target_path` (`src/cart-summary.js`), `ScratchSandbox` materializes the built-in buggy source inside the **scratch sandbox only** before Gate 1 runs. If `--repo` *does* contain `src/cart-summary.js`, the sandbox uses the copied file from `--repo`.
**Rationale**: Preserves 100% backward compatibility with `replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video v.mp4 --mock-heal` in an empty directory (existing v0.2 CLI integration tests), while allowing v0.3 tests and users to point `--repo` at an actual directory containing `src/cart-summary.js` and verify that the base repo's copy remains untouched while the sandbox copy is patched and verified.

## D6 — What exit code and triage status represent a base-repository mutation?
**Decision**: `TriageStatus::BaseRepoDirtyMutationDetected` (`"BASE_REPO_DIRTY_MUTATION_DETECTED"`) with exit code `11` (`SECURITY_VIOLATION` / isolation breach).
**Rationale**: Violating Law III ($\Delta \text{repo} = 0$) is a critical isolation boundary failure. Mapping it to exit code `11` aligns with `patch_verdict.schema.json` (`11: Security/diff violation`).

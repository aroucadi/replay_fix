# Feature Specification: 030-v0.3-ephemeral-scratch-isolation — Ephemeral Scratch Isolation & Dual-Layer Integrity

**Feature Branch**: `release/v0.3.0-scratch-isolation`
**Created**: 2026-10-27
**Status**: Ready for Planning
**Target Release**: `v0.3.0`
**Input**: Ephemeral scratch repository isolation in the system temporary directory, dual-layer base repository integrity verification ($\Delta \text{repo} = 0$ via recursive SHA-256 file manifests and `git status --porcelain`), dynamic ephemeral port allocation (`:0`), and child process tree cleanup with Windows file-lock defense.

---

## Scope Statement (read first)

v0.3 moves the repair and verification loop out of pure in-memory strings and into an **isolated ephemeral scratch sandbox** on disk while proving that the developer's base repository is never modified ($\Delta \text{repo} = 0$). It also introduces dynamic OS-assigned ephemeral port allocation (`:0`) and cross-platform process tree termination with Windows file-lock retry defense. Gates are still evaluated by the deterministic simulated oracle on the files inside the scratch sandbox; live Playwright browser execution arrives in Spec 040 (v0.4).

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Dual-Layer Base Repository Integrity ($\Delta \text{repo} = 0$) (Priority: P1)

As a developer pointing ReplayFix at my working repository, I want the harness to verify before and after every run that my repository had zero bytes and zero Git status entries modified, so that candidate patches and test runs can never corrupt or dirty my working tree.

**Why this priority**: Inviolable base workspace isolation (Constitutional Law III) is a core trust guarantee of the harness.

**Independent Test**: Run `replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --repo ./my-repo --mock-heal`. The command exits `0`, leaves `./my-repo` byte-for-byte unchanged, and records `base_tree_sha256`, `sandbox_tree_sha256`, and `base_repo_unmodified_verified: true` in `.replayfix/audit_receipt.json`.

**Acceptance Scenarios**:
1. **Given** a target repository directory (Git or plain directory), **When** `WorktreeManifest::capture` runs, **Then** it computes a canonical SHA-256 digest over all regular files (sorted by normalized relative path, with file size and file SHA-256) and, if `.git` is present, captures `git status --porcelain=v1 -uall`.
2. **Given** a pre-run manifest and post-run manifest of the base repository, **When** no file or porcelain entry changed, **Then** verification succeeds (`base_repo_unmodified_verified: true`) and `base_tree_sha256` is written to the audit receipt.
3. **Given** a pre-run manifest of the base repository, **When** any tracked file, untracked file, or file content in the base repository changes before post-run verification, **Then** the run aborts with status `BASE_REPO_DIRTY_MUTATION_DETECTED` (exit code `11`) and writes `.replayfix/triage_report.json`.
4. **Given** `--output-dir` is inside `--repo` (such as the default `.replayfix`), **When** the manifest is computed, **Then** `.git` internals and the output directory are excluded so writing the receipt itself never triggers a false mutation alarm.

---

### User Story 2 - Ephemeral Scratch Sandbox & Windows Lock Defense (Priority: P2)

As an engineer running verification and repair, I want candidate patches applied and evaluated inside a temporary scratch copy of the repository that is automatically cleaned up when the run ends, so that failed or partial patches leave no residue behind even on Windows where read-only files and child processes can lock directories.

**Why this priority**: All file mutations and upcoming browser/test runs (v0.4+) depend on a clean, disposable sandbox directory.

**Independent Test**: `cargo test -p replayfix-core -- worktree` verifies that `ScratchSandbox` copies the source tree into `std::env::temp_dir()`, applies patches on disk inside the sandbox, rejects any path or symlink escaping the sandbox root, and removes the temporary directory on teardown or `Drop` even when files are marked read-only.

**Acceptance Scenarios**:
1. **Given** a base repository directory, **When** `ScratchSandbox::create` runs, **Then** a unique directory is created in the OS temporary directory containing a copy of the working files, while the base repository remains untouched.
2. **Given** a candidate unified diff, **When** applied to the scratch sandbox, **Then** the target file inside the sandbox is updated on disk, `sandbox_tree_sha256` reflects the patched tree, and `sandbox_tree_sha256 != base_tree_sha256`.
3. **Given** a patch or target path that attempts to traverse outside the sandbox root (`../`) or write through a symlink/path pointing outside the sandbox, **When** applied, **Then** it is rejected with `SECURITY_VIOLATION` (exit code `11`).
4. **Given** a scratch sandbox containing read-only files (such as Git packfiles or protected fixtures), **When** the sandbox is torn down or dropped, **Then** read-only permissions are cleared and the directory is deleted with bounded retry backoff so Windows `os error 5` (Access Denied) does not leak temporary directories.

---

### User Story 3 - Ephemeral Port Allocation & Process Tree Supervision (Priority: P3)

As a developer running local web servers on standard ports (such as `3000` or `5173`), I want ReplayFix to allocate an OS-assigned free TCP port (`:0`) for each run and cleanly terminate any spawned child process tree on exit, so that runs never collide with my active dev server or leave orphan processes behind.

**Why this priority**: Constitutional Law IV requires child-owned ephemeral provenance on port `:0` and clean process teardown before live servers are launched in v0.4.

**Independent Test**: `cargo test -p replayfix-core -- platform` verifies ephemeral port allocation returns a valid non-zero TCP port on `127.0.0.1` and `Platform::terminate_process_tree` terminates a spawned child process cleanly.

**Acceptance Scenarios**:
1. **Given** a verification run starts, **When** `EphemeralPort::allocate` is called, **Then** the OS assigns an available TCP port on `127.0.0.1:0`, returns the non-zero port number, and records `ephemeral_port` in `audit_receipt.json`.
2. **Given** a spawned child process ID, **When** `Platform::terminate_process_tree` is called, **Then** the process and its descendants are terminated (`taskkill /F /T /PID` on Windows, `kill` on Unix) and an already-exited PID is handled idempotently without error.

---

### Edge Cases

- **Missing or non-directory `--repo` path**: If `--repo` does not exist or is a regular file rather than a directory, `replayfix repro --mock-heal` exits `2` with a clear error message naming the path.
- **Unsafe `--repo` path**: Path traversal (`../`), UNC paths, DOS device names, or ADS colons in `--repo` continue to fail closed with `SECURITY_VIOLATION` (exit `11`).
- **Base repository without the fixture file**: When `--repo` points to an empty directory or a repo that does not yet contain `src/cart-summary.js`, the built-in `RF-01` fixture source is materialized *only* inside the scratch sandbox so Gate 1 and Gate 2 execute against real files on disk in the sandbox while the base repository remains completely untouched ($\Delta \text{repo} = 0$).
- **Base repository with a custom copy of `src/cart-summary.js`**: When `--repo` contains `src/cart-summary.js`, the scratch sandbox copies it from `--repo` and runs Gate 1 and Gate 2 against that copied file.
- **Read-only files inside the sandbox**: Teardown clears read-only attributes before removing files so cleanup succeeds on Windows, macOS, and Linux.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `replayfix_core::worktree::WorktreeManifest` MUST recursively walk a directory tree, hash every regular file with SHA-256, and compute a canonical `tree_sha256` digest over the sorted `(path, size, sha256)` entries while excluding `.git` internals and configured output directories.
- **FR-002**: When `.git` is present in the target repository, `WorktreeManifest` MUST also capture `git status --porcelain=v1 -uall` as Layer 2 of the integrity check.
- **FR-003**: `WorktreeManifest::verify_unchanged` MUST compare pre-run and post-run manifests across both Layer 1 (`tree_sha256` and file entries) and Layer 2 (`git_porcelain`), failing closed if any file or status entry drifted.
- **FR-004**: `replayfix_core::worktree::ScratchSandbox` MUST create an isolated directory in `std::env::temp_dir()`, copy the base repository's working files into it, and enforce that all file reads and patch writes stay strictly within the sandbox root boundary.
- **FR-005**: `ScratchSandbox` MUST automatically clean up its temporary directory on teardown and in `Drop`, clearing read-only file attributes and retrying removal with bounded backoff to prevent Windows `os error 5` file-lock failures.
- **FR-006**: `Platform::terminate_process_tree(pid)` MUST terminate a process and its child tree cross-platform and treat an already-exited process as a non-error.
- **FR-007**: `EphemeralPort::allocate()` MUST request an OS-assigned free TCP port on `127.0.0.1:0` and return the non-zero port number.
- **FR-008**: `MockHealer` MUST execute Gate 1, patch application, and Gate 2 against files inside `ScratchSandbox`, verify `pre_manifest == post_manifest` on the base repository before completing, and reject the run with `BASE_REPO_DIRTY_MUTATION_DETECTED` (exit code `11`) if the base repository was modified.
- **FR-009**: `AuditReceipt` (schema version `"0.3"`) MUST include `base_tree_sha256`, `sandbox_tree_sha256`, `base_repo_unmodified_verified`, and `ephemeral_port` in addition to all v0.2 fields.
- **FR-010**: Existing v0.1 and v0.2 CLI flags, exit codes, and workflows MUST remain backward-compatible.

### Key Entities

- **FileEntry / WorktreeManifest**: Deterministic file-tree snapshot (`tree_sha256`, sorted file entries, optional `git_porcelain`).
- **ScratchSandbox**: RAII ephemeral directory in `std::env::temp_dir()` owning the cloned working tree and any registered child PIDs.
- **EphemeralPort**: Dynamically allocated localhost TCP port (`u16`).
- **AuditReceipt / TriageReport**: Terminal JSON artifacts updated to schema `"0.3"` with workspace isolation provenance and `BASE_REPO_DIRTY_MUTATION_DETECTED`.

---

## Success Criteria *(mandatory)*

- **SC-001**: Running `replayfix repro --mock-heal` against any target repository results in zero bytes and zero porcelain lines changed in the base repository ($\Delta \text{repo} = 0$), verified by automated pre/post manifest comparison.
- **SC-002**: 100% of simulated base-repository mutations (modifying a tracked file, modifying an untracked file, creating a stray file, or deleting a file during execution) are detected and rejected with `BASE_REPO_DIRTY_MUTATION_DETECTED` (exit code `11`).
- **SC-003**: 100% of scratch sandboxes created during tests and CLI runs are removed from `std::env::temp_dir()` upon completion, including when files inside the sandbox are marked read-only.
- **SC-004**: `AuditReceipt` includes non-empty, distinct 64-character hex SHA-256 digests for `base_tree_sha256` and `sandbox_tree_sha256`, `base_repo_unmodified_verified: true`, and a non-zero `ephemeral_port`.
- **SC-005**: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace` pass with zero warnings.

---

## Assumptions

- Target repositories for `--mock-heal` are small to moderate source trees; standard build/dependency caches (`node_modules`, `target`, `.git`, `.replayfix`) are excluded from scratch copying and base-tree hashing so snapshotting remains fast and deterministic.
- Dynamic port allocation binds `127.0.0.1:0` to obtain an OS-assigned free port and releases the listener so the port number can be passed to child server commands in Spec 040.
- Real browser execution and server spawning arrive in Spec 040 (v0.4); in v0.3 the port allocator and process-tree terminator are implemented, unit-tested, and recorded in the receipt provenance.

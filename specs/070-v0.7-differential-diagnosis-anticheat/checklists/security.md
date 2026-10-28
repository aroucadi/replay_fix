# Security Checklist: Spec 070 Cryptographic Anti-Cheat & DiffGuard

- [ ] **SEC-070-01: Zero Panic Architecture**: All non-test code compiles under `#![deny(clippy::unwrap_used, clippy::expect_used)]` or passes `scripts/check-panics.ps1` with 0 non-test violations.
- [ ] **SEC-070-02: Path Traversal & Windows Separator Normalization**:
  - All file paths in patches and freeze guards normalized with canonical forward slashes.
  - Reject `..`, absolute drives (`C:\`), and alternate data streams (`::$DATA`).
  - Windows NTFS case-insensitivity: enforce case-folded matching against protected patterns (`tests/`, `__tests__/`, `*.spec.*`, `*.test.*`).
- [ ] **SEC-070-03: Reparse Point & Symlink Defense**:
  - Enforce reparse point / symlink detection on all scratch workspace targets before read or write.
  - Abort immediately if a patch target is a reparse point or symlink pointing outside the scratch workspace.
- [ ] **SEC-070-04: Surgical `package.json` Test Guard**:
  - When `package.json` is modified by a patch, extract and compare `"scripts"` keys starting with `test` (e.g. `test`, `test:unit`, `test:e2e`).
  - If any test script value is added, altered, or removed, reject with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED: package.json test scripts altered")`.
  - Non-test keys (e.g., dependencies, other scripts) may be modified only if verified safe.
- [ ] **SEC-070-05: Subprocess Isolation & Command Injection**:
  - All test runner commands executed with bounded timeouts via `tokio::process::Command` without shell interpretation (`cmd.exe /c` or `sh -c` avoided unless strictly isolated).
  - Environment sanitization: isolate temp directory and avoid leaking sensitive parent environment variables.
- [ ] **SEC-070-06: Checkpoint Integrity & Anti-Tamper**:
  - Reject `session_state.json` if `base_repo_head` != current git HEAD or `manifest_sha256` != SHA-256 of active task manifest.
  - Reject checkpoint if wall clock epoch > `created_at_epoch + ttl_seconds`.

# Security Checklist: ReplayFix v0.8.0

- [ ] **SEC-001 (Untrusted Data Isolation)**: Issue titles and bodies are enclosed in `<UNTRUSTED_ISSUE_BODY>` blocks and never passed directly into bash/shell executors.
- [ ] **SEC-002 (URL Validation & SSRF Prevention)**: `--issue` URL is strictly matched against `https://github.com/[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+/issues/\d+`. Localhost, `file://`, internal IPs, and non-GitHub domains are rejected.
- [ ] **SEC-003 (Process Separation & Detachment)**: `npx playwright show-trace` is spawned with null stdin/stdout/stderr so it cannot hang the terminal or intercept parent IPC.
- [ ] **SEC-004 (Fail-Closed Non-Interactive Mutation)**: If standard input is not a TTY and `--yes` is not passed, patch application to base repo is rejected with exit code 11.
- [ ] **SEC-005 (Patch Integrity Verification)**: Patch content presented to user matches the SHA-256 hash written in `approval_audit.json` and applied to the working tree.
- [ ] **SEC-006 (Zero Unsafe & Zero Panic)**: Production code has 0 `unsafe` blocks and 0 unhandled `.unwrap()` / `.expect()`.

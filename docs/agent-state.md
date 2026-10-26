# Agent Living State — ReplayFix

## 1. Project Phase & Active Task
- **Active Milestone**: `v0.1.1` (Walking Skeleton & CSO Security Hardened)
- **Status**: Ready for Spec 020 (`020-v0.2-mock-healer-two-gates`)
- **Active Gate**: v1.0.0 Shipping-Readiness Assessment Complete

## 2. Verified Milestones & Empirical Evidence
- [x] **v0.0.0 Genesis Baseline**: Pristine 2-crate root Cargo workspace, platform abstraction, toolchain lock-in, clean-room provenance.
- [x] **v0.1.0 Walking Skeleton**:
  - `replayfix doctor` deterministically checks Git, Node.js, Playwright, and FFmpeg with semantic version validation.
  - Actionable remediation advice and machine-readable `--json` output.
  - `TaskManifest` and `EnvironmentAdapter` strictly conforming to canonical `task_manifest.schema.json`.
  - Dual CLI/JSON file manifest ingestion in `replayfix repro`.
  - Security boundary defense: path traversal detection rejecting `../` escapes with code 11.
- [x] **v0.1.1 CSO Security Hardening & Cross-Platform Refinement**:
  - UNC path rejection (`\\host\share`, `//host/share`) preventing SMB credential coercion / NTLM theft.
  - Windows reserved DOS device name filtering (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`) across all path components.
  - Alternate Data Stream (ADS) colon inspection preventing stream injection.
  - Cross-platform manifest path backslash normalization (`\` $\rightarrow$ `/`) preventing traversal bypasses on POSIX systems.
  - Refined `OsFamily` taxonomy (`Windows`, `MacOS`, `Linux`, `OtherUnix`) and platform-adaptive doctor suggestions (`winget`, `brew`, `apt`/`dnf`).
  - Regex `OnceLock` singleton optimization across all modules.
  - Clean native test execution without PowerShell wrappers, resolving Smart App Control toast warnings.
  - Standalone signing scripts (`scripts/setup-dev-cert.ps1` and `scripts/sign-binary.ps1`).
  - 17/17 automated unit, security, and cross-platform tests passing in 0.18s across workspace.

## 3. Toolchain & Environment
- **Host OS**: Windows 11 (MSVC native)
- **Rustc**: `rustc 1.99.1 (58cf8f9c3 2026-10-13)` (target: `x86_64-pc-windows-msvc`)
- **Cargo**: `cargo 1.99.1`
- **Node.js**: `v24.19.0`
- **Git**: `2.55.0.windows.1`
- **FFmpeg**: `7.1.1`

## 4. Release Gates & v1.0.0 Readiness Verdict
- **Current Published Release**: `v0.1.1`
- **v1.0.0 Shipping Status**: **BLOCKED** pending implementation and verification of Specs 020 through 090.
- **Blocking Deliverables for v1.0.0**:
  1. Spec 020: Dual-gate FSM state machine
  2. Spec 030: Ephemeral worktree isolation ($\Delta\text{repo}=0$)
  3. Spec 040: Playwright CDP spatial grounding
  4. Spec 050: Video telemetry & SAM 3.1 perception
  5. Spec 060: Meta Muse Wonder AST repair
  6. Spec 070: SHA-256 test freezing & 2-run anti-cheat pass
  7. Spec 080: Headed observability suite
  8. Spec 090: RFC 6962 Merkle tree receipts & 5-bug benchmark sweep on `MetaPulse`

## 5. Next Immediate Actionable Task
- Advance to Spec 020: `020-v0.2-mock-healer-two-gates`.

# ReplayFix

ReplayFix is a deterministic Rust-based verification harness designed to reproduce, isolate, and repair visual defects in web applications directly from video recordings and bug reports. Every run is bound by strict causal verification, repository isolation ($\Delta\text{repo} = 0$), and tamper-proof cryptographic audit receipts.

---

## Current Status: v0.1.1 (Walking Skeleton & Security Hardened)

ReplayFix is engineered from the ground up under a strict clean-room protocol and an incremental vertical micro-slice methodology. Every release compiles cleanly, passes automated test suites, and provides verifiable empirical evidence.

- **Current Verified Deliverable**: **v0.1.1** (Environment diagnostics, canonical task manifest validation, and CSO security hardening).
- **Target v1.0.0 Milestone**: Complete autonomous pipeline featuring SAM 3.1 visual grounding, dual-worktree isolation, Meta Muse Wonder AST repair, 2-run anti-cheat stability passes, and RFC 6962 Merkle audit receipts.

---

## Workspace Architecture

ReplayFix maintains a clean, separated 2-crate workspace:

- **`crates/replayfix-core`**: The verification engine containing platform abstractions, multi-dependency diagnostics (`Doctor`), domain schemas (`TaskManifest`), security path validators, and future FSM state machines.
- **`crates/replayfix-cli`**: The user-facing command-line interface (`replayfix`) providing formatted terminal diagnostics, JSON output for CI pipelines, task ingestion, and headed replay visualization.

---

## Prerequisites & System Requirements

ReplayFix runs natively on Windows (MSVC), macOS, and Linux.

### Build Requirements
- **Rust Toolchain**: `1.80+` (tested and verified on `1.99.1`)
- **C/C++ Compiler**:
  - Windows: Visual Studio C++ Build Tools (`x86_64-pc-windows-msvc`)
  - Linux/macOS: GCC or Clang

### Runtime Dependencies
ReplayFix integrates with standard industry tools to perform browser automation and media analysis:
- **Git** ($\ge 2.30.0$): For worktree branching and diff calculation.
- **Node.js** ($\ge 18.0.0$): Runtime for modern web applications and Playwright drivers.
- **Playwright** ($\ge 1.40.0$): Headless and headed browser execution and CDP telemetry.
- **FFmpeg** ($\ge 4.4.0$): Video frame extraction and audio track processing.

---

## Quickstart & CLI Commands

### 1. Check System Health (`replayfix doctor`)

Before running reproduction tasks, verify that all host tools and required versions are available:

```bash
# Pretty-printed terminal status table
cargo run --release -p replayfix-cli -- doctor

# Machine-readable JSON output for automated CI/CD runners
cargo run --release -p replayfix-cli -- doctor --json
```

**Sample Output:**
```text
=========================================================================================
                              ReplayFix Doctor Diagnostics                               
=========================================================================================
Host OS    : Windows
Checked At : 2026-10-26T17:19:14+00:00
-----------------------------------------------------------------------------------------
Dependency   Executable   Found Version    Required         Status    
-----------------------------------------------------------------------------------------
Git          git          2.55.0           2.30.0           [OK]
Node.js      node         24.19.0          18.0.0           [OK]
Playwright   playwright   1.54.1           1.40.0           [OK]
FFmpeg       ffmpeg       7.1.1            4.4.0            [OK]
-----------------------------------------------------------------------------------------
Result: ALL SYSTEM DEPENDENCIES SATISFIED (System Ready for Reproduction)
```

If any dependency is missing or outdated, ReplayFix prints exact, actionable commands to install or upgrade it, exiting with code `1`.

---

### 2. Ingest Reproduction Tasks (`replayfix repro`)

ReplayFix accepts defect reproduction tasks defined by the canonical `task_manifest.schema.json` format.

#### Option A: Direct Command-Line Flags
```bash
cargo run --release -p replayfix-cli -- repro \
  --task-id RF-01 \
  --title "Shopping Cart Null Pointer Dereference" \
  --repo ./target-app \
  --bug-ref bug/RF-01 \
  --clean-ref main \
  --video recordings/RF-01.mp4 \
  --headed \
  --trace
```

#### Option B: Manifest File
```bash
cargo run --release -p replayfix-cli -- repro --manifest task_manifest.json
```

**Verification Output:**
```text
=========================================================================================
                           ReplayFix Task Ingestion Verified                             
=========================================================================================
Task ID       : RF-01
Title         : Shopping Cart Null Pointer Dereference
Target Repo   : ./target-app
Bug Ref       : bug/RF-01
Clean Ref     : main
Video Source  : recordings/RF-01.mp4
Headed Mode   : true
Trace Export  : true
Manifest State: VALIDATED (Schema compliant with task_manifest.schema.json)
-----------------------------------------------------------------------------------------
Status: Ready for Gate 1 Baseline Execution (Slice 010 Walking Skeleton)
```

---

## Built-in Security Guarantees

Following a comprehensive Chief Security Officer (CSO) audit, ReplayFix enforces strict defensive validation on all user inputs:

1. **Path Traversal Protection**: Inputs containing directory traversal sequences (`../`) that escape root boundaries are rejected with `SECURITY_VIOLATION` (exit code 11).
2. **UNC Path Injection Defense**: UNC paths (`\\server\share` or `//server/share`) are disallowed to prevent Windows SMB credential reflection attacks (NTLM theft).
3. **DOS Device Name Filtering**: Reserved Windows device names (such as `CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`) are blocked to prevent OS process hangs or descriptor hijacking.
4. **Regex DoS Mitigation**: Regex evaluation across diagnostics and manifests uses thread-safe `std::sync::OnceLock` singletons, preventing repeated recompilation churn.

---

## Development & Automated Tests

Run the full workspace unit and integration test suite:

```bash
cargo test --workspace
```

*Note for Windows users with Smart App Control (SAC) enabled:* ReplayFix includes runner automation in `.cargo/config.toml` that signs debug test binaries with a local developer certificate, ensuring smooth execution without code integrity blocks.

---

## Roadmap to v1.0.0

| Milestone | Scope & Deliverable | Status |
| :--- | :--- | :--- |
| **v0.0.0** | Clean-room Cargo workspace genesis & platform abstractions | **Delivered** |
| **v0.1.0** | Walking skeleton: `doctor` diagnostics & canonical manifest ingestion | **Delivered** |
| **v0.1.1** | CSO security hardening: UNC defense, DOS device checks, regex singletons | **Delivered** |
| **v0.2.0** | Dual-gate FSM state machine (`Init` $\rightarrow$ `Gate1Red` $\rightarrow$ `Repair` $\rightarrow$ `Gate2Green`) | Next Up |
| **v0.3.0** | Ephemeral dual-worktree repository isolation ($\Delta\text{repo}=0$) & dynamic port binding | Planned |
| **v0.4.0** | Playwright CDP spatial grounding, `ariaSnapshot`, and coordinate snapping | Planned |
| **v0.5.0** | Video telemetry ingestion, FFmpeg keyframe extraction & SAM 3.1 centroids | Planned |
| **v0.6.0** | Meta Muse Wonder bounded AST repair & patch application | Planned |
| **v0.7.0** | Differential diagnosis, SHA-256 test freezing & 2-run stability pass | Planned |
| **v0.8.0** | Full observability suite, headed replay mode & Playwright trace export | Planned |
| **v0.9.0 / v1.0.0** | RFC 6962 Merkle tree audit receipt generation, Ed25519 signing & 5-bug benchmark sweep | Planned |

---

## License

This project is licensed under the [MIT License](LICENSE.md). See [CHANGELOG.md](CHANGELOG.md) for detailed version history.

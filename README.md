# ReplayFix

Deterministic Rust-based verification harness for visual bug reproduction, causal verification, constrained repair, regression testing, and cryptographic audit reporting for modern web applications.

## Status: v0.1.0 (The Walking Skeleton)

ReplayFix is built under a clean-room protocol for the Meta Global AI Developer Hackathon. The codebase follows an Elephant Carpaccio vertical micro-slice methodology, ensuring that every commit compiles, passes automated tests, and delivers verifiable empirical evidence.

Current maturity: **v0.1.0 Walking Skeleton** (comprehensive multi-dependency environment diagnostics and canonical task manifest ingestion).

## Architecture

ReplayFix enforces a strict 2-crate architecture:

- **`crates/replayfix-core`**: Core verification harness, platform abstractions, environment diagnostics (`doctor`), and task manifest schemas (`manifest`).
- **`crates/replayfix-cli`**: Command-line interface (`replayfix`) providing diagnostics reporting, manifest ingestion, and headed replay execution.

## Prerequisites & Dependencies

- **Rust**: `1.80+` (compiled and verified on `1.99.1`)
- **Native Host Toolchain**:
  - Windows: MSVC C++ Build Tools (`x86_64-pc-windows-msvc`)
  - macOS / Linux: Supported native C/C++ toolchain
- **Runtime Dependencies**:
  - **Git**: $\ge 2.30.0$
  - **Node.js**: $\ge 18.0.0$
  - **Playwright**: $\ge 1.40.0$
  - **FFmpeg**: $\ge 4.4.0$

## CLI Usage & Verification

### 1. Environment Diagnostics (`replayfix doctor`)

Probe the host system to verify whether required dependencies are present and compatible:

```bash
# Formatted human-readable diagnostics table
cargo run --release -p replayfix-cli -- doctor

# Machine-readable JSON output for automated CI pipelines
cargo run --release -p replayfix-cli -- doctor --json
```

Example diagnostic output:

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

If a dependency is missing, ReplayFix displays actionable installation commands and exits with code 1.

---

### 2. Task Manifest Ingestion (`replayfix repro`)

ReplayFix ingests reproduction task definitions adhering strictly to the canonical `task_manifest.schema.json`.

#### Option A: Direct CLI Arguments
```bash
cargo run --release -p replayfix-cli -- repro \
  --task-id RF-01 \
  --title "Null Cart Dereference" \
  --repo ./target-app \
  --bug-ref bug/RF-01 \
  --clean-ref main \
  --video recordings/RF-01.mp4 \
  --headed \
  --trace
```

#### Option B: JSON Manifest File
```bash
cargo run --release -p replayfix-cli -- repro --manifest task_manifest.json
```

#### Security Boundary Enforcement
ReplayFix sanitizes all input paths and strictly rejects directory traversal attacks (`../` escaping) with exit code 11 (`SECURITY_VIOLATION`).

---

## Current Scope & Limitations

In `v0.1.0`, the `repro` command validates and hydrates the canonical `TaskManifest` and confirms readiness for execution. Full Gate 1 Playwright browser execution, dual-worktree isolation, and Meta model repair will be connected in subsequent vertical micro-slices (`v0.2.0`+).

---

## Running the Automated Test Suite

```bash
# Run all unit and integration tests across both crates
cargo test --workspace
```

---

## License

Licensed under the [MIT License](LICENSE.md). See [CHANGELOG.md](CHANGELOG.md) for version history.

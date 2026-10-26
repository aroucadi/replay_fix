# Quickstart Guide: 000-v0.0-workspace-genesis

**Feature**: Clean-Room Workspace Genesis & Toolchain Lock-In  
**Date**: 2026-10-26  

---

## 1. Prerequisites

- **Rust Toolchain**: `rustc` and `cargo` installed ($\ge 1.80$, recommended $\ge 1.90$).
- **C Compiler / Host Toolchain**:
  - Windows: MSVC C++ Build Tools (native target `x86_64-pc-windows-msvc`).
  - macOS: Xcode Command Line Tools.
  - Linux: `build-essential` or GCC.
- **Git**: Installed and available in PATH.

---

## 2. Verification Scenarios

### Step 1: Check Workspace Compilation
Verify that all workspace members resolve and compile without warnings or errors:

```bash
cargo check --workspace
```
*Expected Outcome*: Exits with code 0 in $< 60$s.

---

### Step 2: Run Unit Tests
Execute the unit test suite across both crates:

```bash
cargo test --workspace
```
*Expected Outcome*: Exits with code 0; `platform::tests::test_platform_os_family` passes.

---

### Step 3: Run the CLI Binary
Run the CLI binary to inspect version and basic diagnostics:

```bash
cargo run --release -p replayfix-cli -- --version
```
*Expected Outcome*: Prints `replayfix 0.1.0` and exits with code 0.

```bash
cargo run --release -p replayfix-cli -- doctor
```
*Expected Outcome*:
```text
ReplayFix Doctor: checking environment dependencies...
Host OS: Windows
Status: Initialized (Slice 000 Genesis)
```
Exits with code 0.

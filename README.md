# ReplayFix

ReplayFix is a deterministic Rust harness for reproducing, verifying and repairing visual bugs in web applications. The long-term design is simple: a bug is only "fixed" when the same behavioral check goes from RED to GREEN, inside an isolated scratch workspace, with a receipt you can audit.

> **Where the project stands (v0.3.3).** ReplayFix is built in small, verified slices. Today it checks your environment, validates bug-reproduction tasks, clones your target repository into an **isolated ephemeral scratch sandbox**, verifies **dual-layer repository immutability ($\Delta \text{repo} = 0$)**, allocates **dynamic ephemeral ports (`127.0.0.1:0`)**, and runs the full two-gate loop (Gate 1 RED → repair → Gate 2 GREEN) against a **simulated oracle**. It does **not** yet drive a browser, read video, or call a model. Those arrive in later releases (see the roadmap).

---

## Workspace

- **`crates/replayfix-core`** — the engine: platform helpers (`EphemeralPort`, process tree termination), environment diagnostics (`Doctor`), task manifest validation (`TaskManifest`), dual-layer workspace integrity (`WorktreeManifest`) and scratch isolation (`ScratchSandbox`), the phase state machine (`PhaseMachine`), the unified-diff patcher (`UnifiedPatcher`), the simulated oracle (`MockHealer`), and receipt writers (`AuditReceipt`, `TriageReport`).
- **`crates/replayfix-cli`** — the `replayfix` command-line tool.

---

## Prerequisites

ReplayFix builds and runs on Windows (MSVC), macOS and Linux.

**To build:** Rust 1.80 or newer (tested on 1.99.1) and a C/C++ toolchain (Visual Studio Build Tools on Windows, GCC or Clang elsewhere).

**To run `doctor` successfully** you need these tools on your `PATH`. `doctor` tells you exactly what is missing and how to install it for your platform:

| Tool | Minimum | Used for (current or planned) |
| :--- | :--- | :--- |
| Git | 2.30 | worktree status (`git status --porcelain`) and diffs |
| Node.js | 18 | target apps and Playwright |
| Playwright | 1.40 | browser replay |
| FFmpeg | 4.4 | video keyframes |

The v0.3 mock-heal loop runs without Node.js, Playwright, or FFmpeg (and uses `git status --porcelain` when the target directory is a Git repository).

---

## Quickstart

```bash
cargo build --release
# binary: target/release/replayfix (replayfix.exe on Windows)
```

### Check your environment: `replayfix doctor`

```bash
replayfix doctor          # readable table, exit code 1 if something is missing
replayfix doctor --json   # machine-readable report for CI
```

```text
Dependency   Executable   Found Version    Required         Status
Git          git          2.55.0           2.30.0           [OK]
Node.js      node         24.19.0          18.0.0           [OK]
Playwright   playwright   1.64.1           1.40.0           [OK]

FFmpeg       ffmpeg       15.2             4.4.0            [OK]
```

### Describe a bug: `replayfix repro`

`repro` accepts either flags or a manifest that follows `task_manifest.schema.json`.

```bash
# Flags
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 \
  --title "Cart count crash" --repo ./target-app --clean-ref main

# Manifest file
replayfix repro --manifest task.json
```

| Flag | Meaning | Default |
| :--- | :--- | :--- |
| `--manifest <FILE>` | JSON task manifest (alternative to the flags below) | — |
| `--task-id`, `--bug-ref`, `--video` | Required unless `--manifest` is given | — |
| `--title`, `--repo`, `--clean-ref`, `--audio` | Optional task metadata | `--repo .` |
| `--server-cmd`, `--port-env-var`, `--healthcheck-path`, `--startup-timeout-ms` | How to launch the target app | `PORT`, `/api/health`, `10000` |
| `--headed`, `--trace` | Visible browser / trace export (used once browser replay exists) | off |
| `--mock-heal` | Run the simulated two-gate loop inside an isolated scratch sandbox (below) | off |
| `--mock-patch <builtin\|noop\|FILE>` | Candidate patch for `--mock-heal` (needs `--mock-heal`) | `builtin` |
| `--output-dir <DIR>` | Where results are written (needs `--mock-heal`) | `.replayfix` |
| `--dry-run` | Validate environment, manifests, and patch invariants without execution | off |
| `--color <auto\|always\|never>` | Control colored output formatting | `auto` |

Without `--mock-heal`, `repro` validates the task and stops.

### Try the isolated two-gate loop: `--mock-heal`

```bash
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --mock-heal
```

```text
Sandbox  : isolated scratch workspace (ephemeral port :51234, base tree 8f1c9a20b4e1)
Gate 1   : RED   (defect reproduced; contract 400bc991f224)
Repair   : patch applied in scratch sandbox to src/cart-summary.js (+1 -1, sandbox tree 3a7d11e09c44)
Gate 2   : GREEN (same contract 400bc991f224, base repo unmodified: true)
-----------------------------------------------------------------------------------------
Result   : VERIFIED_REPAIRED (simulated oracle; no browser was run)
Receipt  : .replayfix/audit_receipt.json
```

What it does:

1. **Pre-run snapshot**: `WorktreeManifest` records a dual-layer fingerprint of `--repo` (Layer 1 recursive SHA-256 content tree + Layer 2 `git status --porcelain=v1 -uall`).
2. **Scratch isolation & port allocation**: `ScratchSandbox` copies the working tree into an OS temporary directory and `EphemeralPort` binds `127.0.0.1:0` to allocate a collision-free loopback port.
3. **Gate 1 (RED)**: Evaluates the target source file inside the scratch sandbox (`RF-01`, an unguarded `cart.items.length`) and requires a **RED** baseline.
4. **Repair**: Applies the candidate unified diff to the file **on disk inside the scratch sandbox** and computes `sandbox_tree_sha256`.
5. **Gate 2 (GREEN)**: Reads the patched file from the scratch sandbox and evaluates the **same contract** (verified by SHA-256 of its canonical JSON).
6. **Post-run $\Delta \text{repo} = 0$ check & teardown**: Re-verifies that the base repository's `WorktreeManifest` is untouched (`BASE_REPO_DIRTY_MUTATION_DETECTED`, exit code 11 if mutated), terminates any registered child process trees, and removes the temporary scratch directory.
7. **Single terminal artifact**: Writes `audit_receipt.json` (schema `"0.3"`) on success, or `triage_report.json` on rejection, removing any stale artifact from prior runs.

Try breaking it on purpose:

```bash
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video v.mp4 --mock-heal --mock-patch noop
# Result   : REPAIR_NOOP_REJECTED (repair rejected), exit code 30
```

Exit codes: `0` verified, `2` bad input or usage, `10` Gate 1 was not RED or the contract changed, `11` security violation or base repository mutation (`BASE_REPO_DIRTY_MUTATION_DETECTED`), `30` repair rejected (empty patch, no-op, patch did not apply, or Gate 2 not green).

**Limits to know about**

- Only task `RF-01` has a built-in scenario; other task ids exit with a clear message.
- The "oracle" inspects the source file inside the scratch sandbox (`src/cart-summary.js`). It proves the state machine, scratch isolation, $\Delta \text{repo} = 0$ enforcement, contract invariance, and receipt format, not real browser behavior. Every receipt says so.
- `--mock-patch FILE` accepts a single-file unified diff up to 1 MiB; workspace files copied into the scratch sandbox are capped at 32 MiB per file.

---

## Safety checks on inputs & workspace isolation

Paths from flags, manifests, patch headers, sandbox file operations, and `--output-dir` go through strict boundary validation and fail with `SECURITY_VIOLATION` (exit 11):

- directory traversal that escapes the working directory or scratch sandbox (`../..`)
- symlink escape chains inside the scratch sandbox
- UNC network paths (`\\server\share`, `//server/share`)
- Windows reserved device names (`CON`, `NUL`, `COM1`…) in any path segment
- NTFS alternate data streams (`file:stream`)
- unauthorized modification of the user's base repository during a run (`BASE_REPO_DIRTY_MUTATION_DETECTED`)

Windows backslashes are normalized, so a manifest written on Windows is checked the same way on macOS and Linux. Environment variable names and healthcheck paths in the manifest are validated too.

---

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

Tests run natively on every platform with no custom runner.

**Windows 11 with Smart App Control?** Newly built DLLs and executables can be blocked. The repo ships opt-in helper scripts (including `scripts/sign-build-outputs.ps1`, which signs whatever is still unsigned after a build) and a troubleshooting guide: [docs/windows-code-signing.md](docs/windows-code-signing.md).

---

## Roadmap

| Release | Scope | Status |
| :--- | :--- | :--- |
| v0.0.0 | Workspace genesis | Delivered |
| v0.1.0 / v0.1.1 | `doctor`, manifest ingestion, input hardening | Delivered |
| v0.2.0 / v0.2.1 | Dual-gate state machine, unified patcher, no-op guard, receipts (simulated oracle) | Delivered |
| **v0.3.0 – v0.3.3** | **Isolated scratch sandboxes, $\Delta \text{repo} = 0$ verification, ephemeral ports, process tree cleanup, terminal UX & release quality gate** | **Delivered** |
| v0.4.0 | Playwright grounding with real browser gates | Planned |
| v0.5.0 | Video keyframes and SAM 3.1 perception | Planned |
| v0.6.0 | Model-driven bounded repair | Planned |
| v0.7.0 | Test freezing, two-run stability, regression parity | Planned |
| v0.8.0 | Headed replay, traces, evidence matrix | Planned |
| v0.9.0 / v1.0.0 | Merkle-tree signed receipts, five-bug benchmark | Planned |

---

## License

MIT. See [LICENSE.md](LICENSE.md). Engineering guidelines: [docs/engineering-principles-rust.md](docs/engineering-principles-rust.md). Release history: [CHANGELOG.md](CHANGELOG.md) and [docs/release-notes](docs/release-notes/).

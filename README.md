# ReplayFix

ReplayFix is a deterministic Rust harness for reproducing, verifying and repairing visual bugs in web applications. The long-term design is simple: a bug is only "fixed" when the same behavioral check goes from RED to GREEN, inside an isolated scratch workspace, with a receipt you can audit.

> **Where the project stands (v0.4.1).** ReplayFix is built in small, verified slices. Today it checks your environment, validates bug-reproduction tasks, clones your target repository into an **isolated ephemeral scratch sandbox**, verifies **dual-layer repository immutability ($\Delta \text{repo} = 0$)**, allocates **dynamic ephemeral ports (`127.0.0.1:0`)**, and runs the full two-gate loop (Gate 1 RED → repair → Gate 2 GREEN) either in a **live Playwright Chromium browser (`--live-browser` / `--headed`)** with **spatial-semantic CDP grounding (`elementFromPoint` + `ariaSnapshot`), fail-closed ambiguity abstention (`AMBIGUOUS_GROUNDING`, exit code 22), and deadline-bounded IPC**, or against a fast **simulated oracle (`--mock-heal`)**. Video keyframe ingestion and model-driven repair arrive in later releases (see the roadmap).

---

## Workspace

- **`crates/replayfix-core`** — the engine: platform helpers (`EphemeralPort`, process tree termination), environment diagnostics (`Doctor`), task manifest validation (`TaskManifest`), dual-layer workspace integrity (`WorktreeManifest`) and scratch isolation (`ScratchSandbox`), spatial-semantic locator grounding (`LocatorResolver`, `GroundingTelemetry`), live browser bridge (`SandboxServer`, `PlaywrightBridge`), the phase state machine (`PhaseMachine`), the unified-diff patcher (`UnifiedPatcher`), the verification engine (`MockHealer`), and receipt writers (`AuditReceipt`, `TriageReport`).
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
| Playwright | 1.40 | browser replay (`--live-browser`) |
| FFmpeg | 4.4 | video keyframes |

The `--mock-heal` simulated loop runs without Node.js, Playwright, or FFmpeg; `--live-browser` requires Node.js and Playwright with Chromium installed.

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
| `--headed`, `--trace` | Visible Chromium window (`--headed` with `--live-browser`) / trace export | off |
| `--live-browser` | Run the two-gate loop in a live Playwright Chromium browser with CDP spatial-semantic grounding | off |
| `--mock-ambiguous` | Inject a synthetic candidate tie ($|\Delta S| < 0.25$) to test `AMBIGUOUS_GROUNDING` (needs `--live-browser`) | off |
| `--mock-heal` | Run the simulated two-gate loop inside an isolated scratch sandbox (below) | off |
| `--mock-patch <builtin\|noop\|FILE>` | Candidate patch for `--live-browser` or `--mock-heal` | `builtin` |
| `--output-dir <DIR>` | Where results are written (needs `--live-browser` or `--mock-heal`) | `.replayfix` |
| `--dry-run` | Validate environment, manifests, and patch invariants without execution | off |
| `--color <auto\|always\|never>` | Control colored output formatting | `auto` |

Without `--live-browser` or `--mock-heal`, `repro` validates the task and stops.

### Run live Playwright browser verification: `--live-browser`

```bash
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --live-browser
# Add --headed to watch the Chromium window replay Gate 1 (RED) and Gate 2 (GREEN)
```

```text
  ✔ Isolation [PASS] Scratch workspace initialized
    › ephemeral port :51587, base tree d1639803b7d7
  ✔ CDP Grounding [PASS] Grounded target via elementFromPoint + ariaSnapshot
    › page.getByRole('button', { name: 'Refresh Cart', exact: true }) (score: 1.00, margin: 0.95)
  ✔ Gate 1 (RED) [PASS] Defect reproduced causal baseline
    › contract digest: 400bc991f224
  ✔ Repair [PASS] Patch applied in scratch sandbox
    › target: src/cart-summary.js (+1 -1), sandbox tree: cbccadc3bd21
  ✔ Gate 2 (GREEN) [PASS] Behavior verified on patched source
    › contract: 400bc991f224, base repo unmodified: true
```

In `--live-browser` mode, ReplayFix serves the isolated `ScratchSandbox` over `127.0.0.1:<ephemeral_port>`, launches Chromium over a flushed JSON-lines stdio bridge (`PlaywrightBridge`), reverse-probes `(click_x, click_y)` via `document.elementFromPoint` and `.closest()`, verifies the **DOM Grounding Invariant** against Playwright's `ariaSnapshot`, and abstains with `AMBIGUOUS_GROUNDING` (exit code `22`) if the top two candidates score within `0.25` (`2500` bps).

### Try the isolated two-gate loop (simulated oracle): `--mock-heal`

```bash
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --mock-heal
```

What the two-gate loop does:

1. **Pre-run snapshot**: `WorktreeManifest` records a dual-layer fingerprint of `--repo` (Layer 1 recursive SHA-256 content tree + Layer 2 `git status --porcelain=v1 -uall`).
2. **Scratch isolation & port allocation**: `ScratchSandbox` copies the working tree into an OS temporary directory and `EphemeralPort` binds `127.0.0.1:0` to allocate a collision-free loopback port.
3. **Gate 1 (RED)**: Evaluates the target inside the scratch sandbox (in live Chromium with `--live-browser`, or via static source inspection with `--mock-heal`) and requires a **RED** baseline.
4. **Repair**: Applies the candidate unified diff to the file **on disk inside the scratch sandbox** and computes `sandbox_tree_sha256`.
5. **Gate 2 (GREEN)**: Re-evaluates the patched sandbox against the **same contract** (verified by SHA-256 of its canonical JSON).
6. **Post-run $\Delta \text{repo} = 0$ check & teardown**: Re-verifies that the base repository's `WorktreeManifest` is untouched (`BASE_REPO_DIRTY_MUTATION_DETECTED`, exit code 11 if mutated), terminates any registered child process trees, and removes the temporary scratch directory.
7. **Single terminal artifact**: Writes `audit_receipt.json` (schema `"0.4"`) on success, or `triage_report.json` on rejection, removing any stale artifact from prior runs.

Try breaking it on purpose:

```bash
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video v.mp4 --mock-heal --mock-patch noop
# Result   : REPAIR_NOOP_REJECTED (repair rejected), exit code 30

replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video v.mp4 --live-browser --mock-ambiguous
# Result   : AMBIGUOUS_GROUNDING (ambiguity guard abstention), exit code 22
```

Exit codes: `0` verified, `2` bad input or usage, `10` Gate 1 was not RED or the contract changed, `11` security violation or base repository mutation (`BASE_REPO_DIRTY_MUTATION_DETECTED`), `20` browser bridge execution failure (`BRIDGE_EXECUTION_FAILED`), `22` ambiguous or ungrounded DOM target (`AMBIGUOUS_GROUNDING`), `30` repair rejected (empty patch, no-op, patch did not apply, or Gate 2 not green).

**Limits to know about**

- Only task `RF-01` has a built-in scenario; other task ids exit with a clear message.
- `--live-browser` executes real Gate 1 (RED) and Gate 2 (GREEN) inside Chromium against the isolated scratch sandbox; `--mock-heal` inspects `src/cart-summary.js` statically for fast offline runs. Every receipt (`schema_version: "0.4"`) records its `substrate` (`"playwright-cdp"` or `"mock-healer"`).
- `--mock-patch FILE` accepts a single-file unified diff up to 1 MiB; workspace files copied into the scratch sandbox are capped at 32 MiB per file.

---

## Safety checks on inputs & workspace isolation

Paths from flags, manifests, patch headers, sandbox file operations, HTTP requests to `SandboxServer`, and `--output-dir` go through strict boundary validation and fail closed:

- directory traversal that escapes the working directory or scratch sandbox (`../..`)
- symlink escape chains inside the scratch sandbox
- UNC network paths (`\\server\share`, `//server/share`)
- Windows reserved device names (`CON`, `NUL`, `COM1`…) in any path segment
- NTFS alternate data streams (`file:stream`)
- unauthorized modification of the user's base repository during a run (`BASE_REPO_DIRTY_MUTATION_DETECTED`)
- ungrounded or ambiguous DOM locators (`AMBIGUOUS_GROUNDING`, exit code 22) and banned positional `.first()` / `:nth-match` selectors

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
| v0.3.0 – v0.3.3 | Isolated scratch sandboxes, $\Delta \text{repo} = 0$ verification, ephemeral ports, process tree cleanup, terminal UX & release quality gate | Delivered |
| **v0.4.0 / v0.4.1** | **Playwright CDP spatial-semantic grounding, `ariaSnapshot` DOM Grounding Invariant, fail-closed ambiguity guard (`AMBIGUOUS_GROUNDING`), live Chromium browser gates (`--live-browser`), and adversarial systems hardening** | **Delivered** |
| v0.5.0 | Video keyframes and SAM 3.1 perception | Planned |
| v0.6.0 | Model-driven bounded repair | Planned |
| v0.7.0 | Test freezing, two-run stability, regression parity | Planned |
| v0.8.0 | Headed replay, traces, evidence matrix | Planned |
| v0.9.0 / v1.0.0 | Merkle-tree signed receipts, five-bug benchmark | Planned |

---

## License

MIT. See [LICENSE.md](LICENSE.md). Engineering guidelines: [docs/engineering-principles-rust.md](docs/engineering-principles-rust.md). Release history: [CHANGELOG.md](CHANGELOG.md) and [docs/release-notes](docs/release-notes/).

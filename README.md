# ReplayFix

ReplayFix is a deterministic Rust harness for reproducing, verifying and repairing visual bugs in web applications. The long-term design is simple: a bug is only "fixed" when the same behavioral check goes from RED to GREEN, in isolation, with a receipt you can audit.

> **Where the project stands (v0.2.0).** ReplayFix is built in small, verified slices. Today it can check your environment, validate bug-reproduction tasks, and run the full two-gate loop (Gate 1 RED → repair → Gate 2 GREEN) against a **simulated oracle**. It does **not** yet drive a browser, read video, call a model or isolate your repository. Those arrive in later releases (see the roadmap).

---

## Workspace

- **`crates/replayfix-core`** — the engine: platform helpers, environment diagnostics, task manifest validation, the phase state machine, the in-memory patcher, the simulated oracle and the receipt writers.
- **`crates/replayfix-cli`** — the `replayfix` command-line tool.

---

## Prerequisites

ReplayFix builds and runs on Windows (MSVC), macOS and Linux.

**To build:** Rust 1.80 or newer (tested on 1.99.1) and a C/C++ toolchain (Visual Studio Build Tools on Windows, GCC or Clang elsewhere).

**To run `doctor` successfully** you need these tools on your `PATH`. `doctor` tells you exactly what is missing and how to install it for your platform:

| Tool | Minimum | Used for (current or planned) |
| :--- | :--- | :--- |
| Git | 2.30 | worktrees and diffs |
| Node.js | 18 | target apps and Playwright |
| Playwright | 1.40 | browser replay |
| FFmpeg | 4.4 | video keyframes |

The v0.2 mock-heal loop itself needs none of these tools.

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
Playwright   playwright   none             1.40.0           [MISSING]
   >> Remedy: Install Playwright via 'npm install -D playwright' and install browsers via 'npx playwright install chromium'
FFmpeg       ffmpeg       7.1.1            4.4.0            [OK]
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
| `--mock-heal` | Run the simulated two-gate loop (below) | off |
| `--mock-patch <builtin\|noop\|FILE>` | Candidate patch for `--mock-heal` (needs `--mock-heal`) | `builtin` |
| `--output-dir <DIR>` | Where results are written (needs `--mock-heal`) | `.replayfix` |

Without `--mock-heal`, `repro` validates the task and stops.

### Try the two-gate loop: `--mock-heal`

```bash
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --mock-heal
```

```text
Gate 1   : RED   (defect reproduced; contract 400bc991f224)
Repair   : patch applied in memory to src/cart-summary.js (+1 -1)
Gate 2   : GREEN (same contract 400bc991f224)
Result   : VERIFIED_REPAIRED (simulated oracle; no browser was run)
Receipt  : .replayfix/audit_receipt.json
```

What it does:

1. Gate 1 runs a built-in scenario (`RF-01`, an unguarded `cart.items.length`) and must come back **RED**.
2. The candidate patch is applied **in memory** by a strict unified-diff patcher (exact context, no fuzzy matching).
3. Gate 2 evaluates the **same contract** (compared by SHA-256 of its canonical JSON) and must come back **GREEN**.
4. The run ends with exactly one artifact: `audit_receipt.json` on success, or `triage_report.json` on rejection. A rejected run deletes any older receipt in the same folder, so a stale success is never left behind.

Try breaking it on purpose:

```bash
replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video v.mp4 --mock-heal --mock-patch noop
# Result   : repair rejected (REPAIR_NOOP_REJECTED), exit code 30
```

Exit codes: `0` verified, `2` bad input or usage, `10` Gate 1 was not RED or the contract changed, `11` security violation, `30` repair rejected (empty patch, no-op, patch did not apply, or Gate 2 not green).

**Limits to know about**

- Only task `RF-01` has a built-in scenario; other task ids exit with a clear message.
- The "oracle" inspects a bundled source snippet. It proves the state machine, contract check and receipt format, not real browser behavior. Every receipt says so.
- `--mock-patch FILE` accepts a single-file unified diff up to 1 MiB.

---

## Safety checks on inputs

Paths from flags, manifests, patch headers and `--output-dir` go through the same checks and fail with `SECURITY_VIOLATION` (exit 11):

- directory traversal that escapes the working directory (`../..`)
- UNC network paths (`\\server\share`, `//server/share`)
- Windows reserved device names (`CON`, `NUL`, `COM1`…) in any path segment
- NTFS alternate data streams (`file:stream`)

Windows backslashes are normalized, so a manifest written on Windows is checked the same way on macOS and Linux. Environment variable names and healthcheck paths in the manifest are validated too.

---

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Tests run natively on every platform with no custom runner.

**Windows 11 with Smart App Control?** Newly built DLLs and executables can be blocked. The repo ships opt-in helper scripts (including `scripts/sign-build-outputs.ps1`, which signs whatever is still unsigned after a build) and a troubleshooting guide: [docs/windows-code-signing.md](docs/windows-code-signing.md).

---

## Roadmap

| Release | Scope | Status |
| :--- | :--- | :--- |
| v0.0.0 | Workspace genesis | Delivered |
| v0.1.0 / v0.1.1 | `doctor`, manifest ingestion, input hardening | Delivered |
| **v0.2.0** | Dual-gate state machine, in-memory patcher, no-op guard, receipts (simulated oracle) | **Delivered** |
| v0.3.0 | Isolated scratch repositories, ephemeral ports | Planned |
| v0.4.0 | Playwright grounding with real browser gates | Planned |
| v0.5.0 | Video keyframes and SAM 3.1 perception | Planned |
| v0.6.0 | Model-driven bounded repair | Planned |
| v0.7.0 | Test freezing, two-run stability, regression parity | Planned |
| v0.8.0 | Headed replay, traces, evidence matrix | Planned |
| v0.9.0 / v1.0.0 | Merkle-tree signed receipts, five-bug benchmark | Planned |

---

## License

MIT. See [LICENSE.md](LICENSE.md). Release history: [CHANGELOG.md](CHANGELOG.md) and [docs/release-notes](docs/release-notes/).

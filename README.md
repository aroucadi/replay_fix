# ReplayFix

ReplayFix is a deterministic Rust harness for reproducing, verifying and repairing visual bugs in web applications. The long-term design is simple: a bug is only "fixed" when the same behavioral check goes from RED to GREEN, inside an isolated scratch workspace, with a receipt you can audit.

> **Where the project stands (v0.6.0).** ReplayFix is built in small, verified slices. Today it checks your environment, validates bug-reproduction tasks, clones your target repository into an **isolated ephemeral scratch sandbox**, verifies **dual-layer repository immutability ($\Delta \text{repo} = 0$)**, allocates **dynamic ephemeral ports (`127.0.0.1:0`)**, ingests **developer bug videos (`ffprobe` + $120 \times 120\text{px}$ ROI $L_1$ optical-flow differencing + Meta `sam-3.1` contour centroid snapping + Meta `muse-voice-transcribe-1.0` `audioProcessedMs` narration alignment)**, and executes **bounded autonomous Meta Muse repair (`--model muse-spark-1.3` / `muse-wonder-1.0`)** with **anti-cheating SHA-256 test freezing (`TEST_MUTATION_PROHIBITED`), `<UNTRUSTED_CONTEXT>` prompt-injection data fencing, 24h static prefix caching, encrypted deliberation continuity (`reasoning.encrypted_content`), strict 5-field JSON Schema output validation, ambiguity-safe source-block patching, `node --check` syntax preflight, 2-run consecutive Gate 2 stability verification, and bounded 3-attempt failure quarantine (`QUARANTINE_REPAIR_FAILED`, exit code 21)**.

---

## Workspace

- **`crates/replayfix-core`** — the engine: platform helpers (`EphemeralPort`, process tree termination), bounded subprocess supervisor (`run_bounded`), environment diagnostics (`Doctor`), task manifest validation (`TaskManifest`), dual-layer workspace integrity (`WorktreeManifest`) and scratch isolation (`ScratchSandbox`), video prober and ROI differencer (`VideoPreprocessor`, `RoiDiffer`, `GeometricCalibration`), Meta multimodal perception (`SamSegmenter`, `AsrTranscriber`, `VideoTelemetry`), spatial-semantic locator grounding (`LocatorResolver`, `GroundingTelemetry`), live browser bridge (`SandboxServer`, `PlaywrightBridge`), bounded autonomous repair (`MuseModelConfig`, `TestFreezeGuard`, `PromptBuilder`, `StructuredRepairPatch`, `BlockPatcher`, `SyntaxPreflight`, `TokenAccounting`, `BoundedRepairClient`), the phase state machine (`PhaseMachine`), the unified-diff patcher (`UnifiedPatcher`), the verification engine (`MockHealer`), and receipt writers (`AuditReceipt`, `TriageReport`).
- **`crates/replayfix-cli`** — the `replayfix` command-line tool.

---

## Prerequisites

ReplayFix builds and runs on Windows (MSVC), macOS and Linux.

**To build:** Rust 1.80 or newer (tested on 1.99.1) and a C/C++ toolchain (Visual Studio Build Tools on Windows, GCC or Clang elsewhere).

**To run `doctor` successfully** you need these tools on your `PATH`. `doctor` tells you exactly what is missing and how to install it for your platform:

| Tool | Minimum | Used for (current or planned) |
| :--- | :--- | :--- |
| Git | 2.30 | worktree status (`git status --porcelain`) and diffs |
| Node.js | 18 | target apps, Playwright, and `node --check` syntax preflight |
| Playwright | 1.40 | browser replay (`--live-browser`) |
| FFmpeg | 4.4 | video probing, ROI keyframes, and 24 kHz mono PCM audio |

The `--mock-heal` simulated loop runs without Playwright or FFmpeg; `--live-browser` requires Node.js, Playwright with Chromium installed, and FFmpeg when `--video` points to a real recording.

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

### Describe and repair a bug: `replayfix run` / `replayfix repro`

`repro` (aliased as `run`) accepts either flags or a manifest that follows `task_manifest.schema.json`.

```bash
# Live browser + multimodal video ingestion + Meta Muse bounded repair
replayfix run --bug RF-01 --live-browser --headed --model muse-spark-1.3

# Quarantine simulation (exhausts 3 bounded repair attempts -> exit code 21)
replayfix run --bug RF-01 --mock-failing-repair

# Manifest file
replayfix repro --manifest task.json
```

| Flag | Meaning | Default |
| :--- | :--- | :--- |
| `--manifest <FILE>` | JSON task manifest (alternative to the flags below) | — |
| `--bug <ID>` | Shorthand bug identifier (e.g. `RF-01`) populating `--task-id`, `--bug-ref`, and `--video` | — |
| `--task-id`, `--bug-ref`, `--video` | Required unless `--bug` or `--manifest` is given | — |
| `--title`, `--repo`, `--clean-ref`, `--audio` | Optional task metadata | `--repo .` |
| `--server-cmd`, `--port-env-var`, `--healthcheck-path`, `--startup-timeout-ms` | How to launch the target app | `PORT`, `/api/health`, `10000` |
| `--headed`, `--trace` | Visible Chromium window (`--headed` with `--live-browser`) / trace export | off |
| `--live-browser` | Run the two-gate loop in a live Playwright Chromium browser with CDP spatial-semantic grounding | off |
| `--model <SLUG>` | Meta Muse repair model slug (`muse-spark-1.3`, `muse-wonder-1.0`, `muse-spark-1.3-contributor`) | — |
| `--mock-failing-repair` | Simulate 3-attempt bounded repair exhaustion (`QUARANTINE_REPAIR_FAILED`, exit code 21) | off |
| `--mock-syntax-retry` | Simulate `node --check` syntax failure on Attempt 1 and deliberation continuity recovery on Attempt 2 | off |
| `--mock-ambiguous` | Inject a synthetic candidate tie ($|\Delta S| < 0.25$) to test `AMBIGUOUS_GROUNDING` (needs `--live-browser`) | off |
| `--mock-heal` | Run the simulated two-gate loop inside an isolated scratch sandbox | off |
| `--mock-patch <builtin\|noop\|FILE>` | Candidate patch for `--live-browser` or `--mock-heal` | `builtin` |
| `--output-dir <DIR>` | Where results are written (`.replayfix` by default) | `.replayfix` |
| `--dry-run` | Validate environment, manifests, and patch invariants without execution | off |
| `--color <auto\|always\|never>` | Control colored output formatting | `auto` |

### Run live Playwright browser verification with Meta Muse repair

```bash
replayfix run --bug RF-01 --live-browser --headed --model muse-spark-1.3
```

```text
  ✔ Isolation [PASS] Scratch workspace initialized
    › ephemeral port :64679, base tree 22a4c0facc52
  ✔ Video Probe [PASS] Probed recording & extracted ROI keyframes
    › 1280x720 (1000ms), 1 keyframe(s) -> .replayfix/keyframes/
  ✔ ASR Align [PASS] Aligned spoken narration via audioProcessedMs
    › muse-voice-transcribe-1.0 @ 400ms: "I clicked Refresh Cart on RF-01 here"
  ✔ SAM 3.1 Snap [PASS] Snapped hotspot to contour centroid
    › sam-3.1 <|box;x1=100;y1=100;x2=220;y2=140|><|mask;rle:1200:120x40|> -> CSS (160, 120)
  ✔ CDP Grounding [PASS] Grounded target via elementFromPoint + ariaSnapshot
    › page.getByRole('button', { name: 'Refresh Cart', exact: true }) (score: 1.00, margin: 0.95)
  ✔ Gate 1 (RED) [PASS] Defect reproduced causal baseline
    › contract digest: 400bc991f224
  ✔ Test Freeze [PASS] Anti-cheating SHA-256 test freeze verified
    › freeze digest: db4ede630515, verified: true
  ✔ Muse Repair [PASS] Bounded autonomous repair synthesized patch
    › model: muse-spark-1.3 (effort: xhigh, attempt 1/3), cache: replayfix-v0.6-harness-prefix-v1
  ✔ Syntax Check [PASS] Preflight node --check passed in sandbox
    › target: src/cart-summary.js
  ✔ Repair [PASS] Patch applied in scratch sandbox
    › target: src/cart-summary.js (+1 -1), sandbox tree: 84a0ef66fa09
  ✔ Gate 2 (GREEN) [PASS] Behavior verified across 2 consecutive runs
    › contract: 400bc991f224, base repo unmodified: true
```

Exit codes: `0` verified, `2` bad input or usage, `10` Gate 1 was not RED or the contract changed, `11` security violation (`TEST_MUTATION_PROHIBITED`, path traversal, or `BASE_REPO_DIRTY_MUTATION_DETECTED`), `20` browser bridge execution failure (`BRIDGE_EXECUTION_FAILED`), `21` bounded 3-attempt repair exhausted (`QUARANTINE_REPAIR_FAILED`), `22` ambiguous or ungrounded DOM/visual target (`AMBIGUOUS_GROUNDING`), `30` repair rejected (empty patch, no-op, patch did not apply, or Gate 2 not green).

---

## Safety checks on inputs & workspace isolation

Paths from flags, manifests, patch headers, repair `target_file`, sandbox file operations, HTTP requests to `SandboxServer`, and `--output-dir` go through strict boundary validation and fail closed:

- directory traversal that escapes the working directory or scratch sandbox (`../..`)
- symlink escape chains inside the scratch sandbox
- UNC network paths (`\\server\share`, `//server/share`)
- Windows reserved device names (`CON`, `NUL`, `COM1`…) in any path segment
- NTFS alternate data streams (`file:stream`)
- unauthorized modification of the user's base repository during a run (`BASE_REPO_DIRTY_MUTATION_DETECTED`)
- any candidate repair touching protected test or configuration paths (`tests/**`, `*.spec.*`, `*.test.*`, `playwright.config.*`, `package.json` -> `TEST_MUTATION_PROHIBITED`, exit code 11)
- ungrounded or ambiguous DOM/visual locators (`AMBIGUOUS_GROUNDING`, exit code 22) and banned positional `.first()` / `:nth-match` selectors

---

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
powershell -ExecutionPolicy Bypass -File .specify/scripts/check-panics.ps1
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
| v0.4.0 / v0.4.1 | Playwright CDP spatial-semantic grounding, `ariaSnapshot` DOM Grounding Invariant, fail-closed ambiguity guard (`AMBIGUOUS_GROUNDING`), live Chromium browser gates (`--live-browser`), and adversarial systems hardening | Delivered |
| v0.5.0 | Video keyframes, $120 \times 120\text{px}$ ROI optical flow, `sam-3.1` contour centroid snapping, and `muse-voice-transcribe-1.0` `audioProcessedMs` narration alignment | Delivered |
| **v0.6.0** | **Meta Muse bounded repair (`muse-spark-1.3` / `muse-wonder-1.0`), SHA-256 `TestFreezeGuard`, `<UNTRUSTED_CONTEXT>` isolation, 24h static prefix caching, encrypted deliberation continuity (`reasoning.encrypted_content`), strict JSON Schema & ambiguity-safe `BlockPatcher`, `node --check` preflight, 2-run Gate 2 stability, and `QUARANTINE_REPAIR_FAILED` (exit 21)** | **Delivered** |
| v0.7.0 | Differential diagnosis, regression parity suite | Planned |
| v0.8.0 | Headed replay, traces, evidence matrix | Planned |
| v0.9.0 / v1.0.0 | Merkle-tree signed receipts, five-bug benchmark | Planned |

---

## License

MIT. See [LICENSE.md](LICENSE.md). Engineering guidelines: [docs/engineering-principles-rust.md](docs/engineering-principles-rust.md). Release history: [CHANGELOG.md](CHANGELOG.md) and [docs/release-notes](docs/release-notes/).

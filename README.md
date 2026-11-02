# ReplayFix

ReplayFix is a deterministic Rust harness for reproducing, verifying, and autonomously repairing visual and functional defects in web applications with **cryptographic proof**. A defect is only certified as repaired when:
1. The same behavioral contract (`BehavioralContract`, RFC 8785 SHA-256) transitions from **Gate 1 RED** to **Gate 2 GREEN** (across 2 consecutive stability runs),
2. All mutations occur inside an **isolated ephemeral scratch sandbox** with dual-layer repository immutability (`Delta-Repo = 0`),
3. **Anti-cheat guards** (`TestFreezeGuard` + surgical `DiffGuard` on `package.json` `"scripts.test*"`) and **active regression parity** (`F_patched \ F_baseline = empty`) pass, and
4. Disaggregated evidence artifacts are bound into an **RFC 6962 Merkle Tree** (`0x00` leaf / `0x01` internal node domain separation) and signed with **pure RFC 8032 Ed25519** attestation, independently verifiable via `replayfix verify-receipt` or the air-gapped **`proof_viewer.html`** Judge Proof Portal.

> **Current Release: `v1.8.0` (Distributed Multi-Agent Swarm, Live Streaming Dashboard & Production APM Session Replay Ingestion).** Scaling ReplayFix to enterprise-grade distributed triage and real-time observability:
> - **Production APM & RRWeb Ingestion**: `replayfix ingest --sentry <url/file> --rrweb <file>` transforms RRWeb DOM mutation recordings and Sentry/Datadog crash breadcrumbs into deterministic `TaskManifest` reproductions.
> - **Distributed Work-Stealing Swarm**: `replayfix swarm --workers <N>` coordinates multi-agent defect triage across isolated scratch worktrees ($\Delta\text{repo}=0$) with dynamic ephemeral ports and rate-governed model concurrency.
> - **Live Streaming Dashboard**: `replayfix dashboard --port 3000` starts an embedded async HTTP/WebSocket/SSE server streaming live browser screencasts, Muse Spark deliberation feeds, and side-by-side DOM diffs via the embedded single-page `dashboard.html`.
> - **Responsive Inoculation Matrix**: `replayfix inoculate --matrix chromium,firefox,webkit,mobile,desktop` synthesizes multi-browser, multi-device Playwright test suites with device pixel ratio (DPR) coordinate calibration.
> - **RFC 6962 Transparency Ledger**: `replayfix notary` (`append`, `verify-log`, `consistency-proof`) maintains an append-only, tamper-evident cryptographic transparency log (`transparency_log.json`) with Ed25519 attestations.
> All roadmap specifications (`Spec 000` through `Spec 180`) are complete, verified, and qualified across 327 automated tests with 0 non-test panics and 0 Clippy warnings.

---

## Workspace & Deliverables

- **`crates/replayfix-core`** — Core verification engine, GitHub triage engine (`GitHubClient`, `GitHubTransport`, RFC 2104 constant-time HMAC-SHA256 webhook verification, anti-loop bot filter, `IssueCommenter`, `TriageLabel`, issue-specific test synthesis), monotonic phase FSM (`PhaseMachine`, 6-state `DifferentialMatrix`), Gate 3 `StressQuorumRunner` (`PerturbationProfile`, `StressQuorumReport`), Permanent Inoculation engine (`InoculationGenerator`, `InoculationRegistry`), SARIF v2.1.0 & JUnit XML reporters (`SarifReporter`, `JunitReporter`), platform abstractions (`EphemeralPort`, process-tree teardown), `Doctor` diagnostics, `TaskManifest`, `WorktreeManifest` & `ScratchSandbox`, `VideoPreprocessor` & 120x120px `RoiDiffer`, Meta multimodal perception (`SamSegmenter` `sam-3-1`, `AsrTranscriber` `muse-voice-transcribe-1.0`), CDP spatial-semantic grounding (`LocatorResolver` with interactive tie-breaker), live Playwright bridge (`SandboxServer`, `PlaywrightBridge`), bounded autonomous repair (`MuseModelConfig`, `ModelPool`, `TestFreezeGuard`, `DiffGuard`, `BlockPatcher`, `SyntaxPreflight`, `RegressionEvaluator`), bounded cloud gateway (`MultimodalGateway`, `RatePool`, `CircuitBreaker`), RFC 6962 `MerkleTree`, pure Ed25519 `KeypairManager`, 5-bug `sweep` runner, and `portal` verifier.
- **`crates/replayfix-cli`** — The `replayfix` command-line tool (`bot`, `heal`, `chat`/`agent`, `mcp`, `inoculate`, `doctor`, `run`/`repro`, `freeze-spec`, `verify-receipt`, `sweep`, `export-proof`, `session`).
- **`proof_viewer.html`** — Standalone, air-gapped single-file HTML5/ES2022 Judge Proof Portal with client-side RFC 8785 / RFC 6962 / RFC 8032 Ed25519 cryptographic verification (`default-src 'none'; connect-src 'none'`).

---

## Prerequisites

ReplayFix builds and runs natively on **Windows 11/10 (MSVC)**, **macOS**, and **Linux**.

| Tool | Minimum Version | Purpose |
| :--- | :--- | :--- |
| **Rust** | `1.80+` (tested `1.99.1`) | Building from source (`cargo build --release`) |
| **Git** | `2.30+` | Worktree parity (`git status --porcelain`) and unified diff validation |
| **Node.js** | `18.0+` | Target web apps, `node --check` syntax preflight, and Playwright execution |
| **Playwright** | `1.40+` | Live headless/headed Chromium replay (`--live-browser`) and trace viewing (`--trace`) |
| **FFmpeg** | `4.4+` | Video probing (`ffprobe`), ROI keyframe extraction, and 24 kHz mono PCM audio |

*Note*: Offline deterministic verification (`--mock-heal`, `sweep --mock-heal`, `verify-receipt`, `export-proof`, and `proof_viewer.html`) runs with zero external network or browser dependencies.

---

## Quickstart & CLI Reference

### 1. Build & Sign Production Binary (Windows 11)

```powershell
cargo build --release --workspace
# Sign for Windows 11 Smart App Control / WDAC compatibility (local CurrentUser trust store)
powershell -ExecutionPolicy Bypass -File ./scripts/setup-dev-cert.ps1
powershell -ExecutionPolicy Bypass -File ./scripts/sign-binary.ps1 target/release/replayfix.exe
```

### 2. Configure `META_API_KEY` for `--mode live-ai` (Global or `.env`)

Note: Running `$env:META_API_KEY = "your-key"` in PowerShell only sets the variable for the current terminal tab. To make `META_API_KEY` globally accessible across all terminals and child processes, use **either** of the following:

```powershell
# Option A: Persist globally in your Windows User environment (survives terminal restarts)
[Environment]::SetEnvironmentVariable("META_API_KEY", "your-meta-api-key", "User")

# Option B: Save in the gitignored `.env` file in the workspace root
Set-Content -Path .env -Value 'META_API_KEY="your-meta-api-key"'
```

(`ReplayFix` automatically checks process environment variables, `.env` in the working/binary directory tree, and the Windows `HKCU\Environment` / `HKLM` registry hives.)

### 3. Sovereign Autonomous Healer Macro Pipeline: `replayfix heal` (Release 1.6)

Execute the end-to-end visual bug reproduction, multi-turn **Meta Muse Spark 1.3** agent loop (`305 tok/s`), **Meta Muse Wonder 1.0** AST block repair, Gate 2 stability verification, Gate 3 stress quorum, permanent test inoculation, and signed Merkle audit receipt export in **under 60 seconds**:

```bash
# Offline deterministic mode against real MetaPulse MP4 recording
replayfix heal --video benchmarks/metapulse/videos/RF-01_quote_repost_crash_with_cursor.mp4 --repo . --mode deterministic-fixture

# Live AI mode (Meta SAM 3.1 + Muse Voice + Muse Spark 1.3 + Muse Wonder 1.0 with Encrypted Deliberation Caching)
replayfix heal --video benchmarks/metapulse/videos/RF-01_quote_repost_crash_with_cursor.mp4 --repo . --mode live-ai

# Sovereign repair with headless browser, Gate 3 stress quorum, and permanent inoculation
replayfix heal --video benchmarks/metapulse/videos/RF-01_quote_repost_crash_with_cursor.mp4 --repo . --mode deterministic-fixture --inoculate --stress-quorum

# Auto-package verified fix into an isolated Git branch with Ed25519 commit trailers and pr_description.md
replayfix heal --video benchmarks/metapulse/videos/RF-01_quote_repost_crash_with_cursor.mp4 --repo . --package-pr
```

### 4. Autonomous GitHub Issue Triage Bot: `replayfix bot` (Release 1.7)

Autonomously triage incoming GitHub bug issues with attached screen recordings (`.mp4`, `.webm`, `.mov`), reproduce failures in ephemeral scratch sandboxes ($\Delta\text{repo}=0$), open verified Pull Requests with permanent Playwright regression specs (`tests/repro/rf_issue_<id>.spec.ts`), and post cryptographic proof comments back to the issue thread:

```bash
# Ingest and autonomously triage a single GitHub issue with an attached video:
replayfix bot --issue https://github.com/my-org/my-repo/issues/42 --repo .

# Ingest with automated PR creation, issue commenting, and triage labeling:
replayfix bot --issue https://github.com/my-org/my-repo/issues/42 --repo . --create-pr --comment-issue --apply-labels

# Offline deterministic triage in CI (zero external network tokens billed):
replayfix bot --issue https://github.com/my-org/my-repo/issues/42 --repo . --mock-heal

# Continuous webhook listener with RFC 2104 constant-time HMAC-SHA256 security:
replayfix bot --listen 0.0.0.0:8080 --webhook-secret "$GITHUB_WEBHOOK_SECRET" --repo . --create-pr --comment-issue
```

### 5. Interactive Conversational Agent Workspace (Release 1.4)

Launch the natural-language agentic workspace directly in your terminal:

```bash
# Launch interactive agent REPL (or simply run `replayfix` in any interactive terminal)
replayfix chat

# Example conversational requests inside the shell:
# > "Reproduce and fix the cart quantity bug in metapulse"
# > "Run doctor to check my environment"
# > "Benchmark all 5 defect scenarios"
# > "Verify receipt at .replayfix/audit_receipt.json"
# > "/help" (see full command guide)
```

### 6. Environment Diagnostics: `replayfix doctor`

```bash
replayfix doctor
replayfix doctor --json
```

### 7. Single-Bug Reproduction, Multimodal Video Ingestion & Autonomous Repair: `replayfix run`

```bash
# Offline deterministic fixture mode against real MetaPulse MP4 recording (zero API tokens billed)
replayfix run --video benchmarks/metapulse/videos/RF-01_quote_repost_crash_with_cursor.mp4 --repo . --mode deterministic-fixture

# Live AI mode (Meta SAM 3.1 + Muse Voice 24kHz ASR + Muse Spark 1.3 / Wonder 1.0 over HTTPS)
replayfix run --video benchmarks/metapulse/videos/RF-01_quote_repost_crash_with_cursor.mp4 --repo . --mode live-ai --model-intent muse-spark-1.3 --model-repair muse-wonder-1.0

# Live Chromium browser + multimodal video ingestion + Meta Muse repair
replayfix run --bug RF-01 --live-browser --headed --model muse-spark-1.3

# Live GitHub issue ingestion + interactive human approval gate (--apply)
replayfix run --bug RF-01 --issue https://github.com/org/repo/issues/42 --mock-heal --apply --yes
```

### 8. Five-Bug MetaPulse Benchmark Sweep: `replayfix sweep`

Executes isolated scratch-sandbox repairs across `RF-01` through `RF-05`, enforces `Delta-Repo = 0`, and emits a signed `.replayfix/sweep_receipt.json` in < 60s (~4.5s offline):

```bash
replayfix sweep --mock-heal
replayfix sweep --bugs RF-01,RF-02,RF-03,RF-04,RF-05 --mock-heal --json
```

### 9. Standalone 4-Layer Cryptographic Verification: `replayfix verify-receipt`

Verifies Layer 1 (on-disk artifact SHA-256 & byte size), Layer 2 (RFC 6962 Merkle inclusion proofs), Layer 3 (RFC 8032 pure Ed25519 signature), and Layer 4 (signer public-key pinning):

```bash
replayfix verify-receipt .replayfix/audit_receipt.json
replayfix verify-receipt .replayfix/audit_receipt.json --trusted-key <64_CHAR_HEX_PUBKEY> --json
replayfix verify-receipt .replayfix/sweep_receipt.json --json
```

### 10. Freeze Reproduction Spec & Export Proof Bundle

```bash
replayfix freeze-spec --bug RF-01 --repo .
replayfix export-proof --output .replayfix/proof --json
```

Open **`proof_viewer.html`** directly in any browser (`file://`) to inspect receipts, step through RFC 6962 Merkle audit paths, verify Ed25519 signatures, step through the **Muse Spark Deliberation Timeline**, inspect side-by-side accessibility DOM diffs, and inspect SAM 3.1 spatial overlays and unified diffs offline.

---

## Deterministic Exit Code Taxonomy

| Exit Code | Constant / Verdict | Meaning |
| :--- | :--- | :--- |
| **`0`** | `SUCCESS` / `VERIFIED_REPAIRED` | Defect reproduced (Gate 1 RED), repaired, verified (2x Gate 2 GREEN + regression parity), or receipt verified |
| **`1`** | `DOCTOR_OR_INTERNAL_ERROR` | Missing environment dependency (`doctor`) or internal system error |
| **`2`** | `USAGE_OR_INPUT_ERROR` | Invalid CLI flags, disallowed model slug, or malformed manifest |
| **`10`** | `GATE1_NOT_RED` | Defect did not reproduce on Gate 1 or behavioral contract digest changed |
| **`11`** | `SECURITY_VIOLATION` | `TEST_MUTATION_PROHIBITED`, `DiffGuard` violation, path traversal, or `BASE_REPO_DIRTY_MUTATION_DETECTED` |
| **`12`** | `APPROVAL_DENIED` | User declined interactive `--apply` patch approval |
| **`13`** | `NON_INTERACTIVE_TTY_REQUIRED` | `--apply` invoked without `--yes` in a non-interactive terminal |
| **`20`** | `BRIDGE_EXECUTION_FAILED` | Playwright bridge or child server failed |
| **`21`** | `QUARANTINE_REPAIR_FAILED` | Bounded 3-attempt repair budget exhausted or flaky non-deterministic run |
| **`22`** | `AMBIGUOUS_GROUNDING` | Spatial-semantic locator grounding margin `< 0.25` |
| **`23`** | `ISSUE_NOT_FOUND_OR_RATE_LIMITED` | GitHub issue HTTP 404/403/429 |
| **`24`** | `ISSUE_MALFORMED_OR_UNPARSEABLE` | GitHub issue payload missing required reproduction fields |
| **`30`** | `REPAIR_REJECTED` / `ERR_GATEWAY_AUTH_MISSING` | Empty/no-op patch (`REPAIR_NOOP_REJECTED`), patch failed to apply, Gate 2 remained RED, or `--mode live-ai` invoked without `META_API_KEY` |
| **`31`** | `REGRESSION_DETECTED` | Candidate patch broke pre-existing regression tests (`F_patched \ F_baseline != empty`) |
| **`32`** | `QUARANTINE_INVERTED_BASELINE` | Clean reference failed or baseline was already GREEN (`AlreadyHealthy`) |
| **`40`** | `RECEIPT_VERIFICATION_FAILED` | On-disk artifact tampering, invalid RFC 6962 Merkle proof, invalid Ed25519 signature, or untrusted signer key |
| **`41`** | `SWEEP_PARTIAL_FAILURE` | One or more benchmark bugs in `replayfix sweep` failed verification |

---

## Completed Release Roadmap (`v0.0.0` -> `v1.7.0`)

| Release | Scope | Status |
| :--- | :--- | :--- |
| **v0.0.0** | Clean-room 2-crate Rust workspace genesis & platform abstractions | **Delivered** |
| **v0.1.0–0.1.1** | `doctor` diagnostics, manifest ingestion, UNC/DOS/ADS/traversal hardening | **Delivered** |
| **v0.2.0–0.2.1** | Dual-gate FSM (`Gate1Red` -> `Gate2Green`), RFC 8785 contract digest, no-op guard | **Delivered** |
| **v0.3.0–0.3.3** | Ephemeral `ScratchSandbox`, `Delta-Repo = 0` `WorktreeManifest`, `EphemeralPort`, process tree teardown | **Delivered** |
| **v0.4.0–0.4.1** | Playwright CDP grounding, `ariaSnapshot` DOM Grounding Invariant, fail-closed `AMBIGUOUS_GROUNDING` | **Delivered** |
| **v0.5.0** | Video keyframe probing, 120x120px ROI optical flow, `sam-3.1` snapping, `muse-voice-transcribe-1.0` ASR | **Delivered** |
| **v0.6.0–0.6.2** | Meta Muse bounded repair (`muse-spark-1.3`/`muse-wonder-1.0`), `TestFreezeGuard`, 2-run Gate 2 stability | **Delivered** |
| **v0.7.0–0.7.1** | 6-state `DifferentialMatrix`, surgical `DiffGuard`, active `RegressionEvaluator`, `CheckpointManager`, `freeze-spec` | **Delivered** |
| **v0.8.0–0.8.2** | `--trace` viewer, live GitHub `--issue` ingestion, ANSI-safe `--apply` approval gate, atomic `ArtifactStore` | **Delivered** |
| **v0.9.0–0.9.1** | RFC 6962 Merkle receipts, pure RFC 8032 Ed25519 attestation, 4-layer `verify-receipt`, 5-bug `sweep`, `ModelPool` | **Delivered** |
| **v0.9.5** | Offline `proof_viewer.html` Judge Proof Portal, bounded `MultimodalGateway`, rate pool & circuit breaker | **Delivered** |
| **v1.0.0** | Production LTO/abort optimization, Windows Authenticode signing, 3-Act demo video & submission package | **Delivered** |
| **v1.1.0** | Real MetaPulse 20-bug benchmark suite (`RF-01`..`RF-20`), `MetaGatewayClient` (`rustls-tls`), dual execution modes | **Delivered** |
| **v1.2.0** | Live Meta SAM 3.1 wire segmentation, Muse Voice 24kHz PCM ASR, multi-step `ActionTrajectory`, `.env`/Registry auth | **Delivered** |
| **v1.3.0** | Real MetaPulse defect benchmarks, cross-platform video pipeline, and live CDP capture | **Delivered** |
| **v1.4.0** | Interactive Conversational Agent Workspace (`replayfix chat`), natural-language intent router, interactive REPL | **Delivered** |
| **v1.5.0** | Permanent Inoculation ("Dreaming" into Playwright specs), Native MCP Server (`stdio`/`sse`), Gate 3 Stress Quorum, Git PR Packager | **Delivered** |
| **v1.5.1** | Production Meta multimodal wire protocol across all 5 Meta Muse & SAM models | **Delivered** |
| **v1.6.0** | **Sovereign Autonomous Healer (`replayfix heal`), Multi-Turn Meta Muse Spark 1.3 Agent Loop (`305 tok/s`), Wonder AST Encrypted Deliberation Caching, and Sub-60s Macro Pipeline** | **Delivered** |
| **v1.7.0** | **Autonomous GitHub Issue Ingestion Bot, Automated PR Dispatch & Continuous Triage Engine (`replayfix bot`)** | **Delivered** |

---

## License

MIT. See [LICENSE.md](LICENSE.md). Security & trust model: [docs/trust-architecture.md](docs/trust-architecture.md) and [docs/windows-code-signing.md](docs/windows-code-signing.md). Release history: [CHANGELOG.md](CHANGELOG.md) and [docs/release-notes/](docs/release-notes/).

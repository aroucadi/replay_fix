# ReplayFix

ReplayFix is a deterministic Rust harness for reproducing, verifying, and autonomously repairing visual and functional defects in web applications with **cryptographic proof**. A defect is only certified as repaired when:
1. The same behavioral contract (`BehavioralContract`, RFC 8785 SHA-256) transitions from **Gate 1 RED** to **Gate 2 GREEN** (across 2 consecutive stability runs),
2. All mutations occur inside an **isolated ephemeral scratch sandbox** with dual-layer repository immutability ($\Delta	ext{repo} = 0$),
3. **Anti-cheat guards** (`TestFreezeGuard` + surgical `DiffGuard` on `package.json` `"scripts.test*"`) and **active regression parity** ($F_{	ext{patched}} \setminus F_{	ext{baseline}} = \emptyset$) pass, and
4. Disaggregated evidence artifacts are bound into an **RFC 6962 Merkle Tree** (`0x00` leaf / `0x01` internal node domain separation) and signed with **pure RFC 8032 Ed25519** attestation, independently verifiable via `replayfix verify-receipt` or the air-gapped **`proof_viewer.html`** Judge Proof Portal.

> **Current Release: `v1.0.0` (Production MMP).** All ten roadmap specifications (`Spec 000` through `Spec 100`) are complete, verified, and qualified across 223 automated tests with 0 non-test panics and 0 Clippy warnings.

---

## Workspace & Deliverables

- **`crates/replayfix-core`** — Core verification engine, monotonic phase FSM (`PhaseMachine`, 6-state `DifferentialMatrix`), platform abstractions (`EphemeralPort`, process-tree teardown), `Doctor` diagnostics, `TaskManifest`, `WorktreeManifest` & `ScratchSandbox`, `VideoPreprocessor` & $120	imes120	ext{px}$ `RoiDiffer`, Meta multimodal perception (`SamSegmenter` `sam-3-1`, `AsrTranscriber` `muse-voice-transcribe-1.0`), CDP spatial-semantic grounding (`LocatorResolver`), live Playwright bridge (`SandboxServer`, `PlaywrightBridge`), bounded autonomous repair (`MuseModelConfig`, `ModelPool`, `TestFreezeGuard`, `DiffGuard`, `BlockPatcher`, `SyntaxPreflight`, `RegressionEvaluator`), bounded cloud gateway (`MultimodalGateway`, `RatePool`, `CircuitBreaker`), RFC 6962 `MerkleTree`, pure Ed25519 `KeypairManager`, 5-bug `sweep` runner, and `portal` verifier.
- **`crates/replayfix-cli`** — The `replayfix` command-line tool (`doctor`, `run`/`repro`, `freeze-spec`, `verify-receipt`, `sweep`, `export-proof`).
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
powershell -ExecutionPolicy Bypass -File .\scripts\setup-dev-cert.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\sign-binary.ps1 targeteleaseeplayfix.exe
```

### 2. Environment Diagnostics: `replayfix doctor`

```bash
replayfix doctor
replayfix doctor --json
```

### 3. Single-Bug Reproduction, Autonomous Repair & Human Approval: `replayfix run`

```bash
# Simulated two-gate repair in isolated scratch sandbox (emits Merkle + Ed25519 receipt)
replayfix --mock-heal

# Live Chromium browser + multimodal video ingestion + Meta Muse repair
replayfix run --bug RF-01 --live-browser --headed --model muse-spark-1.3

# Live GitHub issue ingestion + interactive human approval gate (--apply)
replayfix run --bug RF-01 --issue https://github.com/org/repo/issues/42 --mock-heal --apply --yes
```

### 4. Five-Bug MetaPulse Benchmark Sweep: `replayfix sweep`

Executes isolated scratch-sandbox repairs across `RF-01` through `RF-05`, enforces $\Delta	ext{repo}=0$, and emits a signed `.replayfix/sweep_receipt.json` in $<60	ext{s}$ (`~4.5	ext{s}` offline):

```bash
replayfix sweep --mock-heal
replayfix sweep --bugs RF-01,RF-02,RF-03,RF-04,RF-05 --mock-heal --json
```

### 5. Standalone 4-Layer Cryptographic Verification: `replayfix verify-receipt`

Verifies Layer 1 (on-disk artifact SHA-256 & byte size), Layer 2 (RFC 6962 Merkle inclusion proofs), Layer 3 (RFC 8032 pure Ed25519 signature), and Layer 4 (signer public-key pinning):

```bash
replayfix verify-receipt .replayfix/audit_receipt.json
replayfix verify-receipt .replayfix/audit_receipt.json --trusted-key <64_CHAR_HEX_PUBKEY> --json
replayfix verify-receipt .replayfix/sweep_receipt.json --json
```

### 6. Freeze Reproduction Spec & Export Proof Bundle

```bash
replayfix freeze-spec --bug RF-01 --repo .
replayfix export-proof --output .replayfix/proof --json
```

Open **`proof_viewer.html`** directly in any browser (`file://`) to inspect receipts, step through RFC 6962 Merkle audit paths, verify Ed25519 signatures, and inspect SAM 3.1 spatial overlays and unified diffs offline.

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
| **`22`** | `AMBIGUOUS_GROUNDING` | Spatial-semantic locator grounding margin $|\Delta S| < 0.25$ |
| **`23`** | `ISSUE_NOT_FOUND_OR_RATE_LIMITED` | GitHub issue HTTP 404/403/429 |
| **`24`** | `ISSUE_MALFORMED_OR_UNPARSEABLE` | GitHub issue payload missing required reproduction fields |
| **`30`** | `REPAIR_REJECTED` | Empty/no-op patch (`REPAIR_NOOP_REJECTED`), patch failed to apply, or Gate 2 remained RED |
| **`31`** | `REGRESSION_DETECTED` | Candidate patch broke pre-existing regression tests (`F_patched \ F_baseline != empty`) |
| **`32`** | `QUARANTINE_INVERTED_BASELINE` | Clean reference failed or baseline was already GREEN (`AlreadyHealthy`) |
| **`40`** | `RECEIPT_VERIFICATION_FAILED` | On-disk artifact tampering, invalid RFC 6962 Merkle proof, invalid Ed25519 signature, or untrusted signer key |
| **`41`** | `SWEEP_PARTIAL_FAILURE` | One or more benchmark bugs in `replayfix sweep` failed verification |

---

## Completed Release Roadmap (`v0.0.0` $ightarrow$ `v1.0.0`)

| Release | Scope | Status |
| :--- | :--- | :--- |
| **v0.0.0** | Clean-room 2-crate Rust workspace genesis & platform abstractions | **Delivered** |
| **v0.1.0–0.1.1** | `doctor` diagnostics, manifest ingestion, UNC/DOS/ADS/traversal hardening | **Delivered** |
| **v0.2.0–0.2.1** | Dual-gate FSM (`Gate1Red` $	o$ `Gate2Green`), RFC 8785 contract digest, no-op guard | **Delivered** |
| **v0.3.0–0.3.3** | Ephemeral `ScratchSandbox`, $\Delta	ext{repo}=0$ `WorktreeManifest`, `EphemeralPort`, process tree teardown | **Delivered** |
| **v0.4.0–0.4.1** | Playwright CDP grounding, `ariaSnapshot` DOM Grounding Invariant, fail-closed `AMBIGUOUS_GROUNDING` | **Delivered** |
| **v0.5.0** | Video keyframe probing, $120	imes120	ext{px}$ ROI optical flow, `sam-3.1` snapping, `muse-voice-transcribe-1.0` ASR | **Delivered** |
| **v0.6.0–0.6.2** | Meta Muse bounded repair (`muse-spark-1.3`/`muse-wonder-1.0`), `TestFreezeGuard`, 2-run Gate 2 stability | **Delivered** |
| **v0.7.0–0.7.1** | 6-state `DifferentialMatrix`, surgical `DiffGuard`, active `RegressionEvaluator`, `CheckpointManager`, `freeze-spec` | **Delivered** |
| **v0.8.0–0.8.2** | `--trace` viewer, live GitHub `--issue` ingestion, ANSI-safe `--apply` approval gate, atomic `ArtifactStore` | **Delivered** |
| **v0.9.0–0.9.1** | RFC 6962 Merkle receipts, pure RFC 8032 Ed25519 attestation, 4-layer `verify-receipt`, 5-bug `sweep`, `ModelPool` | **Delivered** |
| **v0.9.5** | Offline `proof_viewer.html` Judge Proof Portal, bounded `MultimodalGateway`, rate pool & circuit breaker | **Delivered** |
| **v1.0.0** | **Production LTO/abort optimization, Windows Authenticode signing, 3-Act demo video & submission package** | **Delivered** |

---

## License

MIT. See [LICENSE.md](LICENSE.md). Security & trust model: [docs/trust-architecture.md](docs/trust-architecture.md) and [docs/windows-code-signing.md](docs/windows-code-signing.md). Release history: [CHANGELOG.md](CHANGELOG.md) and [docs/release-notes/](docs/release-notes/).

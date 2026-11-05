# ReplayFix v3.0.0 CLI & Operator Reference

This document provides an exhaustive command-line reference for the `replayfix` binary (`v3.0.0`).

---

## 1. Environment Variables & Credentials

ReplayFix automatically loads `.env` from the working directory at startup.

| Variable | Required For | Description | Default |
| :--- | :--- | :--- | :--- |
| `META_API_KEY` | Live AI (`repro`, `heal`) | Bearer token for `https://api.meta.ai/v1/responses` (`muse-spark-1.3`, `muse-wonder-1.0`, `sam-3.1`). | — |
| `META_BASE_URL` | Optional override | Override Meta AI Responses API endpoint URL. | `https://api.meta.ai/v1/responses` |
| `BASETEN_API_KEY` | Optional fallback | Bearer token for open-weights fallback endpoint on HTTP 429/5xx. | — |
| `REPLAYFIX_OUTPUT_DIR` | Optional override | Default output directory for receipts and artifacts. | `.replayfix` |

---

## 2. Primary Subcommands

### `replayfix repro` (Alias: `replayfix run`)
Runs the complete Two-Gate visual reproduction, bounded autonomous repair, and cryptographic receipt pipeline.

```bash
# Live zero-shot reproduction & repair in a visible Chromium browser:
replayfix repro 
  --video benchmarks/react-resizable/DEMO_react-resizable_issue-136.mp4 
  --repo benchmarks/react-resizable/repo 
  --live-browser 
  --headed
```

#### Key Options for `repro`:
| Flag | Description | Default |
| :--- | :--- | :--- |
| `--video <PATH>` | Path to the bug screen recording (`.mp4` or `.webm`). | Required (or via `--bug`) |
| `--repo <DIR>` | Path to the target Git repository (strictly read-only unless `--apply` is passed). | `.` |
| `--live-browser` | Execute Gate 1 and Gate 2 in a real Chromium browser via Playwright CDP. | Auto-enabled when `--video` is provided without `--mock-heal` |
| `--headed [<BOOL>]` | Launch Chromium in visible headed mode so operators/judges can watch the replay live. | `false` |
| `--mode <MODE>` | Execution mode: `live-ai` (real HTTPS calls via `META_API_KEY`) or `deterministic-fixture`. | Auto-detected (`live-ai` when `META_API_KEY` is set) |
| `--model <SLUG>` | Meta Muse model slug (`muse-spark-1.3`, `muse-wonder-1.0`, `muse-spark-1.3-contributor`). | `muse-spark-1.3` |
| `--reasoning-effort <EFFORT>` | Reasoning effort level (`high` or `xhigh`; `max` is prohibited by Law VII). | Model default (`xhigh` for Spark 1.3) |
| `--stress-quorum` | Run Gate 3 concurrency perturbation (3G network / 500ms latency, 3-run quorum). | `false` |
| `--inoculate` | Synthesize a permanent Playwright regression spec (`tests/repro/rf_<id>.spec.ts`). | `false` |
| `--apply` | Apply the verified patch to the base repository after passing the human confirmation gate. | `false` |
| `-y, --yes` | Confirm `--apply` non-interactively. | `false` |
| `--output-dir <DIR>` | Directory where `audit_receipt.json`, `merkle_receipt.json`, and `artifacts/` are written. | `.replayfix` |

---

### `replayfix heal`
Executes the multi-turn `SparkDeliberationEngine` (`muse-spark-1.3` perception/grounding followed by `muse-wonder-1.0` surgical AST patching) and prints the turn-by-turn deliberation timeline and `Execution Authenticity & Forensic Matrix`.

```bash
replayfix heal 
  --video benchmarks/react-resizable/DEMO_react-resizable_issue-136.mp4 
  --repo benchmarks/react-resizable/repo 
  --mode live-ai 
  --headed
```

---

### `replayfix verify-receipt`
Performs 4-Layer Zero-Trust verification on an `audit_receipt.json`, `merkle_receipt.json`, or `sweep_receipt.json`:
1. **Layer 1 (Disk Integrity)**: Verifies every referenced artifact in `artifacts/` exists, matches `size_bytes`, and hashes to `content_sha256`.
2. **Layer 2 (RFC 6962 Inclusion Proofs)**: Recomputes and verifies Merkle inclusion paths against `merkle_root`.
3. **Layer 3 (RFC 8032 Ed25519 Signature)**: Verifies the Ed25519 signature over `REPLAYFIX-ATTESTATION:v0.9:...`.
4. **Layer 4 (Trust Policy)**: Evaluates the signer public key against `--trusted-key` or `.replayfix/trusted_keys.json`.

```bash
replayfix verify-receipt .replayfix/audit_receipt.json
```

---

### `replayfix notary`
Manages the append-only RFC 6962 cryptographic transparency log (`transparency_log.json`):

```bash
# Verify log integrity and consistency proofs:
replayfix notary verify .replayfix/transparency_log.json

# Export a standalone proof bundle:
replayfix notary export-proof .replayfix/audit_receipt.json --output proof_bundle.json
```

---

### `replayfix doctor`
Audits local host readiness (`git`, `node`, `playwright`, `ffmpeg`, `ffprobe`, and loopback socket binding):

```bash
replayfix doctor
```

---

### `replayfix swarm` & `replayfix dashboard`
Run multi-worker parallel triage and stream real-time telemetry over WebSocket/SSE:

```bash
# Run 4 isolated worker sandboxes in parallel:
replayfix swarm --workers 4 --bugs RF-01,RF-02,RF-03,RF-04 --output-dir .replayfix

# Launch the Cybernetic Command Deck on port 3848:
replayfix dashboard --port 3848 --output-dir .replayfix
```

---

## 3. Complete Exit Code Reference

| Exit Code | Constant | Triage Status / Condition |
| :---: | :--- | :--- |
| **`0`** | `SUCCESS` | `VERIFIED_REPAIRED`: Gate 1 RED reproduced, patch applied, Gate 2 GREEN $	imes 2$ verified, receipt signed. |
| **`1`** | `INTERNAL_OR_SYSTEM_ERROR` | Environment check failure (`doctor`) or unexpected I/O error. |
| **`2`** | `CLI_USAGE_ERROR` | Invalid CLI arguments, missing required flags, or disallowed model/effort configuration. |
| **`10`** | `GATE1_NOT_RED` | `GATE1_NOT_RED` / `CONTRACT_MISMATCH`: Defect did not reproduce on Gate 1 baseline. |
| **`11`** | `SECURITY_VIOLATION` | `SECURITY_VIOLATION` / `BASE_REPO_DIRTY_MUTATION_DETECTED`: Test freeze violation, path traversal, or $\Delta	ext{repo} 
eq 0$. |
| **`12`** | `NON_INTERACTIVE_REFUSAL` | `--apply` requested in a non-interactive terminal without `--yes`. |
| **`13`** | `USER_REJECTED_PATCH` | Operator declined patch application at the interactive confirmation prompt. |
| **`20`** | `BRIDGE_EXECUTION_FAILED` | Playwright CDP bridge process timeout or browser crash. |
| **`21`** | `QUARANTINE_REPAIR_FAILED` | Exhausted all 3 bounded repair attempts or failed Gate 3 stress stability quorum. |
| **`22`** | `AMBIGUOUS_GROUNDING` | Top DOM candidates at `(click_x, click_y)` tied within `< 0.25` score margin. |
| **`25`** | `ENVIRONMENT_SETUP_FAILED` | Ephemeral application server failed healthcheck readiness within timeout. |
| **`30`** | `REPAIR_VERIFICATION_FAILED` | `REPAIR_NOOP_REJECTED`, `PATCH_APPLY_FAILED`, or `GATE2_NOT_GREEN`. |
| **`31`** | `REGRESSION_DETECTED` | Candidate patch introduced new failures into the pre-existing test suite. |
| **`32`** | `QUARANTINE_INVERTED_BASELINE` | Clean reference branch failed while buggy branch passed. |
| **`40`** | `CRYPTO_VERIFICATION_FAILED` | Merkle inclusion proof mismatch, artifact tampering, or invalid Ed25519 signature. |

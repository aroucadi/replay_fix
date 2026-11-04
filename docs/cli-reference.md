# ReplayFix CLI & Operator Reference

This document provides a comprehensive command-line reference for the `replayfix` binary.

---

## Global CLI Flags & Environment

| Flag | Environment Variable | Description | Default |
| :--- | :--- | :--- | :--- |
| `--verbose` | `REPLAYFIX_LOG=debug` | Enable verbose debug logging to stderr. | `false` |
| `--json` | `REPLAYFIX_JSON=1` | Output machine-readable structured JSON. | `false` |
| `--output-dir <DIR>` | `REPLAYFIX_OUTPUT_DIR` | Target directory for receipts, logs, and artifacts. | `.replayfix/` |
| `--meta-api-key <KEY>` | `META_API_KEY` | Bearer token for Meta AI model endpoints. | None |

---

## Core Subcommands

### 1. `replayfix doctor`
Performs an offline environment health audit, verifying all mandatory platform dependencies:
```bash
replayfix doctor
```
Checks:
- `git` version (`>= 2.30`)
- `node` & `npm` version (`>= 18.0`)
- `playwright` CLI availability & browser binaries
- `ffmpeg` & `ffprobe` binaries (`>= 4.4`)
- Loopback networking permissions (`127.0.0.1`)

---

### 2. `replayfix repro`
Autonomous end-to-end visual reproduction and repair pipeline from a video artifact:
```bash
replayfix repro \
  --video <PATH_TO_VIDEO.mp4> \
  --repo <PATH_TO_CODEBASE> \
  [--headed <true|false>] \
  [--max-attempts <N>] \
  [--output-dir <DIR>]
```
**Options**:
- `--video <FILE>`: Path to input screen recording (`.mp4` or `.webm`).
- `--repo <DIR>`: Path to target web codebase.
- `--headed`: Run browser in headed mode for visual observation (default: `false`).
- `--max-attempts`: Bounded repair iterations (default: `3`).

---

### 3. `replayfix ingest`
Ingests production telemetry (RRWeb session recordings, Sentry crash breadcrumbs, or OpenTelemetry spans) and converts them into normalized `TaskManifest` reproductions:
```bash
replayfix ingest \
  --rrweb <PATH_TO_RRWEB.json> \
  --sentry <PATH_TO_SENTRY.json> \
  --output <PATH_TO_MANIFEST.json>
```

---

### 4. `replayfix swarm`
Executes high-concurrency distributed bug triage across multi-core workers with work-stealing job orchestration:
```bash
replayfix swarm \
  --workers <N> \
  --bugs <ID1,ID2,ID3...> \
  --output <DIR> \
  [--mock-heal]
```
**Options**:
- `--workers <N>`: Number of concurrent worker threads.
- `--bugs <LIST>`: Comma-separated list of bug identifiers or manifest paths.
- `--mock-heal`: Offline deterministic qualification mode with pre-baked AST fixtures.

---

### 5. `replayfix dashboard`
Boots an embedded async Tokio/Axum HTTP & WebSocket telemetry server streaming live browser screencasts and Muse Spark reasoning tokens:
```bash
replayfix dashboard \
  --port <PORT> \
  --artifacts <DIR> \
  [--headless-check]
```
- Open `http://127.0.0.1:<PORT>` to access the responsive web dashboard (`dashboard.html`).
- `--headless-check`: Verifies server startup, socket binding, and cleanly terminates with code `0`.

---

### 6. `replayfix inoculate`
Synthesizes permanent, multi-engine Playwright regression test matrices calibrated across desktop and mobile device pixel ratios:
```bash
replayfix inoculate \
  --manifest <MANIFEST_PATH> \
  --matrix chromium,firefox,webkit,mobile \
  --output <TEST_FILE_PATH>
```

---

### 7. `replayfix verify-receipt`
Independently verifies an RFC 6962 Merkle tree audit receipt through all 4 cryptographic trust layers:
```bash
replayfix verify-receipt <RECEIPT_PATH> [--trusted-key <64_HEX_PUBKEY>] [--json]
```

---

### 8. `replayfix notary`
Interacts with the append-only cryptographic transparency ledger (`transparency_log.json`):
```bash
# Append receipt to transparency log:
replayfix notary append --receipt <RECEIPT_PATH> [--log <LOG_PATH>]

# Verify entire ledger integrity:
replayfix notary verify-log --log <LOG_PATH>

# Compute RFC 6962 consistency proof between tree sizes:
replayfix notary consistency-proof --first <N> --second <M> --log <LOG_PATH>
```

---

## Deterministic Exit Codes

| Exit Code | Constant | Meaning |
| :---: | :--- | :--- |
| `0` | `SUCCESS` | Pipeline succeeded and repair verified. |
| `1` | `DOCTOR_ERROR` | Missing platform dependencies. |
| `2` | `USAGE_ERROR` | Invalid CLI flags or malformed inputs. |
| `10` | `GATE1_NOT_RED` | Bug failed to reproduce counterfactually on baseline. |
| `11` | `SECURITY_VIOLATION` | Base repo mutation, path traversal, or anti-cheat violation. |
| `21` | `QUARANTINE_FAILED` | Repair attempt budget exhausted. |
| `31` | `REGRESSION_DETECTED` | Candidate patch broke pre-existing regression tests. |
| `40` | `RECEIPT_INVALID` | Invalid Merkle proof or unverified Ed25519 signature. |

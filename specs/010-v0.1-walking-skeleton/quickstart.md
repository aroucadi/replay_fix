# Quickstart Guide: 010-v0.1-walking-skeleton

**Feature**: Environment Diagnostics & Repro Manifest Ingestion  
**Date**: 2026-10-26  

---

## 1. Environment Diagnostics (`replayfix doctor`)

### Human-Readable Diagnostic Matrix
Inspect all required host dependencies and print formatted table:

```bash
cargo run --release -p replayfix-cli -- doctor
```
*Expected Outcome*:
- Renders table with Git, Node.js, Playwright, and FFmpeg.
- Exits with 0 if all required tools are present; prints exact remediation if missing.

### Machine-Readable JSON Diagnostics
Emit structured JSON for CI automation or orchestrators:

```bash
cargo run --release -p replayfix-cli -- doctor --json
```

---

## 2. Repro Manifest Ingestion (`replayfix repro`)

### CLI Argument Ingestion
Validate required manifest arguments via command line:

```bash
cargo run --release -p replayfix-cli -- repro \
  --task-id RF-01 \
  --title "Null Cart Dereference" \
  --repo ./target-app \
  --bug-ref bug/RF-01 \
  --video recordings/RF-01.mp4 \
  --headed
```

### JSON Manifest File Ingestion
Ingest directly from an existing manifest file:

```bash
cargo run --release -p replayfix-cli -- repro --manifest task_manifest.json
```

### Traversal Sanitization Defense
Verify that path traversal attempts are rejected:

```bash
cargo run --release -p replayfix-cli -- repro \
  --task-id RF-01 \
  --repo ../../../etc \
  --bug-ref bug/RF-01 \
  --video test.mp4
```
*Expected Outcome*: Aborts with `SECURITY_VIOLATION` (exit code 11).

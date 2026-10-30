# ADR-004: Synchronized Three-Act Demonstration Video & Evidence Manifest

## Status
Accepted

## Context
Hackathon judges require a 3-minute (`180.0s`) demonstration video (`replayfix-v1.0.0-demo.mp4`) that is 100% grounded in real CLI execution and verifiable artifacts, structured across Act 1 (Grounded Defect), Act 2 (Cheating & No-Op Trap Rejection), and Act 3 (Autonomous Causal Repair & Cryptographic Proof).

## Decision
1. Execute the real signed `v1.0.0` `replayfix.exe` binary across all three acts and persist the resulting JSON receipts and triage reports in `release/v1.0.0/examples/sanitized-receipts/`.
2. Record every command, exit code, and SHA-256 digest in `release/v1.0.0/demo/demo-evidence-manifest.json`.
3. Render `release/v1.0.0/demo/replayfix-v1.0.0-demo.mp4` (`1920x1080`, `30fps`, `180.0s`, H.264 + AAC) and `release/v1.0.0/demo/captions/replayfix-v1.0.0-demo.vtt` using `ffmpeg`.

# Contract: Synchronized Three-Act Judge Demonstration Video (`replayfix-v1.0.0-demo.mp4`)

## 1. Technical Encoding Specifications
- **File Path**: `release/v1.0.0/demo/replayfix-v1.0.0-demo.mp4`
- **Duration**: $180.0\text{ seconds}$ ($3\text{ minutes}$)
- **Video Stream**: `1920x1080` (16:9 Full HD), `30 fps`, H.264 (`libx264`, `yuv420p`)
- **Audio Stream**: AAC (`48000 Hz` or `24000 Hz`), synchronized narration track
- **Captions**: `release/v1.0.0/demo/captions/replayfix-v1.0.0-demo.vtt` (WebVTT)

## 2. Three-Act Structure & Evidence Linkage
- **Act 1 (`00:00`–`00:45`) — Grounded Defect**:
  - Demonstrates `RF-01` (`src/cart-summary.js` empty array defect), video ROI keyframe extraction, SAM 3.1 `<|box;...|>` centroid snapping, CDP `ariaSnapshot` grounding, and Gate 1 RED baseline.
- **Act 2 (`00:45`–`01:30`) — Cheating & No-Op Trap Rejection**:
  - Demonstrates `replayfix` deterministically rejecting a no-op patch (`REPAIR_NOOP_REJECTED`, Exit `30`) and a test-tampering patch (`TEST_MUTATION_PROHIBITED`, Exit `11`).
- **Act 3 (`01:30`–`03:00`) — Autonomous Causal Repair & Cryptographic Proof**:
  - Demonstrates verified repair (`VERIFIED_REPAIRED`, Exit `0`), 2-run Gate 2 stability, active regression parity ($F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$), 5-bug benchmark sweep (`RF-01`..`RF-05`), `replayfix verify-receipt`, and `proof_viewer.html` inspection.

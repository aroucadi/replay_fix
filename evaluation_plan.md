## Architectural Review & Security Audit Plan

### 1. Core Solution Capabilities
* **Task Ingestion & Triage:** Extracts reproduction steps from bug reports (Markdown) and video recordings. Has a GitHub webhook integration (HMAC-SHA256 authenticated) to autonomously triage issues and even post Merkle-signed triage reports.
* **Multimodal Perception & Grounding:** Integrates SAM (Segment Anything Model) for UI segmentation/snapping and ASR (Automatic Speech Recognition) via `muse-voice-transcribe` to process voice commands in video bugs.
* **Deterministic Sandboxing:** Employs dual-layer immutability with an ephemeral worktree (`ScratchSandbox`), enforcing `Delta-Repo = 0`. Path operations are vetted against path traversal, UNC injections, and ADS.
* **Harness & Differential Execution Engine:** A dual-gate FSM (Finite State Machine) verifies the bug (Gate 1 RED), applies model-generated fixes (via Meta models like Muse Spark/Wonder), and verifies stability (Gate 2 GREEN) without regressions.
* **Cryptographic Attestations (Merkle Receipts):** Emits RFC 6962 append-only transparency receipts with pure RFC 8032 Ed25519 signatures, verifying all execution phases and disaggregated evidence.
* **Browser Automation Bridge:** CDP (Chrome DevTools Protocol) integration via Playwright for reproducing DOM interactions (Grounding invariant: `ariaSnapshot`).
* **Swarm & APM observability:** Supports distributed agents and APM (Application Performance Monitoring) ingestion (RRWeb/Sentry).

### 2. Technical Debt / Code Smells
* **Overly Complex Boolean Logic:** `crates/replayfix-core/src/ingestion.rs` has a redundant logic check `if in_repro_section || !in_repro_section` that is literally `true`.
* **Testing Fragility:** Multiple integration tests in `replayfix-cli/tests/cli.rs` fail because `ffmpeg` and `playwright` are not mock-isolated properly, causing CI/CD breakages if run in an environment missing those binaries.
* **Panic Usage in Tests / Tooling:** Test assertions aggressively `unwrap()` and panic (`crates/replayfix-core/src/perception.rs`, `crates/replayfix-core/src/browser.rs`), hiding underlying error dynamics in failure scenarios.

### 3. Potential Security or Logic Vulnerabilities
* **Path Sanitization Completeness:** `assert_safe_path` explicitly mitigates path traversals, UNC paths, and ADS, but symlink/reparse point validation logic might have timing gaps (TOCTOU) during test harness setup.
* **Logic Bug in Repro Extraction:** The `if in_repro_section || !in_repro_section` in `ingestion.rs` causes `is_action_oriented(rest)` or steps parsing to inadvertently execute globally, potentially polluting the reproduction steps payload with non-reproduction list items.
* **Differential Matrix Loophole:** In `crates/replayfix-core/src/fsm/matrix.rs`, the causal decision matrix returns `InvalidTestOverfit` or `CleanPassUnreproduced` based strictly on `baseline.passed` and `clean.passed` being evaluated first, which might shadow later patch evaluation states if the baseline unexpectedly passed.

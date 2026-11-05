# ReplayFix Architecture & Verification Substrate

This document details the systems architecture of **ReplayFix**, explaining how the Rust verification harness interfaces with browser automation, Meta AI models, and cryptographic notary ledgers.

---

## 1. High-Level System Architecture

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        REPLAYFIX v2.0.0 SOVEREIGN NEURAL PIPELINE                      │
│                                                                                        │
│   ┌───────────────────────────────────┐    ┌───────────────────────────────────────┐   │
│   │   1. SPATIAL PERCEPTION           │    │   2. CAUSAL REASONING                 │   │
│   │   Meta SAM 3.1                    │    │   Meta Muse Spark 1.3                 │   │
│   │   • Video keyframe extraction     │───▶│   • 305+ tok/s deliberation stream    │   │
│   │   • Coordinate & DPR calibration  │    │   • Action trajectory reconstruction  │   │
│   │   • Sub-pixel centroid snapping   │    │   • Gate 1 RED test synthesis         │   │
│   └───────────────────────────────────┘    └───────────────────┬───────────────────┘   │
│                                                                │                       │
│                                                                ▼                       │
│   ┌───────────────────────────────────┐    ┌───────────────────────────────────────┐   │
│   │   4. SOVEREIGN VERIFICATION       │    │   3. DETERMINISTIC REPAIR             │   │
│   │   Rust Verification Harness       │    │   Meta Muse Wonder 1.0                │   │
│   │   • Ephemeral ScratchSandbox      │◀───│   • AST search/replace block patching │   │
│   │   • Gate 2 GREEN stability pass   │    │   • Zero hallucination, zero style    │   │
│   │   • RFC 6962 Merkle proof notary  │    │     drift                             │   │
│   └───────────────────────────────────┘    └───────────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. The Two-Gate Causal Verification Oracle

ReplayFix models bug verification as a formal finite-state machine (FSM) governed by counterfactual causality:

```text
              [Unpatched Base Repo]
                       │
                       ▼
             Gate 1: Repro Attempt
                       │
           ┌───────────┴───────────┐
           ▼                       ▼
    [Test Passes (GREEN)]    [Test Fails (RED)]
    State: ALREADY_HEALTHY   State: CAUSAL_ISOLATION_VERIFIED
    Exit Code 10 (Aborted)         │
                                   ▼
                         [Ephemeral Scratch Clone]
                         (Δrepo = 0, Port :0)
                                   │
                                   ▼
                         Meta Muse Wonder Patch
                                   │
                                   ▼
                         Gate 2: Stability Pass
                                   │
                       ┌───────────┴───────────┐
                       ▼                       ▼
                [Pass 1 & Pass 2]       [Any Pass Fails]
                 GREEN Confirmed         State: QUARANTINED
                       │                 Exit Code 21/30
                       ▼
             [Regression Parity Suite]
             (Full existing tests pass 100%)
                       │
                       ▼
             [RFC 6962 Merkle Receipt]
```

### Gate 1 (Counterfactual RED)
To eliminate spurious tests that pass on both broken and unbroken code, the synthesized test MUST execute on the unpatched codebase and fail with an explicit `AssertionError`. If the test passes, ReplayFix aborts with exit code `10` (`GATE1_NOT_RED`).

### Gate 2 (2-Run Stability GREEN)
Once a patch is applied to the ephemeral scratch clone, ReplayFix executes the synthesized test twice consecutively (`--repeat-each=2`) to guarantee the repair is immune to race conditions and timing flakes.

---

## 3. Spatial Perception & Coordinate Calibration

Video click recordings are inherently noisy due to human cursor jitter, browser scaling, and high-DPR screens:
- **Optical Flow Region of Interest (ROI)**: FFmpeg extracts keyframes around click timestamps, computing inter-frame pixel deltas across 120x120px bounding regions.
- **SAM 3.1 Zero-Shot Segmentation**: The SAM model extracts candidate element contours.
- **Coordinate Transformation Matrix**:
  $$\begin{bmatrix} X_{\text{dom}} \\ Y_{\text{dom}} \end{bmatrix} = \frac{1}{\text{DPR}} \begin{bmatrix} X_{\text{video}} - \text{Offset}_X \\ Y_{\text{video}} - \text{Offset}_Y \end{bmatrix}$$
- **Interactive Grounding**: If multiple interactive elements share the click bounding box, ReplayFix evaluates the Playwright `ariaSnapshot` accessibility tree to resolve semantic dominance.

---

## 4. Multi-Worker Swarm & Rate Governance

For distributed execution across multi-core systems (`replayfix swarm --workers <N>`):
- **Work-Stealing Queue**: Tasks are partitioned across Tokio worker tasks.
- **ScratchSandbox Isolation**: Each worker provisions an independent temporary directory.
- **Dynamic Port Allocation**: Target web applications bind to `:0` (kernel-assigned loopback port) to eliminate cross-worker port collisions.
- **RatePool & CircuitBreaker**: Shared token-bucket limiter bounds Meta Muse token consumption and handles exponential backoff on HTTP 429 responses.

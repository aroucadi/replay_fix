# ReplayFix v2.1.0 — Universal Zero-Shot Video-to-Patch Pipeline Specification

**Document ID**: 210-v2.1.0-universal-zero-shot-pipeline  
**Document Version**: 2.1.0  
**Status**: Authoritative Architectural Specification  
**Authority**: ReplayFix Architecture Council & Clean-Room Verification Substrate  
**Target Release**: `release/v2.1.0-universal-zero-shot`  

---

## 1. Executive Summary & Objective

ReplayFix v2.0 provided a robust verification substrate, Merkle receipts, differential diagnosis, and live streaming dashboards. However, several critical execution paths in `heal.rs`, `deliberation.rs`, `browser.rs`, `main.rs`, and `sweep.rs` retained legacy fallback branches bound to fixed fixture identifiers (`RF-01` through `RF-05`), static source files (`src/App.tsx`), hardcoded click coordinates (`100, 100`), or stubbed worker results.

The objective of **ReplayFix v2.1.0** is to achieve **100% Repository-Agnostic, Zero-Shot Video-to-Patch Execution**:
Given **any arbitrary web repository** (`--repo <path>`) and **any arbitrary screen recording** (`--video <file.mp4>`), ReplayFix shall autonomously:
1. Boot the target repository's live dev server on an ephemeral OS-assigned port (`:0`) across any project topology (Vite, Next.js, CRA, Express, or nested workspace/sample packages such as `vite sample`).
2. Ingest the video recording, extract keyframes, detect interactive human pointer trajectories (including multi-step clicks, drags from `(x1, y1)` to `(x2, y2)`, and typing inputs), and ground them via CDP into resilient semantic DOM locators and coordinate actions.
3. Discover the real source code targets in the target repository without hardcoded paths or pre-canned buggy source overwrites, correlating runtime errors, DOM classes, component names, and accessibility hierarchies with the workspace AST.
4. Execute Gate 1 Baseline Reproduction in an isolated ephemeral clone ($\Delta\text{repo} = 0$), capturing real runtime failure signals (console uncaught exceptions, DOM layout jumps/bounding box jitter, unhandled 5xx network responses, or visual state divergence).
5. Synthesize a surgical unified AST patch via live code context and Meta Muse Wonder, apply it within the isolated scratch sandbox, and verify Gate 2 GREEN stability with 0 regressions on pre-existing tests.
6. Generate a tamper-evident RFC 6962 7-leaf Merkle receipt signed with Ed25519 for the unseen defect.

---

## 2. Non-Negotiable Engineering Invariants

1. **Zero Hardcoded Fixture Coupling**:
   - The harness MUST NOT default to, assume, or require `RF-01`..`RF-05` contracts for unseen videos or repositories.
   - The sandbox MUST NOT overwrite target files with pre-baked `fixture.buggy_source`.
   - All source code context sent to repair models MUST originate directly from the target repository's verified files on disk.
2. **Substrate Primacy & Fail-Closed Authority**:
   - The Rust harness is the sole judicial authority. State transitions (`Init` -> `Gate1Red` -> `Repair` -> `Gate2Green`) require empirical browser telemetry and runtime verification.
   - Ambiguous grounding, timeouts, unhandled crashes, or missing dev servers fail closed.
3. **Repository Immutability ($\Delta\text{repo} = 0$)**:
   - Target base repository remains 100% read-only. Pre- and post-manifest SHA-256 tree walks and `git status --porcelain` verify zero workspace drift.
4. **Anti-Cheating Test Freeze**:
   - Any synthesized repro spec (`.replayfix_repro.spec.ts`) is frozen via SHA-256 before repair. Patches modifying test files or configuration are rejected with exit code 20 (`TEST_MUTATION_PROHIBITED`).
5. **Universal Gesture Support**:
   - Playwright/CDP bridge MUST support pointer drag gestures (`mousedown` -> `mousemove` with dynamic intermediate steps -> `mouseup`) and keyboard typing, in addition to discrete clicks.
6. **BeautifulRust Discipline**:
   - `#![forbid(unsafe_code)]`, zero unhandled `.unwrap()` outside tests, strongly-typed `CoreError` with `thiserror`, strict clippy cleanliness (`cargo clippy --workspace --all-targets -- -D warnings`).

---

## 3. Detailed Architecture & Technical Requirements

### 3.1 Component 1: Universal Dev Server & Target Discovery
- **Subpath & Monorepo Support**:
  - `ManagedSpaServer` shall inspect `package.json` at repo root and common subpaths (`sample/`, `demo/`, `examples/`, `packages/*`).
  - If a package defines scripts (e.g. `sample`, `dev`, `start`), or if Vite is installed as a devDependency without a root start script, it launches `npx vite sample --port :0` or the resolved dev command.
- **AST & Source File Resolution (`RepoSourceMapper`)**:
  - Implement universal source file discovery:
    - Search repository source files (`.ts`, `.tsx`, `.js`, `.jsx`, `.vue`, `.svelte`, `.html`) for matching DOM tags, class names, test IDs, component identifiers, or stack traces from runtime console errors.
    - Fallback to project entrypoints (e.g. `src/index.tsx`, `lib/Resizable.tsx`, `server.js`, `public/app.js`) resolved dynamically via project layout rather than hardcoded `src/App.tsx`.

### 3.2 Component 2: Gesture Trajectory & CDP Bridge (`browser.rs`)
- **Pointer Drag & Input Support**:
  - Extend Node.js Playwright bridge IPC protocol to handle:
    - `DRAG_AND_OBSERVE`: accepts `from_x, from_y, to_x, to_y, steps, role, accessible_name, selector, test_id`.
    - `TYPE_AND_OBSERVE`: accepts text input and selector.
    - `CLICK_AND_OBSERVE`: accepts selector fallback in addition to role + accessible_name and test_id.
  - Track DOM geometry before, during, and after gestures to detect layout jumps, jitter, and dimension anomalies.
- **Universal Runtime Signal Capture**:
  - In addition to `page_errors` (uncaught exceptions), capture:
    - Failed network requests (`status >= 400` or aborted).
    - DOM element bounding-box deltas and jitter measurements.
    - Console error/warn messages.

### 3.3 Component 3: Universal Deliberation & Healer (`deliberation.rs`, `heal.rs`)
- **Dynamic Task Context**:
  - Replace static `Fixture::for_task` lookup with dynamic `UniversalTaskContext`:
    - When `--bug <ID>` matches a built-in benchmark fixture, use it as baseline validation.
    - When an unseen video/repo is passed, dynamically derive task ID from the video filename (e.g., `DEMO_react-resizable_issue-136` -> `REACT-RESIZABLE-136`).
    - Synthesize dynamic `BehavioralContract` based on extracted video trajectory, detected UI interaction, and observed failure signals.
- **Genuine Repro & Repair Turns**:
  - Turn 1 (`replayfix_reproduce`): Executes real Playwright reproduction against the live spawned dev server in the scratch clone. Confirms Gate 1 RED based on real browser failure signals.
  - Turn 2 (`replayfix_inspect_trace`): Extracts CDP console logs, DOM snapshot diff, and stack traces to locate candidate source files.
  - Turn 3 (`replayfix_verify_patch`): Reads candidate source file directly from sandbox, synthesizes patch, applies unified diff, and evaluates Gate 2 GREEN stability on the live server.

### 3.4 Component 4: Swarm & Worker Execution (`sweep.rs`)
- **Real Worker Pipeline**:
  - Replace simulated `execute_task` stub in `SweepCoordinator` with real end-to-end execution:
    - Boots `ManagedSpaServer` on worker's allocated ephemeral port.
    - Executes Gate 1 reproduction.
    - Dispatches bounded repair synthesis.
    - Evaluates Gate 2 verification and produces genuine Merkle leaf receipt.

---

## 4. Empirical Evaluation Targets

1. **`react-resizable` Benchmark**:
   - Video: `benchmarks/react-resizable/DEMO_react-resizable_issue-136.mp4`
   - Repo: `benchmarks/react-resizable` (or clone of `https://github.com/react-grid-layout/react-resizable.git`)
   - Target Issue: Issue #136 / #10 Top-Left NW axis delta jitter during drag resize.
   - Verification: Ephemeral Vite sample server boots, pointer drag triggers NW resize, Gate 1 detects delta jitter / layout jump, patch applies bounding-box compensation in `lib/Resizable.tsx`, Gate 2 verifies smooth resizing.
2. **MetaPulse Real Full-Stack Benchmark**:
   - Videos: `benchmarks/metapulse/RF-01` to `RF-20`
   - Repo: `https://github.com/aroucadi/MetaPulse.git` (and local `benchmarks/metapulse`)
   - Verification: Real Node/Express server boots, real Playwright CDP interaction triggers unhandled exception or DOM failure, real patch synthesizes and verifies GREEN.

---

## 5. Acceptance Criteria

- [ ] `Fixture::for_task` is strictly relegated to backwards-compatible test validation; never used as a mandatory fallback for unseen tasks or generic runs.
- [ ] `heal.rs` and `deliberation.rs` execute fully on arbitrary unseen repos and videos without panics or hardcoded source overwrites.
- [ ] `browser.rs` supports `DRAG_AND_OBSERVE`, selector-based clicks, and bounding-box jitter detection.
- [ ] `ManagedSpaServer` automatically handles subpath sample harnesses (e.g., `npx vite sample`).
- [ ] `sweep.rs` executes real verification rather than hardcoded `status = "SUCCESS"`.
- [ ] Live execution passes on `react-resizable` and `metapulse`.
- [ ] All unit, integration, and CLI tests pass cleanly; `cargo clippy --workspace --all-targets -- -D warnings` reports 0 warnings.

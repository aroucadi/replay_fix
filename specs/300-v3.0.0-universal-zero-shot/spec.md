# ReplayFix v3.0.0 — Specification: Universal Zero-Shot Video-to-Code Repair

**Spec ID**: 300-v3.0.0-universal-zero-shot  
**Branch**: `release/v3.0.0-universal-zero-shot`  
**Status**: Active  

---

## 1. Executive Summary & Architectural Mandate

ReplayFix v3.0.0 establishes a genuinely repository-agnostic, zero-shot video-driven autonomous debugging and repair pipeline powered by real Meta AI models (`muse-spark-1.3`, `muse-wonder-1.0`, `sam-3.1`, `muse-voice-transcribe-1.0`) and authentic browser execution.

### Non-Negotiable Mandates
1. **Zero Fixture Fallbacks in Production (`LiveAi`)**: In `LiveAi` / `LiveEndpoint` mode, `Fixture::for_task` and `Fixture::for_task_in_repo` are completely bypassed across CLI, engine, deliberation, and MCP surfaces.
2. **Authentic Component & SPA Execution**: Repositories with runnable dev servers launch via `ManagedSpaServer`; component libraries without prebuilt bundles synthesize a generic browser harness (`synthesize_component_harness_html`) supporting ES6 modules, JSX, TypeScript, and Flow (`presets: ['env', 'react', 'flow']`).
3. **Interactive Handle & Aria Grounding**: `PROBE_PAGE` and `LocatorResolver` discover and ground both standard ARIA widgets and interactive drag/resize handles (`[class*="handle"]`, `[role="slider"]`, `[role="separator"]`), synthesizing accessible labels for handle axes (`nw`, `se`, etc.) when none are explicitly provided by the component.
4. **Physical Bounding-Box Jitter Detection**: When resizing from top or left handles (`nw`, `n`, `w`, `sw`, `ne`), the component harness updates both dimensions and position offset so physical handle movement in the viewport exposes uncompensated delta feedback loops (`jitter_score_bps > 500`) in Gate 1 (RED) and verifies smooth convergence (`jitter_score_bps <= 500`) in Gate 2 (GREEN).
5. **Honest Stage Provenance (`StageExecutionMatrix`)**: Every `AuditReceipt` records exact `StageExecutionStatus` (`REAL`, `HEURISTIC`, `MOCKED`, `HARDCODED`, `SKIPPED`, `FAILED`, `UNVERIFIED`) per pipeline stage.

---

## 2. Functional Requirements

- **FR-300-01 (Flow/ES Module Component Harness)**: `RepoSourceMapper::synthesize_component_harness_html` MUST transpile source modules with `Babel.transform(..., { presets: ['env', 'react', 'flow'] })` and provide a functional pointer-drag `DraggableCore` fallback when UMD `ReactDraggable` does not match the React version or is absent.
- **FR-300-02 (Interactive Handle Discovery)**: `PlaywrightBridge` `PROBE_PAGE` MUST include `[role="slider"]`, `[role="separator"]`, and `[class*="handle"]` in candidate discovery, inferring role `slider` and a deterministic accessible name for unnamed handle elements so `ariaSnapshot` and `LocatorResolver` ground them unambiguously.
- **FR-300-03 (Dynamic Gate 1 Failure Context)**: `HealEngine::execute_autonomous_repair_loop` MUST pass the actual observed Gate 1 failure diagnostic (runtime exceptions, drag jitter on grounded handle, and action trajectory) into `PromptBuilder::build_user_prompt`.
- **FR-300-04 (Dual-Gate Immutability)**: Base repository invariance ($\Delta	ext{repo} = 0$) and `TestFreezeGuard` SHA-256 verification MUST hold across all runs.

# ReplayFix v2.1.0 — Artifact Consistency & Quality Analysis

**Spec ID**: 210-v2.1.0-universal-zero-shot-pipeline  
**Analyzed Artifacts**: `spec.md`, `plan.md`, `tasks.md`  
**Status**: Consistent & Verified  

---

## 1. Traceability & Invariant Mapping Matrix

| Specification Requirement (`spec.md`) | Implementation Plan (`plan.md`) | Execution Tasks (`tasks.md`) | Constitutional Law Alignment |
| :--- | :--- | :--- | :--- |
| Zero hardcoded `RF-01`..`RF-05` coupling & no `buggy_source` sandbox overwrite | Phase 3: Universal Deliberation & Zero-Shot Healing | Tasks 210-06, 210-07, 210-08 | Law VII (Zero Hardcoded Coupling), Law III ($\Delta\text{repo}=0$) |
| Universal gesture support (`DRAG_AND_OBSERVE`, typing, selector clicks) | Phase 1: Browser Bridge & Gesture Extension | Tasks 210-01, 210-02, 210-03 | Law V (Semantic Accessibility & Grounding) |
| Monorepo & subpath dev server detection (`npx vite sample`) | Phase 2: Dev Server & Target Discovery | Task 210-04 | Law IV (Child-Owned Ephemeral Provenance) |
| AST & repository target source discovery (`RepoSourceMapper`) | Phase 2: Dev Server & Target Discovery | Task 210-05 | Law I (Substrate Primacy), Law III ($\Delta\text{repo}=0$) |
| Genuine sweep worker execution without fake `status = "SUCCESS"` | Phase 3: Zero-Shot Deliberation | Task 210-09 | Law I (Substrate Primacy), Law VI (Honest Provenance) |
| Empirical validation on `react-resizable` & `MetaPulse` | Phase 4: Live Benchmark Testing | Tasks 210-10, 210-11, 210-12 | Law VIII (Benchmark Integrity) |
| Zero panics & Clippy clean verification | Phase 5: Verification & Quality Gate | Tasks 210-13, 210-14, 210-15 | Law X (BeautifulRust Discipline) |

---

## 2. Risk Assessment & Mitigations

1. **Risk: Dev server startup divergence on non-standard repos**
   - *Mitigation*: Fallback chain in `ManagedSpaServer`: inspect package scripts -> check devDependencies (vite/next) -> check standard subdirectories (`sample/`, `demo/`, `examples/`). If a port is specified or needed, always pass `--port :0` or parse bound ephemeral port from stdout.
2. **Risk: Dynamic locator ambiguity on complex canvas/resizable drag handles**
   - *Mitigation*: Support cascading locator targets: `test_id` -> `role + accessible_name` -> CSS selector (`.react-resizable-handle-nw`) -> exact pixel coordinate drag with bounding box observation.
3. **Risk: Regression on existing test suites**
   - *Mitigation*: Existing unit tests in `cli.rs` and other modules that specifically test mock fixtures will continue to pass because `Fixture::for_task` remains available for explicitly requested fixtures, while generic runs no longer rely on it as a mandatory crutch.

# ReplayFix v2.1.0 — Execution Tasks: Universal Zero-Shot Pipeline

**Spec ID**: 210-v2.1.0-universal-zero-shot-pipeline  
**Branch**: `release/v2.1.0-universal-zero-shot`  

---

## Task Breakdown

### Phase 1: Browser Bridge & Universal Gestures
- [x] **Task 210-01**: Extend `BridgeCommand` and Playwright Node.js script in `crates/replayfix-core/src/browser.rs` to support `DRAG_AND_OBSERVE` (pointer drag gesture, coordinate interpolation, and bounding box delta tracking) and selector/coordinate click fallbacks.
- [x] **Task 210-02**: Add `ActionType::Drag` and drag coordinates tracking in `crates/replayfix-core/src/perception.rs` and `video.rs`.
- [x] **Task 210-03**: Unit test gesture execution, drag trajectory, and bounding-box jitter detection in `browser.rs`.

### Phase 2: Dev Server & Target Source Discovery
- [x] **Task 210-04**: Enhance `ManagedSpaServer` in `crates/replayfix-core/src/environment.rs` to detect custom dev/sample scripts (e.g. `npx vite sample --port :0`) and nested subpaths (`sample/package.json`).
- [x] **Task 210-05**: Implement `RepoSourceMapper` in `crates/replayfix-core/src/engine.rs` (or new module `crates/replayfix-core/src/discovery.rs`) to scan target repo AST and file paths for matching selectors, component names, and error stack trace frames without hardcoded file paths.

### Phase 3: Zero-Shot Deliberation, Healer & CLI Decoupling
- [x] **Task 210-06**: Refactor `DeliberationEngine` in `crates/replayfix-core/src/deliberation.rs` to support `UniversalTaskContext`, dynamic `BehavioralContract` synthesis, live Playwright Turn 1 Gate 1 reproduction, and sandbox source file patching.
- [x] **Task 210-07**: Refactor `heal.rs` in `crates/replayfix-cli/src/heal.rs` to remove hardcoded `RF-01`..`RF-05` branching and eliminate `fixture.buggy_source` overwriting of sandbox files.
- [x] **Task 210-08**: Fix CLI argument validation in `crates/replayfix-cli/src/main.rs` to allow `--output-dir` universally, and eliminate dummy fallback `("src/App.tsx", 100, 100)` in favor of `RepoSourceMapper` and video trajectory.
- [x] **Task 210-09**: Refactor `crates/replayfix-core/src/sweep.rs` so worker `execute_task` runs real end-to-end verification instead of hardcoded success stubs.

### Phase 4: Live Empirical Validation
- [x] **Task 210-10**: Setup and verify `benchmarks/react-resizable` with pre-fix commit and `DEMO_react-resizable_issue-136.mp4`.
- [x] **Task 210-11**: Execute end-to-end ReplayFix repro and heal on `react-resizable`, verifying drag reproduction and fix verification.
- [x] **Task 210-12**: Execute end-to-end ReplayFix on `MetaPulse` benchmarks.

### Phase 5: Quality Gate & BeautifulRust Audit
- [x] **Task 210-13**: Run full workspace test suite `cargo test --workspace` ensuring 100% pass parity.
- [x] **Task 210-14**: Run `cargo clippy --workspace --all-targets -- -D warnings` and verify zero warnings.
- [x] **Task 210-15**: Verify zero panics, check documentation and commit release.

# ReplayFix v2.1.0 — Technical Implementation Plan: Universal Zero-Shot Pipeline

**Spec ID**: 210-v2.1.0-universal-zero-shot-pipeline  
**Branch**: `release/v2.1.0-universal-zero-shot`  
**Status**: Approved for Autonomous Execution  

---

## 1. Constitutional Alignment & Governance Laws

| Law | Title | Implementation Invariant in v2.1.0 |
| :--- | :--- | :--- |
| **Law I** | Substrate Primacy & Fail-Closed Authority | Universal pipeline uses real browser telemetry (errors, bounding-box jitter, DOM assertions) to verify Gate 1 RED and Gate 2 GREEN. Missing signals or timeouts fail closed. |
| **Law II** | Behavioral Contract Invariance | Dynamic `BehavioralContract` synthesized for unseen defects is hashed with RFC 8785 canonical digest and kept immutable across Gate 1 and Gate 2. |
| **Law III** | Base Workspace Isolation ($\Delta\text{repo} = 0$) | Ephemeral `ScratchSandbox` in temp directory; pre/post `WorktreeManifest` SHA-256 tree walk confirms base repo untouched. |
| **Law IV** | Child-Owned Ephemeral Provenance | Ephemeral port `:0` used for all spawned dev servers (`ManagedSpaServer`). |
| **Law V** | Semantic Accessibility & Gesture Grounding | Extended CDP bridge supporting `DRAG_AND_OBSERVE`, `CLICK_AND_OBSERVE`, and `TYPE_AND_OBSERVE` with bounding-box delta tracking. |
| **Law VI** | Honest Model Provenance | Model IDs (`muse-spark-1.3`, `muse-wonder-1.0`, or local fallback) recorded accurately in all Merkle receipts. |
| **Law VII** | Zero Hardcoded Coupling | Complete removal of `fixture.buggy_source` overwrites, hardcoded `"src/App.tsx"` fallbacks, and simulated sweep success stubs. |
| **Law X** | BeautifulRust Discipline | `#![forbid(unsafe_code)]`, zero panics, strict error typing with `thiserror`, `cargo clippy --workspace --all-targets -- -D warnings`. |

---

## 2. Technical Architecture & Phased Execution

### Phase 1: Browser Bridge & Gesture Extension (`crates/replayfix-core/src/browser.rs`)
1. Extend `BridgeCommand` with:
   - `DragAndObserve`: `from_x: i32, from_y: i32, to_x: i32, to_y: i32, steps: u32, role: Option<String>, accessible_name: Option<String>, selector: Option<String>, test_id: Option<String>`.
   - `TypeAndObserve`: `text: String, selector: Option<String>, role: Option<String>, accessible_name: Option<String>`.
   - Enhance `ClickAndObserve`: Add `selector: Option<String>` and direct coordinate fallback when role/name is absent.
2. In the Node.js Playwright bridge script (`BRIDGE_SCRIPT`):
   - Implement `DRAG_AND_OBSERVE`:
     - Mouse move to `(from_x, from_y)`, `mouse.down()`.
     - Smooth interpolation or intermediate steps to `(to_x, to_y)`, measuring bounding box changes and position deltas on targeted elements.
     - `mouse.up()`.
     - Record `bounding_box_jitter`, `layout_jumps`, `page_errors`, and final geometry.
   - Implement selector fallback on `CLICK_AND_OBSERVE` (e.g. `page.locator(cmd.selector)` or `page.mouse.click(cmd.click_x, cmd.click_y)`).
3. Extend `ActionType` in `perception.rs`:
   - Add `Drag { to_x: i32, to_y: i32 }` or `ActionType::Drag`.

### Phase 2: Universal Dev Server & Source File Discovery (`crates/replayfix-core/src/environment.rs`, `engine.rs`)
1. Enhance `ManagedSpaServer`:
   - Inspect `package.json` scripts: if `sample` exists (as in `react-resizable`), support launching `npx vite sample --port {port}` or `npm run sample`.
   - Support subfolder package manifests (`sample/package.json`, `demo/package.json`).
2. Implement `RepoSourceMapper` in `replayfix-core`:
   - `discover_target_files(repo_root: &Path, query: &SourceQuery) -> Result<Vec<PathBuf>>`:
     - Given DOM candidates (selectors, class names, test IDs, roles) or stack traces, search repo source files for matching definitions, exports, and event handlers.
     - Rank candidates by relevance score.
     - If no direct match, locate primary entrypoints or component directories (`lib/`, `src/components/`, `src/`).

### Phase 3: Universal Deliberation & Zero-Shot Healing (`deliberation.rs`, `heal.rs`, `main.rs`)
1. Refactor `DeliberationEngine`:
   - Support `UniversalTaskContext`:
     - Construct dynamic `BehavioralContract` for unseen defects if no static fixture is found.
     - Turn 1: Run real Playwright test / CDP trajectory on live dev server. Catch real page errors or layout jitter.
     - Turn 2: Trace and identify real candidate source files from target repo.
     - Turn 3: Read candidate source file directly from sandbox worktree. Synthesize patch via `RepairClient`. Apply patch with `UnifiedPatcher`. Run Gate 2 verification against live server.
2. Refactor `heal.rs`:
   - Strip hardcoded `RF-01`..`RF-05` detection. Derive task name from video or repo path.
   - Do NOT overwrite sandbox file with `fixture.buggy_source`. Keep sandbox identical to base repo ($\Delta\text{repo} = 0$).
   - Run universal deliberation loop.
3. Refactor `main.rs`:
   - Remove dummy placeholder fallback `("src/App.tsx", 100, 100)`. Use `RepoSourceMapper` and video's extracted `InteractionEvidence` trajectory.
   - Fix `--output-dir` CLI validation so `--output-dir` is always allowed.
4. Refactor `sweep.rs`:
   - Wire `execute_task` to real `ScratchSandbox`, dev server, and verification rather than fake hardcoded success stubs.

### Phase 4: Live Benchmark Testing on `react-resizable` & `MetaPulse`
1. Test `react-resizable`:
   - Video: `benchmarks/react-resizable/DEMO_react-resizable_issue-136.mp4`.
   - Repo: `benchmarks/react-resizable` (with `react-grid-layout/react-resizable` at pre-fix commit `6684a5b1~1`).
   - Run `replayfix repro` / `replayfix heal`.
   - Verify drag gesture, detection of NW delta jitter, patch synthesis in `lib/Resizable.tsx`, and Gate 2 GREEN.
2. Test `MetaPulse`:
   - Video: `benchmarks/metapulse/RF-01_quote_repost_crash_with_cursor.mp4`.
   - Repo: `benchmarks/metapulse`.
   - Verify real Node server boot, CDP reproduction, patch in `public/app.js`, Gate 2 GREEN.

### Phase 5: Verification, BeautifulRust Audit & Quality Gate
1. `cargo test --workspace` — 100% pass across all tests.
2. `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings.
3. Run `rust-review` audit on all modified crates.

# Implementation Plan: Spec 130 — Substrate Truth & Universal SPA Grounding

**Branch**: `release/v1.3.0-substrate-truth-spa` | **Date**: 2026-11-01 | **Spec**: [specs/130-v1.3.0-substrate-truth-spa/spec.md](spec.md)

---

## 1. Summary

Spec 130 (`v1.3.0`) operationalizes substrate truth across ReplayFix by:
1. Deriving `provenance`, `model_provenance`, and `token_accounting` strictly from verified runtime execution telemetry and shielding `--mode live-ai` from offline mock flags.
2. Introducing a clean process-isolated `crates/replayfix-core/src/environment.rs` module providing `SpaToolchain`, `ManagedSpaServer` (dynamic port `:0` allocation, 30s readiness polling, RAII process tree teardown), and `AttachedSpaEndpoint`.
3. Supporting Universal SPA Attached Mode (`--url <endpoint> --repo <path>`) with CDP page reload synchronization and sandbox isolation ($\Delta\text{repo}=0$).
4. Eliminating synthetic HTML fixture hacks (`#btn-publish-quote`) and enabling authentic file-based MetaPulse benchmark execution.

---

## 2. Technical Context

- **Language / Toolchain**: Rust 1.80+ (2021 Edition), `#![forbid(unsafe_code)]`.
- **Primary Dependencies**: `tokio`, `reqwest` (with `rustls-tls`), `serde`, `serde_json`, `thiserror`, `sha2`, `ed25519-dalek`.
- **Storage**: Ephemeral scratch sandboxes in OS temp dir (`std::env::temp_dir()`), atomic JSON artifacts in `.replayfix/`.
- **Testing**: `cargo test --workspace --all-targets`, PowerShell panics linter (`scripts/check-panics.ps1`), strict Clippy (`-D warnings`).
- **Target Platform**: Windows 10/11 x64, Linux x86_64, macOS aarch64.
- **Constraints**:
  - Dev server boot readiness timeout: bounded at $\le 30$ seconds.
  - Process tree cleanup: 100% deterministic kill via `Platform::terminate_process_tree` (using Windows job objects / `taskkill /F /T /PID`).
  - Base repository isolation: strictly $\Delta\text{repo} = 0$ during speculative verification.

---

## 3. Constitution Check

*GATE: All gates must pass before and during implementation per `.specify/memory/constitution.md`.*

| Constitution Law | Compliance Invariant in Spec 130 | Audit Status |
|:---|:---|:---:|
| **Law I: Substrate Primacy** | The deterministic Rust harness is the sole verification authority for all state transitions, budgets, and terminal verdicts. | **PASS** |
| **Law II: Behavioral Contract Invariance** | Gate 1 and Gate 2 evaluate the exact same immutable `BehavioralContract` across both Managed and Attached modes. | **PASS** |
| **Law III: Inviolable Base Workspace Isolation** | $\Delta\text{repo} = 0$ enforced. In Attached Mode, speculative patch verification occurs within an ephemeral scratch worktree; base repository mutation is strictly prohibited during test trials and requires the verified `--apply` / `--yes` gate. | **PASS** |
| **Law IV: Ephemeral Provenance** | Managed Mode binds to child-owned OS dynamic port `:0`. Attached Mode explicitly stamps `substrate: "attached-spa-remote"` in `audit_receipt.json` to prevent ambient server runs from masquerading as child-supervised sandboxes. | **PASS** |
| **Law VII: Model Honesty** | Provenance and model records are derived from verified runtime execution telemetry. Deterministic fixture replays omit `model_provenance` and zero `token_accounting`. | **PASS** |
| **Law X: BeautifulRust Discipline** | Zero non-test `.unwrap()`, all domain errors typed via `CoreError`, zero warnings on `cargo clippy --workspace --all-targets -- -D warnings`. | **PASS** |

---

## 4. Project Structure

### Documentation Layout (this feature)
```text
specs/130-v1.3.0-substrate-truth-spa/
├── spec.md              # Feature specification with prioritized user stories
├── plan.md              # This implementation plan and architecture design
└── tasks.md             # Phased, dependency-ordered implementation tasks
```

### Source Code Layout (workspace)
```text
crates/replayfix-core/
├── src/
│   ├── environment.rs   # [NEW] SpaToolchain, ManagedSpaServer, AttachedSpaEndpoint
│   ├── engine.rs        # LiveBrowserOptions attached_url, provenance defaults
│   ├── fixtures.rs      # Decouple synthetic HTML fixtures, disk-based lookup
│   └── lib.rs           # Export environment module
crates/replayfix-cli/
├── src/
│   └── main.rs          # CLI argument parsing (--url), mock guard, provenance wiring
└── tests/
    └── cli.rs           # Automated integration test coverage
benchmarks/metapulse/
├── package.json         # Real MetaPulse SPA package manifest
├── server.js            # Express/Node SPA static + API server
└── public/
    ├── index.html       # Authentic MetaPulse client markup
    └── app.js           # Client-side routing, stores, and event dispatch
```

---

## 5. Module Architecture & Data Contracts

### 5.1 `crates/replayfix-core/src/environment.rs` (New Module)
- **`SpaToolchain` Enum**:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
  pub enum SpaToolchain {
      Vite,
      NextJs,
      ReactScripts,
      NodeScript,
  }
  ```
  - `SpaToolchain::detect(repo_dir: &Path) -> Result<(SpaToolchain, Vec<String>), CoreError>`:
    - Enforces deterministic detection cascade: `Vite` > `NextJs` > `ReactScripts` > `scripts.dev` > `scripts.start`, reinforced by filesystem config markers (`vite.config.*`, `next.config.*`).
    - Constructs non-interactive dev server command line arguments.
- **`ManagedSpaServer` Struct**:
  - Fields: `child: Child`, `port: u16`, `url: String`, `toolchain: SpaToolchain`, `working_dir: PathBuf`.
  - `spawn(repo_dir: &Path, base_repo_dir: Option<&Path>) -> Result<Self, CoreError>`:
    - Pre-allocates port via OS socket (`Platform::allocate_ephemeral_port()`).
    - Launches child process with `PORT=<port>`, `--port <port>` (or `-p <port>` for Next.js), `BROWSER=none`, `CI=true`.
    - Wires `NODE_PATH` and `PATH` to `base_repo_dir/node_modules`.
    - Spawns background stdout/stderr threads, parsing bound port regex: `http://(?:127\.0\.0\.1|localhost):(\d+)` as secondary confirmation.
    - Polls HTTP readiness (`GET /` or `GET /api/health`) up to 30s.
  - `impl Drop for ManagedSpaServer`: Calls `Platform::terminate_process_tree(child.id())` to eliminate orphaned processes.
- **`AttachedSpaEndpoint` Struct**:
  - `validate(endpoint: &str) -> Result<Url, CoreError>`: Validates scheme (`http://` or `https://`), extracts host/port, and verifies socket reachability.

### 5.2 `crates/replayfix-core/src/engine.rs`
- Add `attached_url: Option<String>` to `LiveBrowserOptions`.
- In `MockHealer::run_live_browser_in_repo`:
  - If `options.attached_url` is `Some(url)`: Attach CDP directly to `url`, execute Gate 1 in scratch sandbox, verify Gate 2 after `Page.reload`, and set `substrate = "attached-spa-remote"`.
  - Else if `ManagedSpaServer::has_package_json(sandbox.root())`: Boot `ManagedSpaServer` with hybrid pre-allocated port, run two-gate verification, and set `substrate = "managed-spa-supervisor"`.
  - Trajectory Execution Synchronization: Between consecutive actions in `ActionTrajectory`, await DOM mutation quiescence (200ms quiet window) and network-idle (capped at 2000ms max).
  - Dev-Server Crash Handling: If a candidate patch causes dev-server process exit or 500 error during Gate 2, capture stderr into `triage_report.json`, reset sandbox to baseline, and trigger the next bounded repair retry.
  - Else: Fall back to static `SandboxServer` (`substrate = "live-browser-cdp"`).
  - Default `AuditReceipt.provenance` to `Some("deterministic-fixture".to_string())`.

### 5.3 `crates/replayfix-cli/src/main.rs`
- Add `--url <endpoint>` flag to `ReproArgs`.
- In `build_manifest`:
  - Validate mutual exclusion: if `--mode live-ai` is combined with any mock flag (`mock_syntax_retry`, `mock_failing_repair`, `mock_pkg_test_mutation`, `mock_patch != "builtin"`), print error and exit with code `2`.
  - Automatically activate two-gate loop if `--bug`, `--url`, or managed SPA `--repo` is passed.
- In `run_two_gate_loop`:
  - Derive `receipt.provenance = "live-ai"` **only** when authentic telemetry is verified (`live_repair_verified || live_sam_verified`).
  - Otherwise stamp `"deterministic-fixture"`, set `model_provenance = None`, and clear `token_accounting = None`.

---

## 6. Verification Strategy

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace --all-targets`
4. End-to-end integration tests in `crates/replayfix-cli/tests/cli.rs`:
   - `test_provenance_honest_on_default_run`
   - `test_provenance_rejects_mock_with_live_mode`
   - `test_deterministic_fixture_clears_token_accounting`
   - `test_managed_spa_server_boots_on_port_0_and_cleans_up`
   - `test_attached_spa_mode_connects_to_external_url`

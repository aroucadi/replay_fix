# Tasks: Spec 150 — Permanent Inoculation, Agentic Protocol (MCP) & Continuous CI/CD Automation (Release 1.5.0)

**Input**: Design documents from `/specs/150-v1.5.0-autonomous-inoculation-mcp/`  
**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/)

**Organization**: Tasks `T001` through `T030` are grouped by phase and user story (`US1`–`US6`) to enable independent implementation and verification while maintaining 100% backward compatibility.

---

## Phase 1: Setup & Domain Foundations

- [x] T001 Extend `CoreError` with `McpProtocol(String)`, `Inoculation(String)`, `StressQuorumFailed(String)`, and `GitPackaging(String)` in `crates/replayfix-core/src/error.rs`
- [x] T002 [P] Extend `SubstrateEvent` with `CdpTelemetry` and `StressQuorumEvaluated` variants in `crates/replayfix-core/src/observer.rs`
- [x] T003 [P] Verify and register `contracts/mcp_tools.json` and `contracts/inoculation.json` schema contracts in `contracts/`

---

## Phase 2: Foundational Core & Bridge Infrastructure

- [x] T004 Extend `BridgeCommand`, `PlaywrightBridge`, and `PLAYWRIGHT_BRIDGE_JS` to support `SET_NETWORK_PERTURBATION` (simulated 3G latency & 500ms jitter) in `crates/replayfix-core/src/browser.rs`
- [x] T005 Extend `AuditReceipt` with optional `stress_quorum` and `inoculation` evidence fields and Merkle leaf bindings in `crates/replayfix-core/src/receipt.rs`

---

## Phase 3: User Story 3 — Stress & Flakiness Gating (Gate 3 Concurrency Quorum) (Priority: P1)

**Goal**: Execute network latency/jitter perturbation and a 3-pass concurrent stability quorum after Gate 2, quarantining flaky patches with `FLAKY_NON_DETERMINISTIC` (Exit Code 21).  
**Independent Test**: Run `replayfix repro --bug RF-01 --mock-heal --stress-quorum` (passes) and with `--mock-flaky` (fails closed with Exit Code 21).

- [x] T006 [US3] Implement `PerturbationProfile`, `StressPassResult`, `StressQuorumReport`, and `StressQuorumRunner` in `crates/replayfix-core/src/stress.rs`
- [x] T007 [US3] Integrate Gate 3 `StressQuorumRunner` and `mock_flaky` / `stress_quorum` options into `MockHealer` in `crates/replayfix-core/src/engine.rs`
- [x] T008 [US3] Export `stress` module types in `crates/replayfix-core/src/lib.rs` and add unit tests for deterministic pass vs. flaky rejection (Exit 21) in `crates/replayfix-core/src/stress.rs`
- [x] T009 [US3] Add `--stress-quorum` and `--mock-flaky` CLI flags to `repro` subcommand in `crates/replayfix-cli/src/main.rs`

---

## Phase 4: User Story 1 — Permanent Codebase Inoculation ("Dreaming") (Priority: P1) 🎯 MVP

**Goal**: Synthesize idiomatic `@playwright/test` specs (`tests/repro/rf_<id>.spec.ts`) from grounded accessibility locators (`getByRole`, `getByText`, `getByTestId`) and maintain `.replayfix/inoculations.json`.  
**Independent Test**: Run `replayfix inoculate --bug RF-01` and `replayfix repro --bug RF-01 --mock-heal --inoculate`, verifying spec synthesis, `.replayfix/inoculations.json` SHA-256 integrity, and RED-on-unpatched / GREEN-on-patched parity.

- [x] T010 [US1] Implement `InoculationSpec`, `InoculationEntry`, `InoculationRegistry`, and `InoculationGenerator` in `crates/replayfix-core/src/inoculate.rs`
- [x] T011 [US1] Implement `verify_inoculation_parity` in `crates/replayfix-core/src/inoculate.rs` proving synthesized specs fail on unpatched baseline and pass on patched code
- [x] T012 [US1] Export `inoculate` module in `crates/replayfix-core/src/lib.rs` and wire optional inoculation synthesis into `crates/replayfix-core/src/engine.rs`
- [x] T013 [US1] Add `inoculate` subcommand, upgrade `freeze-spec`, and add `--inoculate` flag to `repro` in `crates/replayfix-cli/src/main.rs`

---

## Phase 5: User Story 4 — Autonomous Git Packaging & PR Engine (Priority: P2)

**Goal**: Package verified fixes and inoculation tests into isolated branches (`replayfix/fix-<bug_id>`), sign commit trailers with Ed25519 + RFC 6962 Merkle roots, and emit Evidence Matrix PR markdown (`pr_description.md`).  
**Independent Test**: Run `replayfix repro --bug RF-01 --mock-heal --package-pr` and verify branch/commit trailer signatures and `pr_description.md`.

- [x] T014 [US4] Implement `CommitTrailers`, Ed25519 trailer signing & verification, and `GitPrPackager` branch isolation (`replayfix/fix-<bug_id>`) in `crates/replayfix-cli/src/git_packager.rs`
- [x] T015 [US4] Implement `EvidenceMatrixGenerator` producing `pr_description.md` (keyframes, bounding boxes, side-by-side `ariaSnapshot` diffs, gate logs, token accounting) in `crates/replayfix-cli/src/git_packager.rs`
- [x] T016 [US4] Wire `--package-pr` flag into `repro` workflow in `crates/replayfix-cli/src/main.rs`

---

## Phase 6: User Story 2 — Model Context Protocol (MCP) Universal Server (Priority: P1)

**Goal**: Expose ReplayFix over JSON-RPC 2.0 (`stdio` and `SSE/HTTP`) with 5 tools, 3 resources, and 1 prompt (`triage_bug`).  
**Independent Test**: Connect via `stdio` and HTTP/SSE to `replayfix mcp`, testing `initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, and `prompts/get`.

- [x] T017 [US2] Implement MCP JSON-RPC 2.0 protocol types, request router, and `initialize` / `ping` handlers in `crates/replayfix-cli/src/mcp/mod.rs`
- [x] T018 [US2] Implement the 5 MCP tools (`replayfix_reproduce`, `replayfix_verify_patch`, `replayfix_inoculate`, `replayfix_inspect_trace`, `replayfix_doctor`) in `crates/replayfix-cli/src/mcp/mod.rs`
- [x] T019 [US2] Implement the 3 MCP resources (`replayfix://receipts/{task_id}`, `replayfix://sessions/{session_id}`, `replayfix://benchmarks`) and `triage_bug` prompt in `crates/replayfix-cli/src/mcp/mod.rs`
- [x] T020 [P] [US2] Implement bounded line-delimited JSON-RPC 2.0 `stdio` server transport in `crates/replayfix-cli/src/mcp/stdio.rs`
- [x] T021 [P] [US2] Implement bounded `TcpListener` Streamable HTTP / `SSE` server transport (`/sse`, `/message`, `/mcp`) in `crates/replayfix-cli/src/mcp/sse.rs`
- [x] T022 [US2] Wire `mcp` subcommand (`--transport stdio|sse`, `--port`) into `crates/replayfix-cli/src/main.rs`

---

## Phase 7: User Story 5 — Multi-Modal Conversational Copilot & Ambiguity Picker (Priority: P2)

**Goal**: Render ASR spoken narration aligned with video keyframes, prompt an interactive terminal locator picker on candidate score ties ($|\Delta	ext{score}| < 0.05$ / `500` bps), and stream CDP/console/network events.  
**Independent Test**: Verify ambiguity tie detection (`detect_ambiguous_tie`), interactive picker selection in TTY vs. fail-closed `AMBIGUOUS_GROUNDING` in non-TTY, and ASR keyframe transcript rendering.

- [x] T023 [US5] Implement `detect_ambiguous_tie` ($|\Delta	ext{score}| < 0.05$ / `500` bps) and candidate resolution helper in `crates/replayfix-core/src/locator.rs`
- [x] T024 [US5] Add ASR narration + keyframe timestamp alignment display, interactive ambiguity locator picker dialog, and live CDP event streaming in `crates/replayfix-cli/src/shell.rs` and `crates/replayfix-cli/src/intent.rs`

---

## Phase 8: User Story 6 — CI/CD Action & Headless SARIF / JUnit Reporters (Priority: P3)

**Goal**: Export SARIF v2.1.0 (`results.sarif`) and JUnit XML (`junit.xml`) reports and provide a drop-in GitHub Actions `action.yml`.  
**Independent Test**: Run `replayfix repro --bug RF-01 --mock-heal --sarif results.sarif --junit junit.xml` and validate generated files and `action.yml`.

- [x] T025 [P] [US6] Implement `SarifReporter` (`results.sarif`, SARIF v2.1.0) and `JunitReporter` (`junit.xml`) in `crates/replayfix-core/src/reporters.rs` and export in `crates/replayfix-core/src/lib.rs`
- [x] T026 [US6] Wire `--sarif <path>` and `--junit <path>` CLI flags into `repro` and `sweep` subcommands in `crates/replayfix-cli/src/main.rs`
- [x] T027 [P] [US6] Author GitHub Actions custom composite action in `action.yml`

---

## Phase 9: Empirical Validation, Adversarial Testing & Release Packaging

- [x] T028 Author comprehensive unit, integration, and adversarial tests covering Pillars 1–6 in `crates/replayfix-cli/tests/v15_release.rs`
- [x] T029 Run full verification battery (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace`, `.specify/scripts/check-panics.ps1`) and execute `/speckit-converge`
- [x] T030 Bump version to `1.5.0` across workspace, assemble `release/v1.5.0/` (`replayfix.exe`, `proof_viewer.html`, `SHA256SUMS.txt`), write `docs/release-notes/v1.5.0.md`, merge `release/v1.5.0` into `main`, tag `v1.5.0`, and push to `origin`

---

## Dependencies & Execution Order

- **Phase 1 & Phase 2 (`T001`–`T005`)**: Foundational types and bridge primitives; must complete first.
- **Phase 3 (`T006`–`T009`, US3 Gate 3)** & **Phase 4 (`T010`–`T013`, US1 Inoculation)**: Core verification and test synthesis pillars.
- **Phase 5 (`T014`–`T016`, US4 Git PR Packager)**: Depends on `AuditReceipt` (`T005`) and `InoculationGenerator` (`T010`).
- **Phase 6 (`T017`–`T022`, US2 MCP Server)**: Exposes Gate 1/2/3, Inoculation, Trace, and Doctor over `stdio` and `SSE/HTTP`.
- **Phase 7 (`T023`–`T024`, US5 Copilot)** & **Phase 8 (`T025`–`T027`, US6 CI/CD Reporters)**: Can proceed in parallel once core types are in place.
- **Phase 9 (`T028`–`T030`)**: Final integration/adversarial verification, convergence check, and release packaging.

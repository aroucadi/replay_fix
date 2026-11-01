# Data Model: Spec 150 (Release 1.5.0)

**Feature**: Permanent Inoculation, Agentic Protocol (MCP) & Continuous CI/CD Automation  
**Date**: 2026-11-01

---

## 1. Inoculation Entities (`crates/replayfix-core/src/inoculate.rs`)

### `InoculationSpec`
- `bug_id: String` — Canonical defect identifier (e.g., `"RF-01"`).
- `spec_rel_path: String` — Relative path in repository (`"tests/repro/rf_01.spec.ts"`).
- `target_path: String` — Repaired source file path (e.g., `"src/cart-summary.js"`).
- `grounded_selector: String` — Verified Playwright accessibility locator (`getByRole`, `getByText`, `getByTestId`).
- `expected_output_text: String` — Postcondition text assertion.
- `typescript_content: String` — Complete synthesized `@playwright/test` TypeScript source.
- `spec_sha256: String` — SHA-256 hex digest of `typescript_content`.

### `InoculationEntry` & `InoculationRegistry` (`.replayfix/inoculations.json`)
- `schema_version: String` (`"1.5"`)
- `updated_at: String` (RFC 3339 UTC)
- `registry_sha256: String` — Tamper-evident SHA-256 over canonical JSON of `entries`.
- `entries: Vec<InoculationEntry>`:
  - `bug_id: String`
  - `spec_path: String`
  - `spec_sha256: String`
  - `patch_sha256: String`
  - `contract_sha256: String`
  - `grounded_selector: String`
  - `execution_time_ms: u64`
  - `created_at: String`

---

## 2. Stress Quorum Entities (`crates/replayfix-core/src/stress.rs`)

### `PerturbationProfile`
- `latency_ms: u64` — Simulated network roundtrip delay (e.g., `200` ms for 3G / slow API).
- `jitter_ms: u64` — Simulated timing perturbation window (up to `500` ms).
- `throttle_profile: String` — `"3G-slow-api"`.

### `StressPassResult`
- `pass_index: u32` — `1..=3` for concurrency quorum, `0` for perturbation pass.
- `interval_delay_ms: u64` — Jittered launch interval in milliseconds.
- `verdict: GateVerdict` — `Green`, `Red`, or `Invalid`.
- `duration_ms: u64`
- `deterministic_match: bool`

### `StressQuorumReport`
- `perturbation_profile: PerturbationProfile`
- `perturbation_pass: StressPassResult`
- `concurrency_passes: Vec<StressPassResult>` (3 passes)
- `quorum_passed: bool`
- `flaky_detected: bool`
- `total_duration_ms: u64`

---

## 3. MCP Protocol Entities (`crates/replayfix-cli/src/mcp/`)

### `JsonRpcRequest` & `JsonRpcResponse`
- `jsonrpc: String` (`"2.0"`)
- `id: Option<serde_json::Value>`
- `method: String` (`"initialize"`, `"tools/list"`, `"tools/call"`, `"resources/list"`, `"resources/read"`, `"prompts/list"`, `"prompts/get"`, `"ping"`)
- `params: Option<serde_json::Value>`
- `result: Option<serde_json::Value>`
- `error: Option<JsonRpcError>` (`code: i64`, `message: String`, `data: Option<Value>`)

### `McpToolDefinition`
- `name: String` (`replayfix_reproduce`, `replayfix_verify_patch`, `replayfix_inoculate`, `replayfix_inspect_trace`, `replayfix_doctor`)
- `description: String`
- `input_schema: serde_json::Value`

---

## 4. Git Packaging & PR Entities (`crates/replayfix-cli/src/git_packager.rs`)

### `CommitTrailers`
- `verified: bool` (`ReplayFix-Verified: true`)
- `substrate: String` (`ReplayFix-Substrate: <substrate>`)
- `merkle_root: String` (`ReplayFix-Merkle-Root: <hex>`)
- `signature: String` (`ReplayFix-Signature: <hex>`)
- `gate1_status: String` (`ReplayFix-Gate1-Status: RED (Exit Code 1)`)
- `gate2_status: String` (`ReplayFix-Gate2-Status: GREEN (2-Run Stability Pass)`)

### `GitPrPackage`
- `branch_name: String` (`replayfix/fix-<bug_id>`)
- `commit_sha: Option<String>`
- `commit_message: String`
- `trailers: CommitTrailers`
- `pr_markdown_path: PathBuf`
- `inoculation_spec_path: Option<PathBuf>`

---

## 5. CI/CD Reporter Entities (`crates/replayfix-core/src/reporters.rs`)

### `SarifLog` (SARIF v2.1.0)
- `version: String` (`"2.1.0"`)
- `schema: String` (`"https://json.schemastore.org/sarif-2.1.0.json"`)
- `runs: Vec<SarifRun>` (tool driver `ReplayFix`, rules, results with physical locations and Merkle proof properties).

### `JunitReport`
- `<testsuites>` / `<testsuite>` / `<testcase>` XML elements recording Gate 1, Gate 2, Gate 3, and Sweep bug timings and failure messages.

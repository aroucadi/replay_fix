# Quickstart Validation Guide: Spec 150 (Release 1.5.0)

**Feature**: Permanent Inoculation, Agentic Protocol (MCP) & Continuous CI/CD Automation

---

## 1. Permanent Inoculation ("Dreaming")
```powershell
cargo run -- inoculate --bug RF-01 --output .replayfix
```
- **Expected Outcome**:
  - Generates `tests/repro/rf_01.spec.ts` with resilient accessibility locators (`getByRole`, `getByTestId`).
  - Updates `.replayfix/inoculations.json` matching `contracts/inoculation.json`.

## 2. MCP Universal Server (JSON-RPC 2.0)
```powershell
# Launch over stdio
cargo run -- mcp --transport stdio
```
- **Expected Outcome**:
  - Responds to `initialize`, `tools/list`, `tools/call` (`replayfix_reproduce`, `replayfix_verify_patch`, `replayfix_inoculate`, `replayfix_inspect_trace`, `replayfix_doctor`), `resources/list`, `resources/read`, and `prompts/get` (`triage_bug`).

## 3. Gate 3 Stress Quorum & Flakiness Quarantine
```powershell
# Deterministic pass under Gate 3 stress quorum
cargo run -- repro --bug RF-01 --mock-heal --stress-quorum

# Flaky patch quarantine (exits with code 21 FLAKY_NON_DETERMINISTIC)
cargo run -- repro --bug RF-01 --mock-heal --stress-quorum --mock-flaky
```

## 4. Autonomous Git Packaging & PR Engine
```powershell
cargo run -- repro --bug RF-01 --mock-heal --package-pr --output-dir .replayfix
```
- **Expected Outcome**:
  - Creates branch `replayfix/fix-rf-01` (or packaged commit artifact) with signed `ReplayFix-Merkle-Root` and `ReplayFix-Signature` trailers.
  - Emits `.replayfix/pr_description.md` containing the Evidence Matrix.

## 5. CI/CD SARIF & JUnit Reporters
```powershell
cargo run -- repro --bug RF-01 --mock-heal --sarif .replayfix/results.sarif --junit .replayfix/junit.xml
```
- **Expected Outcome**:
  - Writes valid SARIF v2.1.0 and JUnit XML reports.

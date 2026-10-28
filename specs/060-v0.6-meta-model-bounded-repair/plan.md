# Implementation Plan: 060-v0.6-meta-model-bounded-repair — Meta Muse Bounded Repair, Deliberation Replay & Anti-Cheating Enforcement

**Branch**: `release/v0.6.0-model-repair` | **Date**: 2026-10-28 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/060-v0.6-meta-model-bounded-repair/spec.md`

---

## Summary

Add `crates/replayfix-core/src/repair.rs` and integrate bounded autonomous model repair into `crates/replayfix-core/src/engine.rs`, `crates/replayfix-core/src/receipt.rs`, and `crates/replayfix-cli/src/main.rs`:

1. **Anti-Cheating Test & Config Freeze (`TestFreezeGuard`)**: Captures SHA-256 digests of all protected verification files (`tests/**`, `*.spec.*`, `*.test.*`, `playwright.config.*`, `jest.config.*`, `vitest.config.*`, `tsconfig*.json`, `package.json`) and the canonical `BehavioralContract` before repair begins. Rejects any patch targeting a protected path across case variants, path traversal, or alternate separators with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED")` (exit code `11`), and re-verifies SHA-256 parity after every patch.
2. **Meta Muse Routing & Prompt-Injection Fence (`MuseModelConfig`, `PromptBuilder`)**: Validates model slugs (`muse-spark-1.3` with `effort: "xhigh"`, `muse-wonder-1.0` with `effort: "high"`, `muse-spark-1.3-contributor` with `effort: "xhigh"`, and fallback `meta-models/muse-spark-open-1026`), rejecting `muse-wonder-full-1.0` and `effort: "max"`. Places trusted system instructions in the cached prefix (`prompt_cache_key: "replayfix-v0.6-harness-prefix-v1"`, `prompt_cache_retention: "24h"`) and wraps all repository source code, stack traces, and compiler diagnostics inside escaped `<UNTRUSTED_CONTEXT>` tags.
3. **Encrypted Deliberation Replay (`reasoning.encrypted_content` + `previous_response_id`)**: Requests `include: ["reasoning.encrypted_content"]` on Attempt 1 and threads `previous_response_id` and `encrypted_content` into Attempts 2 and 3.
4. **Strict JSON Schema Decoding & Ambiguity-Safe Source-Block Patching (`StructuredRepairPatch`, `BlockPatcher`)**: Validates the strict 5-field JSON Schema (`explanation`, `target_file`, `search_block`, `replace_block`, `touched_symbols`, `additionalProperties: false`), verifies that `search_block` occurs **exactly once** in `target_file` (`PATCH_TARGET_NOT_FOUND` on 0 matches, `AMBIGUOUS_PATCH` on $\ge 2$ matches), verifies symbol presence, and synthesizes a unified diff for the receipt.
5. **Preflight Syntax Gate, Bounded Retry Loop & 2-Run Gate 2 Stability (`SyntaxPreflight`, `BoundedRepairOrchestrator`)**: Runs `node --check <target_file>` via `run_bounded` inside `ScratchSandbox` before browser execution, retries up to `MAX_REPAIR_ATTEMPTS = 3` within `180s`, requires **2 consecutive GREEN runs** at Gate 2 (`gate2_runs_completed: 2`), and terminates with `TriageStatus::QuarantineRepairFailed` (`QUARANTINE_REPAIR_FAILED`, exit code `21`) if all 3 attempts fail.
6. **Empirical Token & Cost Accounting (`TokenAccounting`)**: Computes integer micro-dollar cost (`total_cost_micros_usd`) from verified model rate cards without double-counting cached prompt tokens, and emits `repair_telemetry` and `token_accounting` in `AuditReceipt` and `TriageReport` (schema `"0.6"`).

---

## Technical Context

**Language/Version**: Rust 1.99.1 (Edition 2021) + Node.js 24 (`node --check`) + Playwright 1.64+
**Primary Dependencies**: Existing workspace crates only (`serde`, `serde_json`, `thiserror`, `regex`, `chrono`, `sha2`, `clap`, `tokio`, `tracing-subscriber`). Zero new external crates added.
**Storage**: Ephemeral scratch directories in `std::env::temp_dir()`; JSON receipts in `--output-dir` (default `.replayfix/`).
**Testing**: `cargo test --workspace` — unit tests in `repair.rs`, `patch.rs`, `engine.rs`, `receipt.rs` and CLI integration tests in `crates/replayfix-cli/tests/cli.rs`.
**Target Platform**: Windows (MSVC), macOS, Linux.
**Performance Goals**: `< 20_000` micro-dollars (`< $0.02`) per repaired bug on `muse-spark-1.3`; `node --check` preflight `< 500ms`.
**Constraints**: `MAX_REPAIR_ATTEMPTS = 3`, `MAX_REPAIR_WALL_CLOCK = 180s`, zero `.unwrap()` / `.expect()` in production code, 100% integer fields in receipt structures for RFC 8785 canonical JSON compatibility.

---

## Constitution Check & Invariant Matrix

| Invariant / Law | Source | Affected Modules | Verification Mechanism | Status |
| :--- | :--- | :--- | :--- | :---: |
| **Invariant A: Substrate Primacy & Typed Errors** | Law I, Law X | `error.rs`, `repair.rs`, `engine.rs` | `ModelApiError`, `PatchApplicationError`, `RepairBudgetExceeded`, `SyntaxValidationError`; `.specify/scripts/check-panics.ps1` (`0` violations) | **PASS** |
| **Invariant B: Anti-Cheating Test Freeze** | Law I, Law V, Op Inv 9 | `repair.rs` (`TestFreezeGuard`) | Pre/post SHA-256 freeze + case-folded path guard rejecting tests/configs with `SecurityViolation("TEST_MUTATION_PROHIBITED")` (exit `11`) | **PASS** |
| **Invariant C: Model Routing & Truthful Attribution** | Law VII, Spec 060 §1 | `repair.rs` (`MuseModelConfig`) | Permits `muse-spark-1.3` (`xhigh`), `muse-wonder-1.0` (`high`), `muse-spark-1.3-contributor` (`xhigh`); rejects `muse-wonder-full-1.0` and `effort: "max"` | **PASS** |
| **Invariant D: Prompt-Injection Fence** | Spec 060 §2.1 | `repair.rs` (`PromptBuilder`) | Static prefix first (`replayfix-v0.6-harness-prefix-v1`, `24h`); untrusted data escaped inside `<UNTRUSTED_CONTEXT>` | **PASS** |
| **Invariant E: Encrypted Deliberation Continuity** | Spec 060 §2.2 | `repair.rs` (`RepairRequest`) | Requests `include: ["reasoning.encrypted_content"]` on Attempt 1; chains `previous_response_id` + `encrypted_content` on Attempts 2–3 | **PASS** |
| **Invariant F: Strict JSON Schema & Block Patcher** | Spec 060 §2.3 | `repair.rs` (`StructuredRepairPatch`, `BlockPatcher`) | Strict 5-field JSON validation (`additionalProperties: false`); exact single-match block replacement (`PATCH_TARGET_NOT_FOUND` / `AMBIGUOUS_PATCH`) | **PASS** |
| **Invariant G: Bounded Retries, Syntax Preflight & 2x Gate 2** | Law IV, Op Inv 8 & 9 | `repair.rs`, `engine.rs` | `node --check` via `run_bounded` before browser; max 3 attempts / 180s; 2x Gate 2 stability pass; exit `21` (`QUARANTINE_REPAIR_FAILED`) on exhaustion | **PASS** |
| **Invariant H: Token & Cost Accounting** | Law VII, Spec 060 §3 | `repair.rs`, `receipt.rs` | Integer `prompt_tokens`, `cached_prompt_tokens`, `completion_tokens`, `reasoning_tokens`, `latency_ms`, `total_cost_micros_usd` in schema `"0.6"` | **PASS** |

---

## Project Structure

### Documentation (this feature)

```text
specs/060-v0.6-meta-model-bounded-repair/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── tasks.md
├── checklists/
│   └── requirements.md
└── contracts/
    └── repair_response.schema.json
```

### Source Code (repository root)

```text
crates/replayfix-core/src/
├── lib.rs          # export repair module and new public types
├── error.rs        # + ModelApiError, PatchApplicationError, RepairBudgetExceeded, SyntaxValidationError
├── repair.rs       # NEW: TestFreezeGuard, MuseModelConfig, PromptBuilder, StructuredRepairPatch, BlockPatcher, SyntaxPreflight, TokenAccounting, RepairTelemetry, BoundedRepairEngine
├── engine.rs       # Wire BoundedRepairEngine, 2-run Gate 2 stability pass, and quarantine triage (exit 21)
└── receipt.rs      # Schema "0.6", + QuarantineRepairFailed (exit 21), + repair_telemetry, + token_accounting

crates/replayfix-cli/
├── src/main.rs     # + --model <SLUG>, --mock-failing-repair, --mock-syntax-retry flags; terminal UI steps for Test Freeze, Muse Repair, Syntax Preflight, and 2x Gate 2 Stability
└── tests/cli.rs    # Integration tests for --model muse-spark-1.3, --mock-failing-repair (exit 21), anti-cheat test mutation rejection (exit 11), and v0.5 backward compatibility
```

---

## Complexity Tracking

Zero new external crate dependencies added. `SyntaxPreflight` reuses `replayfix_core::process::run_bounded` (`node --check`), and `BlockPatcher` + `StructuredRepairPatch` use `serde_json::Value` for strict `additionalProperties: false` schema enforcement.

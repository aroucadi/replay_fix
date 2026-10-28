# Data Model: 060-v0.6-meta-model-bounded-repair

**Feature**: `060-v0.6-meta-model-bounded-repair`  
**Date**: 2026-10-28

---

## 1. `replayfix_core::error::CoreError` Additions (Invariant A)

| Variant | Description | Maps to `TriageStatus` / Exit Code |
| :--- | :--- | :--- |
| `ModelApiError(String)` | Model routing, HTTP/provider, or JSON schema validation error | `QuarantineRepairFailed` (`21`) or `UsageError` (`2`) on invalid config |
| `PatchApplicationError(String)` | `PATCH_TARGET_NOT_FOUND` or `AMBIGUOUS_PATCH` during source-block patching | `PatchApplyFailed` (`30`) or retried up to `QuarantineRepairFailed` (`21`) |
| `RepairBudgetExceeded(String)` | Exceeded `MAX_REPAIR_ATTEMPTS = 3` or `180_000 ms` wall-clock deadline | `QuarantineRepairFailed` (`21`) |
| `SyntaxValidationError(String)` | `node --check` preflight syntax failure in `ScratchSandbox` | Retried up to `QuarantineRepairFailed` (`21`) |

---

## 2. `replayfix_core::repair` Entities

### `MuseModelSlug` & `ReasoningEffort` (Invariant C)
| Model Slug | Permitted Effort | Primary Role | Rate Card (Uncached In / Cached In / Out per 1M tokens) |
| :--- | :--- | :--- | :--- |
| `muse-spark-1.3` | `"xhigh"` (`"max"` forbidden) | Fast compiler & single-file repair loop | `$1.25` / `$0.15` / `$4.25` |
| `muse-wonder-1.0` | `"high"` | Architectural root-cause diagnosis | `$4.00` / `$0.20` / `$20.00` |
| `muse-spark-1.3-contributor` | `"xhigh"` (`"max"` forbidden) | Economic contributor tier | `$0.10` / `$0.002` / `$0.20` |
| `meta-models/muse-spark-open-1026` | `"xhigh"` | Baseten fallback on HTTP 429 / 5xx | `$1.25` / `$0.15` / `$4.25` |

### `TestFreezeGuard` (Invariant B)
| Field | Type | Description |
| :--- | :--- | :--- |
| `frozen_files` | `BTreeMap<String, String>` | Map of normalized relative path -> SHA-256 hex digest of protected files |
| `contract_sha256` | `String` | Frozen SHA-256 digest of the `BehavioralContract` |

### `StructuredRepairPatch` (Invariant F)
| Field | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `explanation` | `String` | Non-empty | Concise root-cause and fix explanation |
| `target_file` | `String` | Safe non-protected relative path | Relative path to source file in repository |
| `search_block` | `String` | Non-empty, occurs exactly once in `target_file` | Exact source lines to replace |
| `replace_block` | `String` | Non-empty, `!= search_block` | Replacement source lines |
| `touched_symbols` | `Vec<String>` | Non-empty array of non-empty strings | Identifiers modified by the patch |

### `AttemptLineage` (Invariants E & G)
| Field | Type | Description |
| :--- | :--- | :--- |
| `attempt` | `u32` | `1..=3` |
| `requested_model` | `String` | Model slug requested by caller |
| `serving_model` | `String` | Actual model slug that served the response |
| `response_id` | `String` | Provider response ID (`resp_...`) |
| `previous_response_id` | `Option<String>` | Prior attempt's `response_id` (`Some` on attempts 2 and 3) |
| `encrypted_reasoning_preserved` | `bool` | True when `reasoning.encrypted_content` was captured/threaded |
| `syntax_preflight_passed` | `bool` | True when `node --check` exited `0` |
| `outcome` | `String` | `"SYNTAX_FAILED"`, `"PATCH_FAILED"`, `"GATE2_RED"`, or `"VERIFIED_GREEN"` |
| `diagnostic` | `Option<String>` | Bounded error feedback passed to next attempt |

### `TokenAccounting` (Invariant H)
| Field | Type | Constraints | Description |
| :--- | :--- | :--- | :--- |
| `prompt_tokens` | `u64` | Total input tokens (includes cached) | Cumulative prompt tokens across attempts |
| `cached_prompt_tokens` | `u64` | `<= prompt_tokens` | Cumulative cached prefix tokens |
| `completion_tokens` | `u64` | Total output tokens | Cumulative completion tokens |
| `reasoning_tokens` | `u64` | Deliberation tokens | Cumulative reasoning tokens |
| `latency_ms` | `u64` | Wall-clock ms | Cumulative model invocation latency in ms |
| `total_cost_micros_usd` | `u64` | Integer micro-dollars (`1 USD = 1_000_000`) | Calculated cost from rate card (`< 20_000` for sub-`$0.02` target) |

### `RepairTelemetry`
| Field | Type | Description |
| :--- | :--- | :--- |
| `requested_model` | `String` | Requested model slug |
| `serving_model` | `String` | Actual serving model slug |
| `provider_mode` | `String` | `"live-meta-api"`, `"live-baseten-fallback"`, or `"deterministic-offline-fixture"` |
| `effort` | `String` | `"xhigh"` or `"high"` |
| `prompt_cache_key` | `String` | `"replayfix-v0.6-harness-prefix-v1"` |
| `untrusted_context_fenced` | `bool` | True when `<UNTRUSTED_CONTEXT>` isolation was enforced |
| `test_immutability_verified` | `bool` | True when `TestFreezeGuard` pre/post check passed |
| `syntax_preflight_passed` | `bool` | True when `node --check` passed on the winning patch |
| `gate2_runs_completed` | `u32` | Number of consecutive Gate 2 verification runs (`2` on pass) |
| `gate2_stability_passed` | `bool` | True when all `gate2_runs_completed` runs passed (`GREEN`) |
| `attempts_used` | `u32` | `1..=3` |
| `lineage` | `Vec<AttemptLineage>` | Per-attempt deliberation and verification lineage |
| `token_accounting` | `TokenAccounting` | Cumulative token usage and micro-dollar cost |

---

## 3. `replayfix_core::receipt` Updates (Schema `"0.6"`)

- **`SCHEMA_VERSION`**: Updated to `"0.6"`.
- **`TriageStatus::QuarantineRepairFailed`**: Serializes as `"QUARANTINE_REPAIR_FAILED"`, exit code `21`.
- **`AuditReceipt`**: Adds optional `repair_telemetry: Option<RepairTelemetry>` and `token_accounting: Option<TokenAccounting>`.
- **`TriageReport`**: Adds optional `repair_telemetry: Option<RepairTelemetry>` and `token_accounting: Option<TokenAccounting>` so quarantined runs preserve full diagnostic and cost attribution.

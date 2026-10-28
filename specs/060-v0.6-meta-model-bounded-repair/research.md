# Research & Technical Decisions: 060-v0.6-meta-model-bounded-repair

**Feature**: `060-v0.6-meta-model-bounded-repair`  
**Date**: 2026-10-28

---

## Decision 1: Ambiguity-Safe Source-Block Patching (`BlockPatcher`) Along With Unified Diff Synthesis

- **Decision**: Model repair responses return a strict JSON object (`explanation`, `target_file`, `search_block`, `replace_block`, `touched_symbols`). `BlockPatcher::apply` validates `target_file` against both `TaskManifest::assert_safe_path` and `TestFreezeGuard::validate_target_path`, normalizes CRLF/LF line endings consistently with `target_file`, counts exact occurrences of `search_block` in `target_file`, and:
  - Fails closed with `CoreError::PatchApplicationError("PATCH_TARGET_NOT_FOUND: ...")` if `occurrences == 0`.
  - Fails closed with `CoreError::PatchApplicationError("AMBIGUOUS_PATCH: ...")` if `occurrences >= 2`.
  - Rejects no-op replacements (`search_block == replace_block`) with `TriageStatus::RepairNoopRejected`.
  - Verifies that every symbol listed in `touched_symbols` is non-empty and appears in either `search_block` or `replace_block`.
  - Synthesizes a canonical unified diff (`--- a/<file>`, `+++ b/<file>`, `@@ ... @@`) from the applied block replacement for audit transparency in `AuditReceipt`.
- **Rationale**:
  - LLMs frequently hallucinate line numbers in raw unified diffs. Exact `search_block` / `replace_block` matching with a strict uniqueness check (`occurrences == 1`) eliminates line-number drift while never guessing when a block appears multiple times.
  - Note on honesty: Per our engineering contract, we describe `BlockPatcher` accurately as **ambiguity-safe exact source-block patching with symbol presence verification**, never falsely claiming a full compiler AST parser when performing exact block replacement.

---

## Decision 2: Anti-Cheating Test & Config Freeze (`TestFreezeGuard`)

- **Decision**: Before repair begins, `TestFreezeGuard::capture` walks the `ScratchSandbox` and records SHA-256 digests of all protected verification files (`tests/**`, `*.spec.*`, `*.test.*`, `playwright.config.*`, `jest.config.*`, `vitest.config.*`, `tsconfig*.json`, `package.json`) plus the canonical `BehavioralContract` digest. Path validation (`TestFreezeGuard::validate_target_path`):
  1. Rejects unsafe paths (`..`, UNC, DOS device names, ADS colons, leading `/` or drive prefixes) via `TaskManifest::assert_safe_path`.
  2. Normalizes separators (`\` -> `/`), collapses `.` segments, and case-folds to lowercase ASCII (`to_ascii_lowercase()`) so Windows case variations (`TESTS/Foo.SPEC.ts`, `Package.JSON`, `TSConfig.build.json`) are caught identically on all platforms.
  3. Rejects any path inside `tests/`, `__tests__/`, `test/`, `e2e/`, `specs/`, or ending with `.spec.js`, `.spec.ts`, `.spec.tsx`, `.spec.mjs`, `.test.js`, `.test.ts`, `.test.tsx`, `.test.mjs`, or matching `playwright.config.*`, `jest.config.*`, `vitest.config.*`, `tsconfig*.json`, or `package.json` with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED: ...")`.
  4. Re-verifies SHA-256 parity of all frozen files after every patch application and before Gate 2 (`TestFreezeGuard::verify_integrity`), failing closed with `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED: ...")` if any protected file was modified, deleted, or added.
- **Rationale**:
  - Satisfies Constitutional Law I, Law V, and Operational Invariant 9 across both pre-patch target validation and post-patch disk integrity checks (preventing TOCTOU or symlink-based bypasses).

---

## Decision 3: Integer Micro-Dollar Token & Cost Accounting (`TokenAccounting`)

- **Decision**: Store all token counts (`prompt_tokens`, `cached_prompt_tokens`, `completion_tokens`, `reasoning_tokens`, `latency_ms`) as `u64` integers and compute monetary cost in **integer micro-dollars** (`1 USD = 1_000_000 micros`, i.e., `1_000_000` micros per dollar, so `$1.25 / 1M tokens` = `1.25 micro-dollars per token`, or in nano-dollars / integer tenths of a micro-dollar rounded deterministically) so that `AuditReceipt` and `TriageReport` remain 100% free of IEEE-754 floats (preserving RFC 8785 `canonical_json` compatibility).
  - Verified Rate Card:
    - `muse-spark-1.3`: `$1.25 / 1M` uncached input tokens, `$0.15 / 1M` cached input tokens, `$4.25 / 1M` output tokens (where `uncached_prompt_tokens = prompt_tokens.saturating_sub(cached_prompt_tokens)` so cached tokens are never double-counted, and `billable_output_tokens = completion_tokens` with `reasoning_tokens` tracked separately and only added if not already included in `completion_tokens`).
    - `muse-wonder-1.0`: `$4.00 / 1M` uncached input, `$0.20 / 1M` cached input, `$20.00 / 1M` output.
    - `muse-spark-1.3-contributor`: `$0.10 / 1M` uncached input, `$0.002 / 1M` cached input, `$0.20 / 1M` output.
- **Rationale**:
  - Integer `cost_nanos_usd` / `cost_micros_usd` avoids floating-point serialization errors in `canonical_json` while providing exact sub-cent precision for the `< $0.02` (`< 20_000` micro-dollars) per-bug budget verification.

---

## Decision 4: Offline Deterministic Provider & Live HTTP Provider Honesty (Law VII)

- **Decision**: Implement a clean provider abstraction in `replayfix_core::repair` supporting:
  1. Live HTTP `POST https://api.meta.ai/v1/responses` (via bounded `curl` subprocess through `run_bounded` when `META_API_KEY` is set and live network mode is requested, avoiding heavy TLS/async HTTP crate bloat) with automatic fallback to `meta-models/muse-spark-open-1026` on HTTP 429/5xx.
  2. Deterministic local model fixtures (`RepairClientMode::DeterministicOffline`, `DeterministicSyntaxRetry`, `DeterministicFailing`) for reproducible CI, unit tests, and offline verification without external credentials.
  - Every `RepairTelemetry` record explicitly sets `provider_mode` (`"live-meta-api"`, `"live-baseten-fallback"`, or `"deterministic-offline-fixture"`), `requested_model`, and `serving_model` so offline runs never masquerade as live cloud inference.

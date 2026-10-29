# ADR-05: Model Pooling and Acceptance Policy

## Status
Accepted

## Context
When three models concurrently propose patches, ReplayFix must decide how to evaluate and select the winning candidate repair.

## Decision
1. **Substrate Primacy as Absolute Authority (Law I)**:
   - LLMs are treated as untrusted patch proposers.
   - A model's self-assessment, confidence score, or mutual agreement with another model NEVER constitutes qualification.
   - The ONLY authority that can qualify a patch is the deterministic two-gate verification harness (`Gate1 RED -> Apply Patch -> Gate2 GREEN x2 + Regression Parity + DiffGuard`).

2. **First-Verified-Green Selection (Competitive Race)**:
   - As models return candidates, each patch is passed through syntax preflight (`node --check`) and submitted to sandbox verification.
   - The first candidate that passes both Gate 2 stability runs, preserves regression parity, and satisfies DiffGuard is declared the winner.
   - Pending tasks for other models on that bug are cancelled immediately, conserving token budget and compute.

3. **Disagreement Handling**:
   - If model A and model B propose patch 1, and model C proposes patch 2: both patches are evaluated through the deterministic gates.
   - If both patch 1 and patch 2 pass verification, the one with fewer lines of code changed (minimal AST edit) or first to complete is selected.
   - If neither passes, the bug transitions to `QUARANTINE_REPAIR_FAILED`.

4. **Negative Guarantees**:
   - Majority voting is rejected as an acceptance criterion (two hallucinating models agreeing on an invalid patch must still fail closed).
   - Patches that modify protected test files or configuration are unconditionally rejected with exit code 11 (`TEST_MUTATION_PROHIBITED`).

## Consequences
- **Positive**: Zero risk of model collusion; guarantees that only provably correct patches are accepted.

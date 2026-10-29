# ADR-04: Tri-Model Orchestration & Concurrency Boundaries

## Status
Accepted

## Context
ReplayFix v0.6 introduced single-model autonomous repair with sequential retry. In v0.9.0, multiple model candidates (`muse-spark-1.3`, `muse-wonder-1.0`, `muse-spark-1.3-contributor` / fallback) must be coordinated to generate candidate repairs, targeting sub-60-second completion across a 5-bug benchmark sweep.

## Decision
1. **Bounded Concurrency Model**:
   - Model requests are dispatched concurrently using Tokio tasks bounded by a concurrency semaphore (`tokio::sync::Semaphore`).
   - Concurrency limit default is 3 (one per model provider candidate).
   - Task cancellation is propagated via `tokio_util::sync::CancellationToken` or `tokio::select!` cancellation scopes.

2. **Timeout Propagation & Resource Caps**:
   - Per-attempt model timeout: 15 seconds.
   - Sweep timeout: 60 seconds wall-clock budget.
   - Dropping or cancelling a future must abort in-flight network requests or subprocesses cleanly.

3. **Candidate Isolation**:
   - Each model candidate runs in a dedicated ephemeral scratch directory or memory buffer.
   - Candidate outputs are strictly segregated by `(bug_id, model_name, attempt_id)`.
   - No model candidate can mutate another model's scratch state or the base repository.

4. **Resource Accounting**:
   - Aggregate token usage and micro-dollar accounting are accumulated across all three models and reported in the receipt.

## Consequences
- **Positive**: Lowers sweep latency by executing model synthesis in parallel; bounded tasks avoid runaway memory or subprocess leakage.
- **Fail-Closed**: If all models fail, timeout, or produce invalid AST patches, the harness records an unfixable defect triage report and fails closed.

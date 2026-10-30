# ADR-05: Rate Pooling, Quota Accounting, and Circuit Breakers

## Status
Accepted

## Context
Multi-model repair and 5-bug benchmark sweeps can generate bursts of concurrent model requests. Quotas may be per-model or shared across an organization account, and upstream endpoints can return HTTP 429 or 5xx errors.

## Decision
1. **Bounded Token-Bucket Rate Pool (`RatePool`)**:
   - Tracks both Requests Per Minute (`rpm_limit`, default configurable per pool) and Tokens Per Minute (`tpm_limit`), plus `max_in_flight` concurrent requests (default: `3`).
   - Supports both per-model buckets and shared organization-level quota pools so quota is never blindly multiplied across model slugs without authorization.
2. **HTTP 429 & Retry Policy**:
   - Honors explicit `Retry-After` seconds/milliseconds when returned by the provider.
   - Otherwise computes bounded exponential backoff with deterministic jitter up to `max_retries` (default: `2`).
   - Authentication errors (`401`/`403`) and malformed requests (`400`) fail immediately without retry.
3. **Three-State Circuit Breaker (`CircuitBreaker`)**:
   - `Closed`: Normal operation; increments `consecutive_failures` on timeout, connection error, or HTTP 5xx/429 exhaustion.
   - `Open`: Trips when `consecutive_failures >= failure_threshold` (default: `3`). Rejects primary calls immediately or routes eligible text tasks to `BasetenFallback` (`meta-models/muse-spark-open`) until `cooldown_duration` (default: `30s`) elapses.
   - `HalfOpen`: Permits a single probe request after cooldown; transitions to `Closed` on success or re-opens on failure.

## Consequences
- **Positive**: Prevents retry storms, enforces strict concurrency and token budgets, and provides measurable state transitions under fault injection.

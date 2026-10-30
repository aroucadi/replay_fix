# Contract: Rate Limiting & Circuit Breaker Policy

## 1. Rate Limiting Parameters
- **Requests Per Minute (RPM)**: Configurable ceiling (default: `9,000` RPM across shared organization quotas, or clamped to environment configuration).
- **Tokens Per Minute (TPM)**: Configurable ceiling (default: `12,000,000` TPM).
- **In-Flight Concurrency**: Capped by `max_concurrent_tasks` (default: 3).
- **HTTP 429 Handling**:
  - Parse `Retry-After` header if present.
  - If omitted, apply exponential backoff with full jitter: $T_{\text{wait}} = \min(T_{\max}, T_{\text{base}} \times 2^{\text{attempt}}) + \text{jitter}$.

## 2. Circuit Breaker State Machine
- **States**:
  - `Closed`: Normal traffic flows to primary provider. Consecutive failure count is tracked.
  - `Open`: Tripped after 3 consecutive connection or server failures (HTTP 5xx, timeouts). All traffic immediately routes to fallback or rejects fail-closed without network stalls. Cooldown: 30 seconds.
  - `Half-Open`: After cooldown, a single canary request tests upstream health. On success $\to$ `Closed`; on failure $\to$ `Open` with reset cooldown.

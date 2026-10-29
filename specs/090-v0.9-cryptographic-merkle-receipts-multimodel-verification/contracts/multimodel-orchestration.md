# Contract: Bounded Tri-Model Orchestration & Pooling

## 1. Allowed Model Pool
The authorized models for concurrent candidate generation are:
1. `muse-spark-1.3` (Primary lightweight fast-response candidate)
2. `muse-wonder-1.0` (Reasoning-heavy candidate for complex AST patches)
3. `muse-spark-1.3-contributor` (Alternative candidate with contributor persona)
4. Offline fallback test doubles for deterministic testing without external API requirements.

---

## 2. Competitive Race Protocol

```mermaid
sequenceDiagram
    participant O as Orchestrator
    participant M1 as muse-spark-1.3
    participant M2 as muse-wonder-1.0
    participant M3 as muse-spark-1.3-contributor
    participant V as Verifier (Two-Gate)

    O->>M1: Request Candidate (Timeout 15s)
    O->>M2: Request Candidate (Timeout 15s)
    O->>M3: Request Candidate (Timeout 15s)

    M1-->>O: Patch Candidate 1
    O->>V: Evaluate Patch 1
    alt Patch 1 Green (2x consecutive + Regression Parity)
        V-->>O: Qualified (VERIFIED_REPAIRED)
        O->>M2: Cancel Task
        O->>M3: Cancel Task
        O->>O: Commit Winning Patch
    else Patch 1 Red
        M2-->>O: Patch Candidate 2
        O->>V: Evaluate Patch 2
    end
```

---

## 3. Mandatory Limits & Accounting
- Max attempts per bug: 3 attempts.
- Max concurrent model tasks: 3 tasks.
- Per-task timeout: 15 seconds.
- Memory/scratch isolation: Each model writes patches to isolated in-memory buffers; disk application occurs strictly inside `ScratchSandbox`.

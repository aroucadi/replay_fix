# Research & Mathematical Foundations: Spec 070 Differential Diagnosis & Cryptographic Anti-Cheat

## 1. Causal Differential Diagnosis & State Machine

In autonomous repair systems, a candidate patch can pass a target test for spurious reasons:
1. The test was flawed or non-assertive, passing even without the fix.
2. The environment or test runner crashed, masking the test failure.
3. The defect was already absent in the repository clean reference.
4. The patch muted, skipped, or deleted the test.
5. The test is non-deterministic (flaky).

To achieve mathematical causal attribution, ReplayFix models verification across three distinct execution baselines:
- $B_{\text{unpatched}}$: The repository worktree in its unpatched, buggy state.
- $B_{\text{clean}}$: The clean reference worktree (e.g. `main` or known healthy commit $\text{SHA}_{\text{clean}}$).
- $B_{\text{patched}}$: The ephemeral scratch clone containing the candidate repair patch.

### Causal Truth Table

Let $R(W)$ represent the boolean evaluation of the reproduction test on worktree $W$ (where `true` = PASS, `false` = FAIL):

| Baseline $R(B_{\text{unpatched}})$ | Clean $R(B_{\text{clean}})$ | Patched Run 1 $R_1(B_{\text{patched}})$ | Patched Run 2 $R_2(B_{\text{patched}})$ | Regression Parity | FSM State Classification | Exit Code | Action |
| :---: | :---: | :---: | :---: | :---: | :--- | :---: | :--- |
| `false` (RED) | `true` (GREEN) | `true` (GREEN) | `true` (GREEN) | `true` (PASS) | `CAUSAL_DEFECT_CONFIRMED` | `0` | Issue Audit Receipt |
| `true` (GREEN) | *any* | *any* | *any* | *any* | `CLEAN_PASS_UNREPRODUCED` | `10` | Fail Closed (Gate 1 not RED) |
| `false` (RED) | `false` (RED) | *any* | *any* | *any* | `QUARANTINE_INVERTED_BASELINE` | `32` | Clean baseline already broken |
| `false` (RED) | `true` (GREEN) | `true` (GREEN) | `false` (RED) | *any* | `FLAKY_NON_DETERMINISTIC` | `21` | Reject flaky repair |
| `false` (RED) | `true` (GREEN) | `false` (RED) | *any* | *any* | `QUARANTINE_REPAIR_FAILED` | `21` | Repair failed Gate 2 |
| `false` (RED) | `true` (GREEN) | `true` (GREEN) | `true` (GREEN) | `false` (FAIL) | `REGRESSION_DETECTED` | `31` | Patch broke existing tests |

If the reproduction test runner crashes or fails to execute cleanly due to infrastructure errors rather than assertion failures, the state is categorized as `UNATTRIBUTED_SYMPTOM`.

---

## 2. Cryptographic Anti-Cheat & Surgical Configuration Immutability

### Protected File Set
Protected paths are defined as:
$$\mathcal{P} = \{ p \mid p \in \text{glob}(\text{"tests/**"}) \cup \text{glob}(\text{"**/__tests__/**"}) \cup \text{glob}(\text{"*.spec.*"}) \cup \text{glob}(\text{"*.test.*"}) \cup \text{Configs} \}$$
where $\text{Configs} = \{ \text{playwright.config.*}, \text{jest.config.*}, \text{vitest.config.*}, \text{tsconfig*.json} \}$.

### Surgical `package.json` Test Script Immutability
In web projects, models frequently attempt to bypass tests by tampering with npm test scripts, e.g. changing `"test": "playwright test"` to `"test": "exit 0"`.
However, legitimate patches may require updating dependencies or non-test scripts (e.g. build flags).

Rule:
For any patch modifying `package.json`:
1. Parse the pre-patch AST / JSON and extract all key-value pairs under `"scripts"` where the key matches `test*`:
   $$\mathcal{S}_{\text{pre}} = \{ (k, v) \mid k \in \text{scripts}, k \text{ starts with "test"} \}$$
2. Parse the post-patch AST / JSON and extract the same mapping:
   $$\mathcal{S}_{\text{post}} = \{ (k, v) \mid k \in \text{scripts}, k \text{ starts with "test"} \}$$
3. Invariant:
   $$\mathcal{S}_{\text{pre}} = \mathcal{S}_{\text{post}}$$
   If $\mathcal{S}_{\text{pre}} \neq \mathcal{S}_{\text{post}}$, the patch is immediately rejected with:
   `CoreError::SecurityViolation("TEST_MUTATION_PROHIBITED: package.json test scripts altered")`

---

## 3. Strict Regression Parity Math

Let the suite of all baseline tests be $T_{\text{baseline}}$, with failing tests $F_{\text{baseline}} \subseteq T_{\text{baseline}}$ and skipped tests $S_{\text{baseline}} \subseteq T_{\text{baseline}}$.
Let the suite of patched tests be $T_{\text{patched}}$, with failing tests $F_{\text{patched}}$ and skipped tests $S_{\text{patched}}$.

Parity is satisfied if and only if all three conditions hold:
1. **Zero New Failures**:
   $$F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$$
2. **Zero Test Deletions**:
   $$T_{\text{baseline}} \setminus T_{\text{patched}} = \emptyset$$
3. **Zero Silent Skips**:
   $$|S_{\text{patched}}| \le |S_{\text{baseline}}|$$

Any violation marks `regression_parity_passed = false` and triggers exit code `31`.

---

## 4. Durable Checkpointing & Stale-State Defense

To support resumable execution across stages:
1. Checkpoint file: `.replayfix/session_state.json`.
2. Stale-state rejection invariants:
   - Git HEAD check: `session_state.base_repo_head == git rev-parse HEAD`.
   - Manifest integrity check: `session_state.manifest_sha256 == sha256(task_manifest.json)`.
   - TTL ceiling: $\text{CurrentEpoch} - \text{created\_at\_epoch} \le \text{ttl\_seconds}$ (default 3600 seconds).
3. If any check fails, the checkpoint is discarded as invalid/stale, and execution restarts fail-closed from `INITIALIZED`.

# Quickstart: Spec 070 Differential Diagnosis & Cryptographic Anti-Cheat

## Common Workflows

### 1. Freeze Repro Spec with Anti-Cheat Hash
```bash
replayfix freeze-spec --bug RF-01 --output .replayfix/specs/RF-01.spec.json
```
Output:
```text
[freeze-spec] Frozen reproduction specification for RF-01.
[freeze-spec] Digest: a1b2c3d4...
[freeze-spec] Protected files: 12 test/config paths locked with SHA-256 baseline.
```

### 2. Run Autonomous Repair with Clean Baseline Attribution
```bash
replayfix run \
  --repo-path ./my-app \
  --clean-ref main \
  --bug-id RF-01 \
  --output-dir .replayfix
```
What ReplayFix executes:
1. **Gate 1 Baseline**: Validates defect on unpatched repo (must be RED).
2. **Clean Reference Check**: Validates reproduction test on `main` (must NOT fail).
3. **Model Repair**: Proposes search/replace patch under strict prompt fencing.
4. **Anti-Cheat Verification**: Verifies zero test mutations and zero `package.json` test script alterations.
5. **Gate 2 Stability Pass**: Runs repro test 2 consecutive times on patched scratch clone (must be GREEN both times).
6. **Regression Parity**: Executes existing test suite, ensuring $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$.
7. **Audit Receipt Generation**: Issues RFC 8785 signed audit receipt schema `0.7` and writes disaggregated artifacts to `.replayfix/artifacts/`.

### 3. Check State & Matrix Attribution
```bash
replayfix run --verify-matrix --clean-ref main --bug-id RF-01
```
Output:
```text
Differential Matrix Result:
  State:               CAUSAL_DEFECT_CONFIRMED
  Clean Ref:           main (3a4f89b...)
  Baseline Result:     FAIL (Exit 1)
  Clean Ref Result:    PASS (Exit 0)
  Patched Run 1:       PASS (Exit 0)
  Patched Run 2:       PASS (Exit 0)
  Regression Parity:   PASS (0 new failures, 0 dropped tests)
  Causal Confidence:   HIGH
```

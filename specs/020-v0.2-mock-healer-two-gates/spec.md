# Feature Specification: 020-v0.2-mock-healer-two-gates — Dual-Gate State Machine & Mock Healer

**Feature Branch**: `release/v0.2.0-mock-healer`
**Created**: 2026-10-26
**Status**: Ready for Planning
**Target Release**: `v0.2.0`
**Input**: The v0.2 slice of the release plan: a dual-gate state machine (`Init` → `Gate1Red` → `Repair` → `Gate2Green`) that proves the closed loop end to end with a hardcoded fix, an in-memory unified-diff patcher, an anti-no-op guard, and a machine-readable receipt.

---

## Scope Statement (read first)

v0.2 proves the **control flow**, not the **perception**. There is no browser, no Playwright run, no video analysis and no model call in this release. Gates are evaluated by a **simulated oracle** that inspects a built-in fixture (`RF-01`). Every receipt says so explicitly. Real browser-backed gates arrive with Spec 040 (v0.4); isolated scratch repositories arrive with Spec 030 (v0.3).

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Two-Gate Closed Loop (Priority: P1)

As an engineer evaluating ReplayFix, I want `replayfix repro --mock-heal` to run the complete Gate 1 → Repair → Gate 2 loop against a known defect, so that I can see the state machine, contract invariance and receipt format working before real execution backends exist.

**Why this priority**: The state machine is the spine every later release plugs into.

**Independent Test**: Run `replayfix repro --task-id RF-01 --bug-ref bug/RF-01 --video recordings/RF-01.mp4 --mock-heal`. The command exits `0` and writes `.replayfix/audit_receipt.json` with status `VERIFIED_REPAIRED`.

**Acceptance Scenarios**:
1. **Given** the built-in `RF-01` fixture, **When** Gate 1 runs, **Then** the oracle event stream is `SETUP pass → TARGET pass → ACTION pass → ASSERT fail` and the machine moves to `Gate1Red`.
2. **Given** Gate 1 is RED, **When** the built-in patch is applied in memory, **Then** the machine moves to `Repair` and the patch statistics (files, insertions, deletions) are recorded.
3. **Given** a repaired fixture, **When** Gate 2 runs the same contract, **Then** all four events pass, the machine moves to `Gate2Green`, and a receipt with Gate 1 and Gate 2 timestamps is emitted.
4. **Given** Gate 1 and Gate 2, **When** both evaluate the contract, **Then** their canonical contract digests are identical; a mismatch fails closed.

---

### User Story 2 - Unified Diff Patcher (Priority: P2)

As a developer wiring later repair stages, I want a dependency-free, in-memory unified-diff applier, so that candidate patches can be validated deterministically without touching the filesystem.

**Independent Test**: `cargo test -p replayfix-core -- patch`.

**Acceptance Scenarios**:
1. **Given** a well-formed single-file diff whose context matches exactly, **When** applied, **Then** the patched text and insertion/deletion counts are returned.
2. **Given** a diff whose context or removed lines do not match, **When** applied, **Then** it fails with a descriptive patch error (no fuzzy matching).
3. **Given** a diff with a malformed hunk header, mismatched hunk line counts, overlapping hunks, multiple files, or a file-creation header, **When** applied, **Then** it is rejected.
4. **Given** a diff header path that is unsafe (traversal, UNC, DOS device, ADS), **When** applied, **Then** it fails with `SECURITY_VIOLATION`.

---

### User Story 3 - Anti-No-Op Guard (Priority: P3)

As a maintainer, I want empty, whitespace-only and net-zero patches rejected, so that a repair can never be "verified" by doing nothing.

**Independent Test**: `replayfix repro ... --mock-heal --mock-patch noop` exits `30`, writes `.replayfix/triage_report.json` with status `REPAIR_NOOP_REJECTED`, and leaves no `audit_receipt.json` behind.

**Acceptance Scenarios**:
1. **Given** an empty or whitespace-only patch, **When** the loop reaches `Repair`, **Then** it is rejected with `REPAIR_NOOP_REJECTED`.
2. **Given** a patch that applies cleanly but leaves the source byte-identical, **When** evaluated, **Then** it is rejected with `REPAIR_NOOP_REJECTED`.
3. **Given** a patch that changes the source but does not fix the defect, **When** Gate 2 runs, **Then** Gate 2 stays RED and the run is rejected with `GATE2_NOT_GREEN`.
4. **Given** a patch that removes the target function, **When** Gate 2 runs, **Then** TARGET fails and the result is never classified green (non-crash invariant, Law VI).
5. **Given** a previous successful run left `audit_receipt.json` in the output directory, **When** a later run is rejected, **Then** the stale receipt is removed.

---

### Edge Cases

- **Unknown task id**: `--mock-heal` supports only `RF-01`; any other task exits `2` with a clear message naming the supported fixture.
- **Custom patch file**: `--mock-patch <file>` reads a diff from disk (path-validated, 1 MiB limit); an unreadable or oversized file exits `2`.
- **Unsafe output directory**: `--output-dir ../escape` is rejected with `SECURITY_VIOLATION` (exit `11`) before any write.
- **Flags without `--mock-heal`**: `--mock-patch` and `--output-dir` require `--mock-heal`; clap rejects them otherwise.
- **CRLF input**: diffs and sources with CRLF line endings are compared line-wise without silent corruption.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST model the run as a finite state machine with phases `Init`, `Gate1Red`, `Repair`, `Gate2Green` and a terminal `Rejected`; any other transition MUST fail closed.
- **FR-002**: Gate 1 MUST establish RED before any patch is considered; a Gate 1 that is not RED MUST terminate the run (`GATE1_NOT_RED`).
- **FR-003**: Gate 1 and Gate 2 MUST evaluate a `BehavioralContract` whose canonical digest (sorted-key compact JSON, SHA-256) is identical; a mismatch MUST terminate with `CONTRACT_MISMATCH`.
- **FR-004**: An oracle result MUST be classified RED only when SETUP, TARGET and ACTION passed before a failed ASSERT; any earlier failure MUST be classified invalid and never RED or GREEN.
- **FR-005**: `replayfix_core::patch::UnifiedPatcher::apply_diff` MUST apply a single-file unified diff in memory with exact context matching, validating hunk counts and ordering.
- **FR-006**: Patch header paths MUST pass the shared path-safety validation used by manifests.
- **FR-007**: Empty, whitespace-only and net-zero patches MUST be rejected as `REPAIR_NOOP_REJECTED`.
- **FR-008**: On success the system MUST write `audit_receipt.json` containing status, task id, contract digest, Gate 1 and Gate 2 timestamps and event streams, patch digest and statistics, the phase trace and the simulated-oracle disclaimer.
- **FR-009**: On any rejection the system MUST write `triage_report.json` (status, exit code, reason, phase trace) and MUST remove any stale `audit_receipt.json` from the same directory.
- **FR-010**: `replayfix repro` MUST accept `--mock-heal`, `--mock-patch <builtin|noop|FILE>` and `--output-dir <DIR>`; exit codes: `0` verified, `10` Gate 1/contract failure, `11` security violation, `30` repair rejected or Gate 2 not green, `2` usage/input error.
- **FR-011**: Existing `doctor` and `repro` v0.1 behaviour MUST remain unchanged when `--mock-heal` is absent, except for the corrected ingestion status line.
- **FR-012**: The CLI version string MUST derive from the crate version (no hardcoded duplicate).

### Key Entities

- **Phase / PhaseMachine**: The four-phase state machine plus terminal `Rejected`, recording the visited trace.
- **BehavioralContract**: Immutable description of setup, target, action and postcondition identifier, hashed canonically.
- **OracleEvent / GateVerdict**: Ordered `SETUP`/`TARGET`/`ACTION`/`ASSERT` events and their `Red`/`Green`/`Invalid` classification.
- **UnifiedPatcher / PatchOutcome**: In-memory applier and its result (patched text, statistics, target path).
- **AuditReceipt / TriageReport**: Machine-readable terminal outputs (success / failure).

---

## Success Criteria *(mandatory)*

- **SC-001**: `replayfix repro --mock-heal` completes in under 500 ms on standard hardware.
- **SC-002**: 100% of the adversarial cases listed in User Story 3 are rejected, each with its documented status and exit code.
- **SC-003**: The patcher has unit tests for every rejection class in User Story 2 and passes with zero fuzzy matches.
- **SC-004**: Every run leaves exactly one terminal artifact (`audit_receipt.json` xor `triage_report.json`) in the output directory.
- **SC-005**: Workspace builds, `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test --workspace` pass with zero warnings.

---

## Assumptions

- The simulated oracle is a stand-in: it statically inspects the fixture for the unguarded `cart.items.length` defect signature. It makes no claim about real browser behaviour.
- Unified diffs target one file; multi-file patches are out of scope until the repair stage (v0.6).
- Rejecting patches that touch test files, and the two-run stability pass, belong to Spec 070 and are not part of this release.
- The base repository is not read or modified by `--mock-heal` (no repository access occurs in v0.2).

# Data Model: 020-v0.2-mock-healer-two-gates

## Phase (enum)
`Init` → `Gate1Red` → `Repair` → `Gate2Green`; any non-terminal phase may move to `Rejected`. `Gate2Green` and `Rejected` are terminal.

## BehavioralContract
| Field | Type | Notes |
| :--- | :--- | :--- |
| `contract_id` | string | e.g. `RF-01-cart-count` |
| `task_id` | string | must equal the manifest task id |
| `setup` | string | scenario precondition |
| `target` | string | element/function under test |
| `action` | string | interaction performed |
| `postcondition_id` | string | assertion identifier |

Digest = SHA-256 of canonical JSON (hex, lowercase).

## OracleEvent
`{ seq: u32, kind: SETUP|TARGET|ACTION|ASSERT, passed: bool }`. Classification: `Red` (first three pass, ASSERT fails), `Green` (all pass), `Invalid` (any of SETUP/TARGET/ACTION fails).

## PatchOutcome
`{ target_path, patched, insertions, deletions, files_changed }`; `is_noop(original)` is true when `patched == original`.

## AuditReceipt (`audit_receipt.json`)
`schema_version`, `status` (`VERIFIED_REPAIRED`), `task_id`, `substrate` (`mock-static-oracle`), `disclaimer`, `contract_id`, `contract_sha256`, `gate1 {observed_at, verdict, events}`, `patch {target_path, sha256, insertions, deletions, files_changed}`, `gate2 {observed_at, verdict, events, contract_sha256}`, `phase_trace`, `created_at`.

## TriageReport (`triage_report.json`)
`schema_version`, `status` (`REPAIR_NOOP_REJECTED` | `PATCH_APPLY_FAILED` | `GATE1_NOT_RED` | `GATE2_NOT_GREEN` | `CONTRACT_MISMATCH` | `SECURITY_VIOLATION`), `exit_code`, `reason`, `task_id`, `substrate`, `phase_trace`, `created_at`.

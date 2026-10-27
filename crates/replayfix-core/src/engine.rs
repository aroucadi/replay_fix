//! Dual-gate engine: Gate 1 (RED) -> Repair -> Gate 2 (GREEN), driven by a simulated oracle.
//!
//! The oracle statically inspects a fixture for the `cart.items.length`
//! defect signature. It stands in for browser execution until real gates exist.

use crate::digest::{canonical_json, sha256_hex};
use crate::error::{CoreError, Result};
use crate::fixtures::Fixture;
use crate::fsm::{Phase, PhaseMachine};
use crate::patch::UnifiedPatcher;
use crate::receipt::{
    timestamp_now, AuditReceipt, GateRecord, PatchRecord, TriageReport, TriageStatus, DISCLAIMER,
    SCHEMA_VERSION, SUBSTRATE,
};
use serde::{Deserialize, Serialize};

/// Immutable description of what both gates must evaluate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BehavioralContract {
    pub contract_id: String,
    pub task_id: String,
    pub setup: String,
    pub target: String,
    pub action: String,
    pub postcondition_id: String,
}

impl BehavioralContract {
    /// SHA-256 over the canonical (key-sorted, compact) JSON form.
    pub fn digest(&self) -> Result<String> {
        let value = serde_json::to_value(self)?;
        Ok(sha256_hex(canonical_json(&value)?.as_bytes()))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OracleEventKind {
    Setup,
    Target,
    Action,
    Assert,
}

const EVENT_ORDER: [OracleEventKind; 4] = [
    OracleEventKind::Setup,
    OracleEventKind::Target,
    OracleEventKind::Action,
    OracleEventKind::Assert,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct OracleEvent {
    pub seq: u32,
    pub kind: OracleEventKind,
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateVerdict {
    Red,
    Green,
    Invalid,
}

/// Run the simulated oracle. Execution stops at the first failing event,
/// exactly as a real run could not assert after a failed setup or action.
pub fn run_oracle(contract: &BehavioralContract, source: &str) -> Vec<OracleEvent> {
    let target_marker = format!("function {}", contract.target);
    let checks: [bool; 4] = [
        !source.trim().is_empty(),
        source.contains(&target_marker),
        source.contains(&target_marker),
        !source.contains("cart.items.length"),
    ];

    let mut events = Vec::with_capacity(4);
    for (seq, (kind, passed)) in EVENT_ORDER.iter().zip(checks).enumerate() {
        events.push(OracleEvent {
            seq: seq as u32,
            kind: *kind,
            passed,
        });
        if !passed {
            break;
        }
    }
    events
}

/// Classify an event stream (Law VI): RED needs SETUP, TARGET and ACTION to
/// pass before a failed ASSERT. Anything incomplete or out of order is INVALID.
pub fn classify(events: &[OracleEvent]) -> GateVerdict {
    let in_order = events
        .iter()
        .enumerate()
        .all(|(i, e)| e.seq as usize == i && EVENT_ORDER.get(i) == Some(&e.kind));
    if !in_order || events.len() < EVENT_ORDER.len() {
        return GateVerdict::Invalid;
    }
    let (pre, last) = events.split_at(EVENT_ORDER.len() - 1);
    if !pre.iter().all(|e| e.passed) {
        return GateVerdict::Invalid;
    }
    if last[0].passed {
        GateVerdict::Green
    } else {
        GateVerdict::Red
    }
}

fn describe_invalid(events: &[OracleEvent]) -> String {
    match events.iter().find(|e| !e.passed) {
        Some(e) => format!(
            "oracle stopped at {:?} before asserting; this is not a valid verdict",
            e.kind
        ),
        None => "oracle event stream was incomplete or out of order".to_string(),
    }
}

/// Terminal result of a mock-heal run.
#[derive(Debug, Clone)]
pub enum HealOutcome {
    Verified(Box<AuditReceipt>),
    Rejected(Box<TriageReport>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MockHealer;

impl MockHealer {
    /// Run the full loop against `fixture` using `diff` as the candidate repair.
    pub fn run(fixture: &Fixture, diff: &str) -> Result<HealOutcome> {
        Self::run_with_gate2_contract(fixture, diff, &fixture.contract)
    }

    /// Like [`run`](Self::run) but lets Gate 2 evaluate a different contract.
    /// Production code never does this; it exists to prove the invariance guard fires.
    pub fn run_with_gate2_contract(
        fixture: &Fixture,
        diff: &str,
        gate2_contract: &BehavioralContract,
    ) -> Result<HealOutcome> {
        let task_id = fixture.task_id;
        let mut fsm = PhaseMachine::new();

        macro_rules! reject {
            ($status:expr, $reason:expr) => {{
                fsm.advance(Phase::Rejected)?;
                return Ok(HealOutcome::Rejected(Box::new(TriageReport::new(
                    $status,
                    $reason,
                    task_id,
                    fsm.trace(),
                ))));
            }};
        }

        // Gate 1: the defect must reproduce before any repair is considered.
        let contract_sha256 = fixture.contract.digest()?;
        let gate1_events = run_oracle(&fixture.contract, fixture.buggy_source);
        let gate1_verdict = classify(&gate1_events);
        if gate1_verdict != GateVerdict::Red {
            reject!(
                TriageStatus::Gate1NotRed,
                format!(
                    "Gate 1 must be RED before repair (got {:?}): {}",
                    gate1_verdict,
                    describe_invalid(&gate1_events)
                )
            );
        }
        fsm.advance(Phase::Gate1Red)?;
        let gate1_at = timestamp_now();

        // Repair: validate and apply the candidate patch in memory.
        fsm.advance(Phase::Repair)?;
        if diff.trim().is_empty() {
            reject!(
                TriageStatus::RepairNoopRejected,
                "The candidate patch is empty; a repair must change the source"
            );
        }
        let outcome = match UnifiedPatcher::apply_diff(fixture.buggy_source, diff) {
            Ok(o) => o,
            Err(CoreError::SecurityViolation(msg)) => {
                reject!(TriageStatus::SecurityViolation, msg)
            }
            Err(e) => reject!(TriageStatus::PatchApplyFailed, e.to_string()),
        };
        if outcome.target_path != fixture.target_path {
            reject!(
                TriageStatus::PatchApplyFailed,
                format!(
                    "Patch targets '{}' but the repair scope is '{}'",
                    outcome.target_path, fixture.target_path
                )
            );
        }
        if outcome.is_noop(fixture.buggy_source) {
            reject!(
                TriageStatus::RepairNoopRejected,
                "The patch applies cleanly but leaves the source unchanged"
            );
        }

        // Gate 2: same contract, patched source.
        let gate2_sha256 = gate2_contract.digest()?;
        if gate2_sha256 != contract_sha256 {
            reject!(
                TriageStatus::ContractMismatch,
                "Gate 2 contract digest differs from Gate 1; refusing to compare results"
            );
        }
        let gate2_events = run_oracle(gate2_contract, &outcome.patched);
        let gate2_verdict = classify(&gate2_events);
        match gate2_verdict {
            GateVerdict::Green => {}
            GateVerdict::Red => reject!(
                TriageStatus::Gate2NotGreen,
                "The defect still reproduces after the patch"
            ),
            GateVerdict::Invalid => reject!(
                TriageStatus::Gate2NotGreen,
                format!(
                    "Gate 2 produced no valid verdict: {}",
                    describe_invalid(&gate2_events)
                )
            ),
        }
        fsm.advance(Phase::Gate2Green)?;
        let gate2_at = timestamp_now();

        Ok(HealOutcome::Verified(Box::new(AuditReceipt {
            schema_version: SCHEMA_VERSION.to_string(),
            status: "VERIFIED_REPAIRED".to_string(),
            task_id: task_id.to_string(),
            substrate: SUBSTRATE.to_string(),
            disclaimer: DISCLAIMER.to_string(),
            contract_id: fixture.contract.contract_id.clone(),
            contract_sha256: contract_sha256.clone(),
            gate1: GateRecord {
                observed_at: gate1_at,
                verdict: gate1_verdict,
                contract_sha256,
                events: gate1_events,
            },
            patch: PatchRecord {
                target_path: outcome.target_path.clone(),
                sha256: sha256_hex(diff.as_bytes()),
                insertions: outcome.insertions,
                deletions: outcome.deletions,
                files_changed: outcome.files_changed,
            },
            gate2: GateRecord {
                observed_at: gate2_at,
                verdict: gate2_verdict,
                contract_sha256: gate2_sha256,
                events: gate2_events,
            },
            phase_trace: fsm.trace().to_vec(),
            created_at: timestamp_now(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rf01() -> Fixture {
        Fixture::for_task("RF-01").unwrap()
    }

    fn rejected(outcome: HealOutcome) -> TriageReport {
        match outcome {
            HealOutcome::Rejected(r) => *r,
            HealOutcome::Verified(_) => panic!("expected rejection"),
        }
    }

    #[test]
    fn test_gate1_oracle_is_red_on_buggy_source() {
        let f = rf01();
        let events = run_oracle(&f.contract, f.buggy_source);
        assert_eq!(events.len(), 4);
        assert_eq!(
            events.iter().map(|e| e.passed).collect::<Vec<_>>(),
            [true, true, true, false]
        );
        assert_eq!(classify(&events), GateVerdict::Red);
    }

    #[test]
    fn test_happy_path_is_verified_with_ordered_phases() {
        let f = rf01();
        let HealOutcome::Verified(receipt) = MockHealer::run(&f, f.builtin_diff).unwrap() else {
            panic!("expected verification");
        };
        assert_eq!(receipt.status, "VERIFIED_REPAIRED");
        assert_eq!(receipt.gate1.verdict, GateVerdict::Red);
        assert_eq!(receipt.gate2.verdict, GateVerdict::Green);
        assert_eq!(
            receipt.phase_trace,
            [
                Phase::Init,
                Phase::Gate1Red,
                Phase::Repair,
                Phase::Gate2Green
            ]
        );
        assert_eq!(receipt.gate1.contract_sha256, receipt.gate2.contract_sha256);
        assert_eq!(receipt.contract_sha256.len(), 64);
        assert!(receipt.gate1.observed_at <= receipt.gate2.observed_at);
        assert_eq!(receipt.patch.sha256, sha256_hex(f.builtin_diff.as_bytes()));
        assert_eq!((receipt.patch.insertions, receipt.patch.deletions), (1, 1));
        assert_eq!(receipt.gate2.events.len(), 4);
        assert!(receipt.gate2.events.iter().all(|e| e.passed));
    }

    #[test]
    fn test_empty_and_whitespace_patches_are_rejected_as_noop() {
        let f = rf01();
        for diff in ["", "   \n\t\n"] {
            let t = rejected(MockHealer::run(&f, diff).unwrap());
            assert_eq!(t.status, TriageStatus::RepairNoopRejected);
            assert_eq!(t.exit_code, 30);
            assert_eq!(
                t.phase_trace,
                [Phase::Init, Phase::Gate1Red, Phase::Repair, Phase::Rejected]
            );
        }
    }

    #[test]
    fn test_net_zero_patch_is_rejected_as_noop() {
        let f = rf01();
        let diff = "--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -2 +2 @@\n-  return cart.items.length;\n+  return cart.items.length;\n";
        let t = rejected(MockHealer::run(&f, diff).unwrap());
        assert_eq!(t.status, TriageStatus::RepairNoopRejected);
    }

    #[test]
    fn test_patch_that_does_not_fix_the_defect_leaves_gate2_red() {
        let f = rf01();
        let diff = "--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -2 +2 @@\n-  return cart.items.length;\n+  return cart.items.length + 0;\n";
        let t = rejected(MockHealer::run(&f, diff).unwrap());
        assert_eq!(t.status, TriageStatus::Gate2NotGreen);
        assert!(t.reason.contains("still reproduces"));
    }

    #[test]
    fn test_patch_deleting_the_target_is_never_green() {
        let f = rf01();
        let diff = "--- a/src/cart-summary.js\n+++ b/src/cart-summary.js\n@@ -1,3 +1 @@\n-export function cartItemCount(cart) {\n-  return cart.items.length;\n-}\n+// removed\n";
        let t = rejected(MockHealer::run(&f, diff).unwrap());
        assert_eq!(t.status, TriageStatus::Gate2NotGreen);
        assert!(t.reason.contains("no valid verdict"));
    }

    #[test]
    fn test_malformed_patch_fails_closed() {
        let f = rf01();
        let t = rejected(MockHealer::run(&f, "not a diff").unwrap());
        assert_eq!(t.status, TriageStatus::PatchApplyFailed);
        assert_eq!(t.exit_code, 30);
    }

    #[test]
    fn test_unsafe_patch_path_is_a_security_violation() {
        let f = rf01();
        let diff = "--- a/../../etc/passwd\n+++ b/../../etc/passwd\n@@ -1 +1 @@\n-x\n+y\n";
        let t = rejected(MockHealer::run(&f, diff).unwrap());
        assert_eq!(t.status, TriageStatus::SecurityViolation);
        assert_eq!(t.exit_code, 11);
    }

    #[test]
    fn test_patch_for_another_file_is_rejected() {
        let f = rf01();
        let diff = "--- a/src/other.js\n+++ b/src/other.js\n@@ -1 +1 @@\n-x\n+y\n";
        let t = rejected(MockHealer::run(&f, diff).unwrap());
        assert_eq!(t.status, TriageStatus::PatchApplyFailed);
    }

    #[test]
    fn test_contract_mismatch_blocks_gate2() {
        let f = rf01();
        let mut other = f.contract.clone();
        other.postcondition_id = "something-weaker".to_string();
        let t = rejected(MockHealer::run_with_gate2_contract(&f, f.builtin_diff, &other).unwrap());
        assert_eq!(t.status, TriageStatus::ContractMismatch);
        assert_eq!(t.exit_code, 10);
    }

    #[test]
    fn test_gate1_not_red_terminates_the_run() {
        let mut f = rf01();
        f.buggy_source =
            "export function cartItemCount(cart) {\n  return cart.items?.length || 0;\n}\n";
        let t = rejected(MockHealer::run(&f, f.builtin_diff).unwrap());
        assert_eq!(t.status, TriageStatus::Gate1NotRed);
        assert_eq!(t.phase_trace, [Phase::Init, Phase::Rejected]);
    }

    #[test]
    fn test_crash_before_assert_is_invalid_not_red() {
        let mut f = rf01();
        f.buggy_source = "// nothing here\n";
        let events = run_oracle(&f.contract, f.buggy_source);
        assert_eq!(classify(&events), GateVerdict::Invalid);
        let t = rejected(MockHealer::run(&f, f.builtin_diff).unwrap());
        assert_eq!(t.status, TriageStatus::Gate1NotRed);
    }

    #[test]
    fn test_classify_rejects_out_of_order_events() {
        let shuffled = [
            OracleEvent {
                seq: 0,
                kind: OracleEventKind::Target,
                passed: true,
            },
            OracleEvent {
                seq: 1,
                kind: OracleEventKind::Setup,
                passed: true,
            },
            OracleEvent {
                seq: 2,
                kind: OracleEventKind::Action,
                passed: true,
            },
            OracleEvent {
                seq: 3,
                kind: OracleEventKind::Assert,
                passed: false,
            },
        ];
        assert_eq!(classify(&shuffled), GateVerdict::Invalid);
        assert_eq!(classify(&[]), GateVerdict::Invalid);
    }

    #[test]
    fn test_contract_digest_is_stable_and_sensitive() {
        let f = rf01();
        let d1 = f.contract.digest().unwrap();
        assert_eq!(d1, f.contract.clone().digest().unwrap());
        let mut changed = f.contract.clone();
        changed.action.push('!');
        assert_ne!(d1, changed.digest().unwrap());
    }
}

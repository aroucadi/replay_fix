pub mod digest;
pub mod doctor;
pub mod engine;
pub mod error;
pub mod fixtures;
pub mod fsm;
pub mod manifest;
pub mod patch;
pub mod platform;
pub mod receipt;

pub use digest::{canonical_json, sha256_hex};
pub use doctor::{DependencyCheck, DependencyStatus, Doctor, DoctorReport};
pub use engine::{
    BehavioralContract, GateVerdict, HealOutcome, MockHealer, OracleEvent, OracleEventKind,
};
pub use error::{CoreError, Result};
pub use fixtures::Fixture;
pub use fsm::{Phase, PhaseMachine};
pub use manifest::{EnvironmentAdapter, TaskManifest};
pub use patch::{PatchOutcome, UnifiedPatcher};
pub use platform::{OsFamily, Platform};
pub use receipt::{AuditReceipt, TriageReport};

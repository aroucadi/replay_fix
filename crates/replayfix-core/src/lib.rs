#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unreachable
    )
)]

pub mod artifacts;
pub mod browser;
pub mod checkpoint;
pub mod crypto;
pub mod diffguard;
pub mod digest;
pub mod doctor;
pub mod engine;
pub mod error;
pub mod fixtures;
pub mod fsm;
pub mod ingestion;
pub mod locator;
pub mod manifest;
pub mod merkle;
pub mod patch;
pub mod perception;
pub mod platform;
pub mod process;
pub mod receipt;
pub mod regression;
pub mod repair;
pub mod sweep;
pub mod trace;
pub mod video;
pub mod worktree;

pub use artifacts::ArtifactStore;
pub use browser::{BridgeCommand, BridgeResponse, PlaywrightBridge, SandboxServer};
pub use checkpoint::{
    CheckpointManager, CheckpointStage, SessionStateCheckpoint, StageArtifacts, CHECKPOINT_VERSION,
    DEFAULT_CHECKPOINT_TTL_SECONDS,
};
pub use crypto::{
    build_signing_payload, verify_attestation, AttestationRecord, KeypairManager, TrustPolicy,
    VerificationOutcome,
};
pub use diffguard::DiffGuard;
pub use digest::{canonical_json, sha256_hex};
pub use doctor::{DependencyCheck, DependencyStatus, Doctor, DoctorReport};
pub use engine::{
    BehavioralContract, GateVerdict, HealOutcome, LiveBrowserOptions, MockHealer, OracleEvent,
    OracleEventKind,
};
pub use error::{CoreError, Result};
pub use fixtures::Fixture;
pub use fsm::{
    CausalConfidence, DifferentialDiagnosisReport, DifferentialMatrix, DifferentialState, Phase,
    PhaseMachine, ReproExecutionResult,
};
pub use ingestion::{
    build_untrusted_context_block, extract_repro_steps, ingest_issue, ingestion_to_manifest,
    GitHubIssuePayload, GitHubIssueRef, GitHubLabel, GitHubUser, IngestionResult, IssueFetcher,
    MockIssueFetcher, SystemIssueFetcher, MAX_ISSUE_PAYLOAD_BYTES,
};
pub use locator::{BoundingBox, DomCandidate, GroundedTarget, GroundingTelemetry, LocatorResolver};
pub use manifest::{EnvironmentAdapter, TaskManifest};
pub use merkle::{EvidenceLeaf, MerkleInclusionProof, MerkleReceipt, MerkleTree};
pub use patch::{PatchOutcome, UnifiedPatcher};
pub use perception::{
    AsrRequestConfig, AsrTranscriber, NarrationCue, SamContourBox, SamSegmenter, SamSnapResult,
    VideoTelemetry, ASR_MODEL_SLUG, SAM_MODEL_SLUG,
};
pub use platform::{EphemeralPort, OsFamily, Platform};
pub use receipt::{
    AuditReceipt, TriageReport, TriageStatus, AUDIT_RECEIPT_FILE, TRIAGE_REPORT_FILE,
};
pub use regression::{RegressionEvaluator, RegressionParityReport, TestStatus, TestSuiteSnapshot};
pub use repair::{
    AttemptLineage, BlockPatcher, BoundedRepairClient, ModelCandidateOutcome, ModelPool,
    ModelPoolConfig, MuseModelConfig, MuseModelSlug, PromptBuilder, ReasoningEffort,
    RepairClientMode, RepairModelResponse, RepairRequest, RepairTelemetry, StructuredRepairPatch,
    SyntaxPreflight, TestFreezeGuard, TokenAccounting, DISALLOWED_MUSE_WONDER_FULL_SLUG,
    MAX_REPAIR_ATTEMPTS, MAX_REPAIR_WALL_CLOCK, MUSE_SPARK_13_CONTRIBUTOR_SLUG, MUSE_SPARK_13_SLUG,
    MUSE_SPARK_OPEN_FALLBACK_SLUG, MUSE_WONDER_10_SLUG, PROMPT_CACHE_KEY, PROMPT_CACHE_RETENTION,
    REASONING_ENCRYPTED_CONTENT_FIELD,
};
pub use sweep::{run_sweep, SweepBugResult, SweepOptions, SweepReceipt};
pub use trace::{is_ci_environment, launch_trace_viewer, TraceConfig, TraceLaunchOutcome};
pub use video::{
    DetectionParams, FrameSet, GeometricCalibration, InteractionEvidence, RoiDiffer, VideoMetadata,
    VideoPreprocessor,
};
pub use worktree::{FileEntry, ScratchSandbox, WorktreeManifest};

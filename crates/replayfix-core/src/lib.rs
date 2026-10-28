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

pub mod browser;
pub mod digest;
pub mod doctor;
pub mod engine;
pub mod error;
pub mod fixtures;
pub mod fsm;
pub mod locator;
pub mod manifest;
pub mod patch;
pub mod perception;
pub mod platform;
pub mod process;
pub mod receipt;
pub mod repair;
pub mod video;
pub mod worktree;

pub use browser::{BridgeCommand, BridgeResponse, PlaywrightBridge, SandboxServer};
pub use digest::{canonical_json, sha256_hex};
pub use doctor::{DependencyCheck, DependencyStatus, Doctor, DoctorReport};
pub use engine::{
    BehavioralContract, GateVerdict, HealOutcome, LiveBrowserOptions, MockHealer, OracleEvent,
    OracleEventKind,
};
pub use error::{CoreError, Result};
pub use fixtures::Fixture;
pub use fsm::{Phase, PhaseMachine};
pub use locator::{BoundingBox, DomCandidate, GroundedTarget, GroundingTelemetry, LocatorResolver};
pub use manifest::{EnvironmentAdapter, TaskManifest};
pub use patch::{PatchOutcome, UnifiedPatcher};
pub use perception::{
    AsrRequestConfig, AsrTranscriber, NarrationCue, SamContourBox, SamSegmenter, SamSnapResult,
    VideoTelemetry, ASR_MODEL_SLUG, SAM_MODEL_SLUG,
};
pub use platform::{EphemeralPort, OsFamily, Platform};
pub use receipt::{
    AuditReceipt, TriageReport, TriageStatus, AUDIT_RECEIPT_FILE, TRIAGE_REPORT_FILE,
};
pub use repair::{
    AttemptLineage, BlockPatcher, BoundedRepairClient, MuseModelConfig, MuseModelSlug,
    PromptBuilder, ReasoningEffort, RepairClientMode, RepairModelResponse, RepairRequest,
    RepairTelemetry, StructuredRepairPatch, SyntaxPreflight, TestFreezeGuard, TokenAccounting,
    DISALLOWED_MUSE_WONDER_FULL_SLUG, MAX_REPAIR_ATTEMPTS, MAX_REPAIR_WALL_CLOCK,
    MUSE_SPARK_13_CONTRIBUTOR_SLUG, MUSE_SPARK_13_SLUG, MUSE_SPARK_OPEN_FALLBACK_SLUG,
    MUSE_WONDER_10_SLUG, PROMPT_CACHE_KEY, PROMPT_CACHE_RETENTION,
    REASONING_ENCRYPTED_CONTENT_FIELD,
};
pub use video::{
    DetectionParams, FrameSet, GeometricCalibration, InteractionEvidence, RoiDiffer, VideoMetadata,
    VideoPreprocessor,
};
pub use worktree::{FileEntry, ScratchSandbox, WorktreeManifest};

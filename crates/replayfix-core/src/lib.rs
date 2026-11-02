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
pub mod deliberation;
pub mod diffguard;
pub mod digest;
pub mod doctor;
pub mod engine;
pub mod environment;
pub mod error;
pub mod fixtures;
pub mod fsm;
pub mod gateway;
pub mod ingestion;
pub mod inoculate;
pub mod locator;
pub mod manifest;
pub mod merkle;
pub mod observer;
pub mod patch;
pub mod perception;
pub mod platform;
pub mod portal;
pub mod process;
pub mod receipt;
pub mod regression;
pub mod repair;
pub mod reporters;
pub mod stress;
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
pub use deliberation::{
    DeliberationContext, DeliberationEngine, DeliberationOutcome, DeliberationStatus,
    DeliberationToolCall, DeliberationToolObservation, SparkDeliberationSession,
    SparkDeliberationTurn, FRONTIER_WONDER_SLUG, SOVEREIGN_SPARK_SLUG,
    SPARK_AGENT_LOOP_PROMPT_CACHE_KEY, TOOL_INOCULATE, TOOL_INSPECT_TRACE, TOOL_REPRODUCE,
    TOOL_VERIFY_PATCH,
};
pub use diffguard::DiffGuard;
pub use digest::{canonical_json, sha256_hex};
pub use doctor::{DependencyCheck, DependencyStatus, Doctor, DoctorReport};
pub use engine::{
    BehavioralContract, GateVerdict, HealOutcome, LiveBrowserOptions, MockHealer, OracleEvent,
    OracleEventKind,
};
pub use environment::{
    probe_http_ready, AttachedSpaEndpoint, ManagedSpaServer, SpaToolchain,
    DEFAULT_SPA_STARTUP_TIMEOUT,
};
pub use error::{CoreError, Result};
pub use fixtures::{
    Fixture, MetaPulseCaseRecord, MetaPulseManifest, MetaPulseRepoRunner, METAPULSE_REPO_URL,
};
pub use fsm::{
    CausalConfidence, DifferentialDiagnosisReport, DifferentialMatrix, DifferentialState, Phase,
    PhaseMachine, ReproExecutionResult,
};
pub use gateway::{
    align_transcript_with_keyframes, escape_untrusted_dom_context, validate_wav_audio,
    CircuitBreaker, CircuitState, ExecutionMode, GatewayConfig, GatewayProvenance, GatewayProvider,
    GatewayRequest, GatewayResponse, KeyframeAlignment, MetaGatewayClient, ModelRole,
    MultimodalGateway, RatePool, TranscriptSegment, TransportOutcome, WavAudioMetadata,
    BASETEN_FALLBACK_ENDPOINT, BASETEN_MUSE_SPARK_OPEN_SLUG, DEFAULT_ALIGNMENT_TOLERANCE_MS,
    DEFAULT_CIRCUIT_COOLDOWN_MS, DEFAULT_CIRCUIT_FAILURE_THRESHOLD, DEFAULT_GATEWAY_TIMEOUT_MS,
    DEFAULT_MAX_RETRIES, LLAMA_32_11B_VISION_FALLBACK_SLUG, LLAMA_33_70B_FALLBACK_SLUG,
    MAX_AUDIO_DURATION_MS, META_CONNECT_TIMEOUT_SECS, META_READ_TIMEOUT_SECS,
    META_RESPONSES_ENDPOINT,
};
pub use ingestion::{
    build_untrusted_context_block, extract_repro_steps, ingest_issue, ingestion_to_manifest,
    GitHubIssuePayload, GitHubIssueRef, GitHubLabel, GitHubUser, IngestionResult, IssueFetcher,
    MockIssueFetcher, SystemIssueFetcher, MAX_ISSUE_PAYLOAD_BYTES,
};
pub use inoculate::{
    InoculationEntry, InoculationGenerator, InoculationRegistry, InoculationSpec,
    INOCULATION_REGISTRY_FILE, INOCULATION_SCHEMA_VERSION,
};
pub use locator::{
    AmbiguousTieCandidate, BoundingBox, DomCandidate, GroundedTarget, GroundingTelemetry,
    LocatorResolver, AMBIGUITY_TIE_THRESHOLD_BPS,
};
pub use manifest::{EnvironmentAdapter, TaskManifest};
pub use merkle::{EvidenceLeaf, MerkleInclusionProof, MerkleReceipt, MerkleTree};
pub use observer::{
    BroadcastObserver, ChannelObserver, NullObserver, SubstrateEvent, SubstrateObserver,
};
pub use patch::{PatchOutcome, UnifiedPatcher};
pub use perception::{
    ActionTrajectory, ActionType, AsrRequestConfig, AsrTranscriber, NarrationCue, SamContourBox,
    SamSegmenter, SamSnapResult, UserAction, VideoTelemetry, ASR_MODEL_SLUG, SAM_MODEL_SLUG,
};
pub use platform::{EphemeralPort, OsFamily, Platform};
pub use portal::{
    evaluate_portal_receipt_bytes, export_proof_bundle, validate_and_map_sam_box,
    validate_portal_html_contract, PortalDemoPresets, PortalVerificationVerdict,
    ValidatedSpatialOverlay, DEMO_PRESET_PUBLIC_SEED_HEX, MAX_PORTAL_JSON_DEPTH,
    MAX_PORTAL_RECEIPT_BYTES,
};
pub use receipt::{
    AuditReceipt, ModelProvenanceRecord, TriageReport, TriageStatus, AUDIT_RECEIPT_FILE,
    TRIAGE_REPORT_FILE,
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
pub use reporters::{
    JunitReporter, SarifArtifactLocation, SarifDriver, SarifLocation, SarifLog, SarifMessage,
    SarifPhysicalLocation, SarifReporter, SarifResult, SarifRule, SarifRun, SarifTool,
    SARIF_SCHEMA_URI, SARIF_VERSION,
};
pub use stress::{
    PerturbationProfile, StressPassResult, StressQuorumReport, StressQuorumRunner,
    DEFAULT_PERTURBATION_JITTER_MS, DEFAULT_PERTURBATION_LATENCY_MS, GATE3_QUORUM_PASS_COUNT,
};
pub use sweep::{run_sweep, SweepBugResult, SweepOptions, SweepReceipt};
pub use trace::{is_ci_environment, launch_trace_viewer, TraceConfig, TraceLaunchOutcome};
pub use video::{
    DetectionParams, FrameSet, GeometricCalibration, InteractionEvidence, RoiDiffer, VideoMetadata,
    VideoPreprocessor,
};
pub use worktree::{FileEntry, ScratchSandbox, WorktreeManifest};

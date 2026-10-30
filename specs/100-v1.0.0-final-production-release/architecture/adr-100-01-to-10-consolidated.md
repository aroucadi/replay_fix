# ADR-100-01 through ADR-100-10: Consolidated v1.0.0 Stable Release Architecture Decision Records

## ADR-100-01: Stable Product Architecture & Substrate Primacy
- **Context**: ReplayFix v1.0.0 consolidates a 2-crate Rust workspace (`replayfix-core`, `replayfix-cli`) and a single-file offline browser verifier (`proof_viewer.html`).
- **Decision**: Maintain strict separation between the deterministic Rust verification substrate (`replayfix-core`), the CLI presentation layer (`replayfix-cli`), and the air-gapped client-side verification portal (`proof_viewer.html`). No AI model or frontend component can override deterministic Gate 1 / Gate 2 or regression parity verdicts.
- **Verification**: Enforced by compile-time module boundaries and 224 automated workspace tests.

## ADR-100-02: Public CLI & Schema Compatibility Policy
- **Context**: v1.0.0 locks in the stable CLI subcommands (`doctor`, `run`/`repro`, `freeze-spec`, `verify-receipt`, `sweep`, `export-proof`), exit codes (`0`–`41`), and JSON schemas (`0.9` for cryptographic receipts, `1.0` for demo evidence manifest).
- **Decision**: All receipt verifiers fail closed (`UNSUPPORTED PROTOCOL VERSION` / Exit `40`) on unrecognized schema versions rather than silently reinterpreting fields.

## ADR-100-03: Receipt & Evidence Protocol Stability (RFC 8785 + RFC 6962)
- **Context**: Cross-language verification between Rust (`replayfix-core`) and JavaScript (`proof_viewer.html`) requires byte-for-bit deterministic serialization.
- **Decision**: Enforce RFC 8785 (UTF-16 code-unit key sorting, zero floats, duplicate-key rejection) and RFC 6962 domain separation (`0x00` leaf prefix, `0x01` internal node prefix, empty root `e3b0c44...`).

## ADR-100-04: Cryptographic Trust & 4-Layer Verification
- **Context**: A valid Ed25519 signature proves key ownership, not signer authorization or policy compliance.
- **Decision**: Enforce 4 distinct verification layers: Layer 1 (on-disk artifact SHA-256 & byte size), Layer 2 (RFC 6962 Merkle proofs), Layer 3 (RFC 8032 pure Ed25519 signature over `REPLAYFIX-ATTESTATION:v0.9:...`), and Layer 4 (`TrustPolicy::PinnedKeys` + deterministic gate/regression policy).

## ADR-100-05: AI Orchestration & Deterministic Acceptance
- **Context**: Multi-model candidate generation (`ModelPool`) races up to 3 models (`muse-spark-1.3`, `muse-wonder-1.0`, `muse-spark-1.3-contributor`).
- **Decision**: Candidate selection is governed strictly by `TestFreezeGuard`, `DiffGuard`, `SyntaxPreflight` (`node --check`), 2-run Gate 2 stability, and `RegressionEvaluator` ($F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$), never by model majority vote.

## ADR-100-06: Provider Abstraction, Circuit Breaker & Baseten Fallback
- **Context**: Live cloud endpoints (`https://api.meta.ai/v1/responses`) can experience rate limits (`HTTP 429`) or transient `5xx` outages.
- **Decision**: `MultimodalGateway` enforces a token-bucket `RatePool` (RPM/TPM/in-flight), honors `Retry-After`, trips a 3-state `CircuitBreaker` (`Closed -> Open -> HalfOpen`) after 3 consecutive failures, and routes eligible text/code synthesis tasks to `https://inference.baseten.co/v1` (`meta-models/muse-spark-open`).

## ADR-100-07: Multimodal Provenance & Zero-Audio Rule
- **Context**: Video keyframes, SAM 3.1 (`sam-3-1`) bounding boxes, and ASR (`muse-voice-transcribe-1.0`) transcripts must preserve provenance and prevent audio billing leaks.
- **Decision**: Enforce the Zero-Audio Rule (`ZERO_AUDIO_RULE_VIOLATION` if audio is attached to any text/code model), validate 24 kHz 16-bit mono PCM WAV headers, and label `LiveProvider` vs. `DeterministicFixture` provenance on every response.

## ADR-100-08: Storage, Atomic Persistence & Secret Redaction
- **Context**: Abrupt termination (`panic = "abort"`) or diagnostic logging must never corrupt files or leak credentials.
- **Decision**: All artifact writes use `atomic_write_file` (`.tmp.<pid>.<counter>` -> `fs::rename`), private Ed25519 keys are excluded from `release/v1.0.0/`, and `GatewayConfig` redacts API tokens as `[REDACTED]`.

## ADR-100-09: Resource Budgets & Performance Bounds
- **Context**: External inputs and subprocesses must have strict resource ceilings.
- **Decision**: Enforce 50 MB max receipt size, depth <= 32 JSON nesting, 60s max audio duration, 15s per-task model timeout, and < 60s 5-bug sweep budget (measured at `6.87s` offline).

## ADR-100-10: Distribution, Windows Authenticode Signing & Upgrades
- **Context**: Windows 11 distribution requires an optimized executable and clear Authenticode / Smart App Control guidance.
- **Decision**: Ship `release/v1.0.0/binaries/windows-x64/replayfix.exe` (`3.86 MB`, signed via `scripts/sign-binary.ps1`), verified by `SHA256SUMS.txt` and `SIGNATURE-VERIFICATION.txt`.

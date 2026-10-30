# Agent Living State — ReplayFix

## 1. Project Phase & Active Task
- **Active Milestone**: `v1.0.0` (`100-v1.0.0-final-production-release` — Final Production Release & Hackathon Submission)
- **Status**: Production Release Qualification Complete; 223/223 Tests Passing, Zero Panics, 0 Clippy Warnings. Optimized LTO release binary (`replayfix.exe` v1.0.0), Windows 11 Authenticode signature verification, 180-second Three-Act demonstration video (`replayfix-v1.0.0-demo.mp4`) + `demo-evidence-manifest.json` + WebVTT captions, offline `proof_viewer.html`, and complete `release/v1.0.0/` GitHub & Meta Hackathon submission package verified.
- **Release process**: verify repo → specify → plan/tasks → implement → test → review → QA → security → docs/changelog/release notes → ship → verify remote. Release notes live in `docs/release-notes/vX.Y.Z.md`; every tag needs a changelog entry and release notes.

## 2. Verified Milestones & Evidence
- [x] **v1.0.0 Final Production Release, Authenticode Signing & Three-Act Demonstration (Spec 100)**:
  - Production `replayfix.exe` (`v1.0.0`) compiled with `opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, Authenticode-signed (`Status: Valid`), and packaged in `release/v1.0.0/binaries/windows-x64/`.
  - Real Three-Act execution and 180.0s `1920x1080` 30fps H.264/AAC demonstration video (`release/v1.0.0/demo/replayfix-v1.0.0-demo.mp4`) with `demo-evidence-manifest.json` and WebVTT captions.
  - Complete `release/v1.0.0/` distribution and Meta Hackathon submission package verified by `SHA256SUMS.txt` and automated test `test_v1_0_0_version_and_release_package_integrity`.
- [x] **v0.9.5 Judge Proof Portal & Live Multimodal Cloud Gateway (Spec 095)**:
  - Self-contained single-file `proof_viewer.html` with strict air-gapped CSP (`default-src 'none'; connect-src 'none'`), client-side RFC 8785 UTF-16 canonical JSON, RFC 6962 Merkle proof verification, and pure RFC 8032 Ed25519 verification across 6 honest verification states (`VERIFIED GENUINE`, `SIGNATURE VALID — SIGNER UNTRUSTED`, `INTEGRITY CHECK FAILED`, `UNSIGNED RECEIPT`, `UNSUPPORTED PROTOCOL VERSION`, `MALFORMED RECEIPT`).
  - Bounded `MultimodalGateway` (`crates/replayfix-core/src/gateway.rs`) with token-bucket `RatePool` (RPM/TPM/in-flight), HTTP 429 `Retry-After` backoff, 3-state `CircuitBreaker` (`Closed -> Open -> HalfOpen`), Baseten `meta-models/muse-spark-open` fallback, Zero-Audio Rule enforcement, 24 kHz mono PCM WAV header verification, and automatic credential redaction (`[REDACTED]`).
  - `replayfix export-proof` CLI subcommand and cross-language Node.js parity test executing `proof_viewer.html`'s embedded JS cryptographic engine against Rust vectors.
  - 222 automated tests passing (182 core unit tests, 40 CLI integration tests); zero Clippy warnings; zero panics in production source.
- [x] **v0.9.1 Independent Zero-Trust Audit Remediation & Cryptographic Hardening**:
  - 4-Layer Zero-Trust verification in `AuditReceipt::verify_receipt_and_attestation`: verifies on-disk artifact files (`01_candidate.diff`, `02_preflight.json`, `03_differential_diagnosis.json`, `04_regression_parity.json`, `04_audit_receipt.json`) against committed `EvidenceLeaf` `content_sha256` and `size_bytes`, cross-checks top-level `audit_receipt.json` against `04_audit_receipt.json`, fails closed (`Exit 40`) if `merkle_receipt.json` is missing or any artifact is tampered with on disk, and verifies `sweep_receipt.json` against per-bug receipts.
  - Strict RFC 6962 `MerkleReceipt::verify_all` invariants (`tree_size == leaves.len()`, `proof.tree_size == tree_size`, `proof.leaf_index == idx`, unique leaf `file_name`, and empty remaining `audit_path` at `size == 1`).
  - RFC 8785 Section 3.2.3 UTF-16 code-unit key sorting (`encode_utf16`) in `digest::canonical_json` with supplementary-plane surrogate-pair regression tests.
  - `KeypairManager::load_or_generate` supporting `REPLAYFIX_SIGNING_KEY` and atomic key persistence (`atomic_write_file`).
  - Harmonized `contracts/sweep_receipt.schema.json` with `SweepReceipt`, added pre/post `WorktreeManifest` $\Delta\text{repo}=0$ verification and `atomic_write_file` in `run_sweep`, and upgraded `ModelPool::race_candidates` to bounded concurrent scoped threads with per-task timeouts.
  - 209 automated tests passing (170 core unit tests, 39 CLI integration tests); zero Clippy warnings; zero panics in production source.
- [x] **v0.9.0 RFC 6962 Merkle Receipts, Ed25519 Attestation, 5-Bug Suite & Tri-Model Sweep**:
  - RFC 6962 compliant Merkle trees with explicit domain separation ($0x00$ leaf, $0x01$ node) and reproducible inclusion proofs across all disaggregated fact artifacts.
  - Pure Ed25519 cryptographic signing (`ed25519-dalek`) over versioned signing payload (`REPLAYFIX-ATTESTATION:v0.9:...`), public-key trust policies (`AllowAny`, `PinnedKeys`), and fail-closed exit code 40 (`RECEIPT_VERIFICATION_FAILED`).
  - Independent receipt verifier CLI: `replayfix verify-receipt <path> [--trusted-key <hex>] [--json]`.
  - Authoritative 5-bug MetaPulse benchmark suite (`RF-01` through `RF-05`) with behavioral contracts, buggy sources, clean patches, and HTML test environments.
  - Bounded tri-model orchestration (`ModelPool`) across `[muse-spark-1.3, muse-wonder-1.0, muse-spark-1.3-contributor]` with first-verified-green acceptance and atomic task cancellation.
  - High-performance benchmark sweep CLI: `replayfix sweep` executing all 5 bugs in isolated sandboxes ($\Delta\text{repo}=0$), emitting `.replayfix/sweep_receipt.json` in $< 2$ seconds ($< 60$s SLA).
  - 203 automated tests passing (166 core unit tests, 37 CLI integration tests); zero Clippy warnings; zero panics in production source.
- [x] **v0.0.0 Genesis**: two-crate workspace, platform abstraction, toolchain lock-in.
- [x] **v0.1.0 Walking Skeleton**: `doctor` (Git, Node.js, Playwright, FFmpeg), manifest ingestion, path-traversal rejection.
- [x] **v0.1.1 Hardening**: UNC, DOS-device and ADS path defenses, backslash normalization, platform-aware doctor advice, native test execution (no Cargo runner).
- [x] **v0.2.0 / v0.2.1 Mock Healer & Clean Architecture**:
  - Phase machine `Init → Gate1Red → Repair → Gate2Green` (+ `Rejected`), illegal transitions fail closed.
  - Gates evaluate the same contract (SHA-256 of canonical JSON); mismatch aborts.
  - Strict unified-diff patcher; anti-no-op guard; receipts (`audit_receipt.json` / `triage_report.json`) with stale-artifact removal.
  - Codified 10 Rust Engineering Principles (`docs/engineering-principles-rust.md`) with `unsafe_code = "forbid"` and `missing_debug_implementations = "warn"`.
- [x] **v0.3.0 – v0.3.3 Ephemeral Scratch Isolation & Terminal UX**:
  - `WorktreeManifest`: dual-layer workspace integrity snapshot (Layer 1 recursive SHA-256 content tree + Layer 2 `git status --porcelain=v1 -uall`).
  - `ScratchSandbox`: clones `--repo` into an isolated OS temp directory, runs Gate 1 -> disk patch -> Gate 2 inside the sandbox, verifies $\Delta \text{repo} = 0$ (`BASE_REPO_DIRTY_MUTATION_DETECTED`, exit code 11 on violation), and tears down child process trees and read-only files with retry backoff.
  - `EphemeralPort::allocate()` (`127.0.0.1:0`) and `Platform::terminate_process_tree(pid)`.
  - Terminal UX (`OutputMode`, `--color`, `--quiet`, `--dry-run`, trust boundary cards, structured `doctor` diagnostics).
- [x] **v0.4.0 & v0.4.1 Playwright CDP Grounding, Live Browser Verification & Adversarial Systems Hardening**:
  - `LocatorResolver`: spatial-semantic candidate scoring in integer basis points (`0..=10_000` bps), **DOM Grounding Invariant** cross-checked against Playwright `ariaSnapshot`, `.first()` / `:nth-match` ban, duplicate-selector detection across all page candidates, `getByTestId` fallback when `aria_verified == false`, and **Fail-Closed Ambiguity Guard** ($|\Delta S| < 0.25$ / `< 2_500` bps -> `AMBIGUOUS_GROUNDING`, exit code `22`).
  - `SandboxServer` & `PlaywrightBridge`: serves `ScratchSandbox` over `127.0.0.1:<ephemeral_port>` with concurrent per-connection HTTP handling, path-traversal/drive-prefix/symlink defenses, and drives real Chromium over line-delimited JSON-lines stdio IPC.
- [x] **v0.5.0 Multimodal Video Telemetry, SAM 3.1 & ASR Alignment**:
  - `run_bounded` subprocess supervisor, `VideoPreprocessor` (`ffprobe`, grayscale frames, keyframes, 24 kHz mono PCM WAV), `GeometricCalibration`, `RoiDiffer` ($120 \times 120\text{px}$ $L_1$ optical flow), `SamSegmenter` (`sam-3.1` `<|box;...|>` wire-token centroid snapping), and `AsrTranscriber` (`muse-voice-transcribe-1.0` `audioProcessedMs` locking + Zero-Audio Rule on `muse-spark-1.3`).
- [x] **v0.6.0 Meta Model Bounded Repair, Anti-Cheating Test Freeze & Deliberation Continuity**:
  - `TestFreezeGuard`: SHA-256 pre-repair freeze of all test/config files (`tests/**`, `*.spec.*`, `*.test.*`, `playwright.config.*`, `package.json`), rejecting any patch or disk mutation touching protected paths across case variants or `..` traversal with `TEST_MUTATION_PROHIBITED` (exit code `11`).
  - `MuseModelConfig`: validates `muse-spark-1.3` (`xhigh`), `muse-wonder-1.0` (`high`), `muse-spark-1.3-contributor` (`xhigh`), and fallback `meta-models/muse-spark-open-1026`, rejecting `muse-wonder-full-1.0` and `effort: "max"`.
  - `PromptBuilder` & `RepairRequest`: 24h static prefix caching (`replayfix-v0.6-harness-prefix-v1`), `<UNTRUSTED_CONTEXT>` tag escaping, and Meta Superpower #3 encrypted deliberation continuity (`reasoning.encrypted_content` + `previous_response_id`).
  - `StructuredRepairPatch`, `BlockPatcher`, & `SyntaxPreflight`: strict 5-field JSON Schema (`deny_unknown_fields`), exact single-match uniqueness (`0` -> `PATCH_TARGET_NOT_FOUND`, $\ge 2$ -> `AMBIGUOUS_PATCH`), `node --input-type=module --check` syntax preflight, 2-run consecutive Gate 2 stability (`gate2_runs_completed: 2`), bounded 3-attempt quarantine (`QUARANTINE_REPAIR_FAILED`, exit code `21`), and integer micro-dollar `TokenAccounting`.
  - 151 automated tests passing (125 core unit tests, 24 CLI integration tests, 2 CLI UI unit tests); zero Clippy warnings; zero `.unwrap()` or `.expect()` in production source.
- [x] **v0.6.1 Adversarial Security Hardening & Preflight Reliability**:
  - Windows `FILE_ATTRIBUTE_REPARSE_POINT` (NTFS junctions `mklink /J`) and canonical root containment verification in `write_file`.
  - DOS device stem isolation (`CON.txt.bak`) and trailing dot/space sanitization in `manifest.rs`.
  - `BlockPatcher` overlapping match detection and enclosing declaration boundary scope verification for `touched_symbols`.
  - Exact nano-dollar rate cards across multi-model fallback sequences in `TokenAccounting`.
  - Dynamic `syntax_preflight_passed` state reflection and exhaustive error matching in `engine.rs`.
  - CLI validation rejecting `--reasoning-effort max` with exit code `2`.
  - Lexical tokenizer for `check-panics.ps1` with self-test suite and crate-level AST denial lints.
- [x] **v0.6.2 Packaging Integrity, Test Resiliency & Sandbox Seeding Hardening**:
  - Git packaging: un-ignored `.specify/scripts/` in `.gitignore` and added root `scripts/check-panics.ps1` with dynamic repo root discovery.
  - Windows test runner: replaced fragile `.expect()` in `tests/cli.rs::code()` with `unwrap_or(-1)`.
  - Sandbox seeding: `collect_entries` and `copy_tree` in `worktree.rs` inspect `is_symlink_or_reparse_point(&symlink_meta)` to skip NTFS directory junctions (`mklink /J`).
  - Block patcher hygiene: `strip_comments_and_strings` in `repair.rs` blanks comments (`//`, `/* */`) and string literals before scope extraction.
- [x] **v0.7.0 Differential Diagnosis, Cryptographic Anti-Cheat, Regression Parity & Resumable Checkpoints**:
  - 6-state differential diagnosis matrix (`CAUSAL_DEFECT_CONFIRMED`, `INVALID_TEST_OVERFIT`, `QUARANTINE_INVERTED_BASELINE`, `UNATTRIBUTED_SYMPTOM`, `CLEAN_PASS_UNREPRODUCED`, `FLAKY_NON_DETERMINISTIC`).
  - Surgical `package.json` test script locking (`"scripts.test*"` locked byte-by-byte while allowing dependency updates; violations throw exit `11`).
  - Structured regression parity engine enforcing $F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$, test suite preservation, and zero test skip inflation.
  - Durable resumable checkpoints (`session_state.json`) with git HEAD, manifest hash, and TTL staleness rejection.
  - Schema bumped to `"0.7"` with disaggregated stage artifacts (`candidate.patch`, `repro_spec.json`, `differential_diagnosis.json`, `regression_parity.json`).
  - New subcommand `freeze-spec --bug <ID>` and CLI flags `--clean-ref <REF>`, `--verify-matrix`, `--mock-pkg-test-mutation`.
  - 168 automated tests passing (139 core unit tests, 27 CLI integration tests, 2 CLI unit tests); zero panic violations; zero clippy warnings.
- [x] **v0.7.1 Spec 070 Hardening, Active Regression Parity & Inoculation**:
  - Dedicated `DiffGuard` module (`crates/replayfix-core/src/diffguard.rs`) enforcing runner configuration immutability and structural `package.json` script locking.
  - Active `RegressionEvaluator::evaluate` and test snapshot discovery wiring into `engine.rs`, rejecting test regressions with exit code 31 (`REGRESSION_DETECTED`).
  - `CheckpointManager` orchestrating monotonic stage advancement, durable disk persistence of `session_state.json`, and staleness invalidation.
  - `replayfix freeze-spec --bug <ID>` generating permanent Playwright regression tests (`tests/repro/rf_{id}_regression.spec.ts`) with immutable cryptographic aggregate SHA-256 headers.
  - Complete numbered fact artifacts suite (`01_candidate.diff`, `02_preflight.json`, `03_differential_diagnosis.json`, `04_regression_parity.json`, `04_audit_receipt.json`).
  - Real git ref resolution via `resolve_git_ref`.
  - Added CLI flag `--mock-regression` and integration test asserting exit code 31.
  - 174 automated tests passing (144 core unit tests, 28 CLI integration tests, 2 CLI unit tests); zero panic violations; zero clippy warnings.
- [x] **v0.8.0 Observability Suite, GitHub Issue Ingestion & Human Confirmation Gate**:
  - Playwright Trace Auto-Launcher (`--trace`) capturing `playwright.trace.zip` and spawning `npx playwright show-trace` detached, safely skipping in CI and non-interactive sessions.
  - Live GitHub Issue Ingestion (`--issue <url>`) via strict regex, HTTPS curl/gh CLI fetch, untrusted `<UNTRUSTED_ISSUE_METADATA>` isolation, and automated reproduction step extraction into task manifests.
  - Interactive Human Approval Gate (`--apply` / `--yes`) previewing syntax-highlighted ANSI unified diffs (+green, -red, @@ cyan) and prompting `[y/N]`, failing closed with exit code 13 (`NON_INTERACTIVE_REFUSAL`) in non-TTY sessions unless `--yes` is provided.
  - Base worktree mutation (`UnifiedPatcher::apply_to_worktree`) applying verified and approved patches with path traversal defenses.
  - Disaggregated fact artifacts persisted in `.replayfix/artifacts/` (`candidate.patch`, `approval_audit.json`, `issue_ingestion.json`, `differential_diagnosis.json`, `regression_parity.json`, `playwright.trace.zip`).
  - 185 automated tests passing (152 core unit tests, 33 CLI integration tests); zero Clippy warnings; zero panics in production source.

- [x] **v0.8.2 Gap Remediation & Atomic Write Hardening**:
  - Unified atomic fact artifact pipeline via `ArtifactStore` and `atomic_write_file` (`.tmp` -> `rename` with unique PID/counter) across all CLI and receipt writers with defensive error cleanup.
  - Artifact path traversal and separator validation in `ArtifactStore`.
  - Worktree mutation atomicity in `UnifiedPatcher::apply_to_worktree` via `atomic_write_file`, eliminating partial file writes.
  - Output directory alignment for `write_approval_audit` across custom `--output-dir` arguments.
  - Schema version harmonization to `"0.8"` across `receipt.rs`, `matrix.rs`, `regression.rs`, `main.rs` (`freeze-spec`), and integration tests.
  - Dynamic harness User-Agent resolution via `CARGO_PKG_VERSION` in `ingestion.rs`.
  - 188 automated tests passing (155 core unit tests, 33 CLI integration tests); zero Clippy warnings; zero panics in production source.

- [x] **v0.8.1 GStack Release Qualification & Baseline Hardening**:
  - Adversarial prompt injection defense escaping `<UNTRUSTED_ISSUE_METADATA>` delimiters within GitHub issue titles, steps, and descriptions.
  - Authenticated GitHub API ingestion via `GH_TOKEN` / `GITHUB_TOKEN` environment variables to prevent rate limiting (60/hr) with leak-proof error diagnostics.
  - Terminal injection sanitization in `highlight_diff` neutralizing raw ANSI control sequences (`\x1b` -> `^[`) in diff previews.
  - Interactive EOF fail-closed handling (`io::Cursor::new(b"")`) ensuring safe `ManualNegative` rejection when interactive streams terminate unexpectedly.
  - Spec 080 exit code taxonomy alignment (12, 13, 23, 24).
  - 186 automated tests passing (153 core unit tests, 33 CLI integration tests); zero Clippy warnings; zero panics in production source.

## 3. Toolchain & Environment
- Windows 11 (MSVC native), Rust 1.99.1, Node.js 24.19.0, Git 2.55, FFmpeg 15.2, Playwright 1.64.1 present and verified compatible by `replayfix doctor`.
- Windows Smart App Control can block fresh build outputs: use `scripts/sign-build-outputs.ps1` (see `docs/windows-code-signing.md`).

## 4. Deferred Items (documented, not hidden)
- `tracing_subscriber` is initialised for structured diagnostic logging.

## 5. v1.0.0 Readiness
- **Shipping status**: Spec 000 through Spec 090 (`v0.9.1`) complete and verified. Ready for final v1.0.0 packaging and submission.

## 6. Next Immediate Task
- Stand by for v0.9.1 release tagging / v1.0.0 Grand Finale packaging instructions.

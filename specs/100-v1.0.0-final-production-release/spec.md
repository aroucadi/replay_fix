# 📋 Spec 100: Vertical Slice v1.0.0 — Final Production Release, Optimized Rust Distribution, Windows Authenticode Signing, Three-Act Judge Demonstration & Hackathon Submission

**Feature Identity**: `100-v1.0.0-final-production-release`  
**Target Release**: `v1.0.0` (Minimum Marketable Product / Grand Finale)  
**Baseline Foundation**: `v0.9.5` (`81d5c64`, 222/222 Tests Passing)  
**Status**: Approved for Implementation  

---

## 1. Executive Summary & Product Scope

**ReplayFix v1.0.0** consolidates the ten preceding vertical milestones (`v0.0.0` through `v0.9.5`) into a reproducible, optimized, Authenticode-signed, independently verifiable production release and hackathon submission package. It addresses four distinct audiences simultaneously:
1. **Independent Engineers**: Reproducible `cargo build --release` binary (`opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`), complete CLI documentation (`README.md`), and 100% passing test suite.
2. **Security Reviewers**: Fail-closed `panic = "abort"` invariant backed by zero non-test panics (`scripts/check-panics.ps1`), 4-layer RFC 6962 / RFC 8032 receipt verification, air-gapped `proof_viewer.html`, and sanitized public artifacts with zero secrets.
3. **Windows 11 Users**: Standalone `replayfix.exe` signed with Authenticode (`scripts/sign-binary.ps1`), verified via `Get-AuthenticodeSignature`, packaged with `SHA256SUMS.txt` and honest Windows 11 Smart App Control (`os error 4551`) guidance.
4. **Skeptical Hackathon Judges**: A real, 180-second Three-Act demonstration video (`replayfix-v1.0.0-demo.mp4`) backed by `demo-evidence-manifest.json`, captions (`.vtt`), `proof_viewer.html`, and a complete Meta Hackathon submission package.

---

## 2. User Stories & Priorities

### User Story 1 — Optimized Production Binary & Windows Authenticode Distribution (Priority: P1) 🎯 MVP
As a Windows 11 developer or technical evaluator, I can build or download the optimized `replayfix.exe` (`v1.0.0`), verify its SHA-256 checksum and Authenticode digital signature, and execute all CLI workflows (`doctor`, `run`, `freeze-spec`, `verify-receipt`, `sweep`, `export-proof`) with sub-second startup and deterministic exit codes.

**Acceptance Scenarios**:
1. **Given** the workspace Cargo profile (`opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`), **When** `cargo build --release --workspace` runs, **Then** `target/release/replayfix.exe` compiles cleanly with 0 warnings and reports `replayfix 1.0.0`.
2. **Given** `target/release/replayfix.exe`, **When** signed via `scripts/sign-binary.ps1` and inspected with `Get-AuthenticodeSignature`, **Then** the digital signature is intact under `CN=ReplayFix Local Developer` (Thumbprint `14FAB92131428EC6302B7D025E3611AC32BEE8F8`), trusted via `Cert:\CurrentUser\TrustedPublisher`, and `SHA256SUMS.txt` matches the exact signed binary.

### User Story 2 — Synchronized Three-Act Demonstration Video & Evidence Manifest (Priority: P2)
As a hackathon judge, I can watch a 180-second (`3:00`), 1920x1080 30fps demonstration video (`replayfix-v1.0.0-demo.mp4`) with synchronized narration audio and WebVTT captions covering:
- **Act 1 (`0:00–0:45`) — Grounded Defect**: Reproducing `RF-01` (`src/cart-summary.js` empty-array bug) with multimodal video/SAM 3.1/CDP grounding and Gate 1 RED baseline.
- **Act 2 (`0:45–1:30`) — Cheating & No-Op Trap Rejection**: Demonstrating deterministic rejection of a no-op patch (`REPAIR_NOOP_REJECTED`, Exit `30`) and a protected test-mutation cheating patch (`TEST_MUTATION_PROHIBITED`, Exit `11`).
- **Act 3 (`1:30–3:00`) — Autonomous Causal Repair & Cryptographic Proof**: Demonstrating verified causal repair (`VERIFIED_REPAIRED`, Exit `0`), 2-run Gate 2 stability, active regression parity ($F_{\text{patched}} \setminus F_{\text{baseline}} = \emptyset$), 5-bug benchmark sweep (`RF-01`..`RF-05`), `replayfix verify-receipt`, and offline inspection in `proof_viewer.html`.

### User Story 3 — Complete GitHub Release & Meta Hackathon Submission Package (Priority: P3)
As a release auditor or hackathon organizer, I can inspect `release/v1.0.0/` containing the signed binary, `SHA256SUMS.txt`, `SIGNATURE-VERIFICATION.txt`, `proof_viewer.html`, sanitized receipts, `replayfix-v1.0.0-demo.mp4`, `demo-evidence-manifest.json`, `INSTALLATION.md`, `QUICKSTART.md`, `SECURITY.md`, `VERIFICATION.md`, `HACKATHON_SUBMISSION.md`, `RELEASE_NOTES.md`, and `QUALIFICATION_REPORT.md`.

---

## 3. Normative Functional Requirements

- **REQ-100-01 (Release Profile & `panic = "abort"` Safety)**: Workspace `[profile.release]` MUST enforce `opt-level = 3`, `lto = true`, `codegen-units = 1`, and `panic = "abort"`, backed by `0` non-test panics (`scripts/check-panics.ps1`), OS-enforced subprocess cleanup, and atomic `.tmp` $\rightarrow$ `rename` file writes.
- **REQ-100-02 (Version 1.0.0 Identity)**: `replayfix-core` and `replayfix-cli` MUST report version `1.0.0` across `Cargo.toml`, `replayfix --version`, `README.md`, `CHANGELOG.md`, and release notes.
- **REQ-100-03 (Performance & Size Qualification)**: Debug vs. Release binary size, startup latency (`replayfix --version`), `verify-receipt` latency, and 5-bug `sweep` latency MUST be physically measured and recorded.
- **REQ-100-04 (Windows Authenticode Signing & Verification)**: The production Windows binary MUST be signed via `scripts/sign-binary.ps1`, verified via `Get-AuthenticodeSignature`, and documented with explicit distinction between local developer trust (`Cert:\CurrentUser\TrustedPublisher`) and public CA / Smart App Control cloud reputation.
- **REQ-100-05 (Three-Act Demonstration Video & Evidence Manifest)**: `release/v1.0.0/demo/replayfix-v1.0.0-demo.mp4` MUST be ~180 seconds (`180.0s`), 1920x1080 H.264 video + AAC audio, accompanied by `captions/replayfix-v1.0.0-demo.vtt` and `demo-evidence-manifest.json` mapping every claim to actual CLI commands, exit codes, and SHA-256 artifact digests.
- **REQ-100-06 (Complete Sanitized Release & Meta Hackathon Package)**: `release/v1.0.0/` MUST contain all required binaries, checksums, proof portal, sanitized example receipts, user/security documentation, Meta Hackathon submission text, and final qualification report with zero secrets or private keys.

---

## 4. Measurable Success Criteria

- **SC-100-01**: `cargo build --release --workspace` exits `0` and produces `target/release/replayfix.exe` (`v1.0.0`).
- **SC-100-02**: `Get-AuthenticodeSignature release/v1.0.0/binaries/windows-x64/replayfix.exe` reports `Status: Valid`.
- **SC-100-03**: `ffprobe` confirms `release/v1.0.0/demo/replayfix-v1.0.0-demo.mp4` is valid `1920x1080` `h264`/`aac` MP4 with duration $180.0\text{s} \pm 2.0\text{s}$ covering Act 1 (`0–45s`), Act 2 (`45–90s`), and Act 3 (`90–180s`).
- **SC-100-04**: Every file listed in `release/v1.0.0/SHA256SUMS.txt` exists and matches its recorded SHA-256 digest.
- **SC-100-05**: 100% of workspace tests pass (`cargo test --workspace`), 0 non-test panics (`scripts/check-panics.ps1`), 0 Clippy warnings (`-D warnings`), and clean `cargo fmt --all -- --check`.

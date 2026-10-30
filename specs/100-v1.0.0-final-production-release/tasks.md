# Tasks: Spec 100 — Final Production Release, Optimized Rust Distribution, Windows Authenticode Signing, Three-Act Judge Demonstration & Hackathon Submission

**Input**: Design documents from `/specs/100-v1.0.0-final-production-release/`  
**Prerequisites**: `spec.md`, `plan.md`, `research.md`, `data-model.md`, `contracts/`, `architecture/`, `security/`

---

## Phase 1: Setup & Version 1.0.0 Identity

- [X] T001 Bump workspace crate versions to `1.0.0` in `crates/replayfix-core/Cargo.toml` and `crates/replayfix-cli/Cargo.toml`
- [X] T002 [P] Update `README.md` to document all `v1.0.0` capabilities (`doctor`, `run`/`repro`, `freeze-spec`, `verify-receipt`, `sweep`, `export-proof`, `proof_viewer.html`, and exit codes `0`–`41`)
- [X] T003 [P] Update `CHANGELOG.md`, `docs/release-notes/v1.0.0.md`, and `docs/agent-state.md` for `v1.0.0`

---

## Phase 2: User Story 1 — Optimized Production Binary & Windows Authenticode Signing (Priority: P1) 🎯 MVP

- [X] T004 [US1] Build optimized release binary (`cargo build --release --workspace`) and benchmark size and latency vs. debug build
- [X] T005 [US1] Provision/verify `CN=ReplayFix Local Developer` certificate via `scripts/setup-dev-cert.ps1` and sign `target/release/replayfix.exe` via `scripts/sign-binary.ps1`
- [X] T006 [US1] Copy signed `replayfix.exe` to `release/v1.0.0/binaries/windows-x64/replayfix.exe` and record `Get-AuthenticodeSignature` output in `release/v1.0.0/binaries/windows-x64/SIGNATURE-VERIFICATION.txt` and `SHA256SUMS.txt`

---

## Phase 3: User Story 2 — Synchronized Three-Act Demonstration Video & Evidence Manifest (Priority: P2)

- [X] T007 [US2] Execute real Act 1 (Grounded Defect `RF-01`), Act 2 (No-Op `Exit 30` & Cheating `Exit 11` rejection), and Act 3 (Verified Repair, 5-Bug Sweep & `verify-receipt`) using the signed `v1.0.0` binary and save sanitized artifacts in `release/v1.0.0/examples/sanitized-receipts/`
- [X] T008 [US2] Generate synchronized 180.0-second (`1920x1080`, `30fps`, H.264/AAC) Three-Act demonstration video at `release/v1.0.0/demo/replayfix-v1.0.0-demo.mp4` and WebVTT captions at `release/v1.0.0/demo/captions/replayfix-v1.0.0-demo.vtt`
- [X] T009 [US2] Write `release/v1.0.0/demo/demo-evidence-manifest.json` binding Act 1, Act 2, and Act 3 video timestamps to exact commands, exit codes, and SHA-256 artifact digests

---

## Phase 4: User Story 3 — Complete GitHub Release & Meta Hackathon Submission Package (Priority: P3)

- [X] T010 [US3] Copy `proof_viewer.html` to `release/v1.0.0/proof-portal/proof_viewer.html`
- [X] T011 [P] [US3] Author `release/v1.0.0/documentation/INSTALLATION.md`, `QUICKSTART.md`, `SECURITY.md`, `VERIFICATION.md`, and `HACKATHON_SUBMISSION.md`
- [X] T012 [US3] Author `release/v1.0.0/RELEASE_NOTES.md`, `release/v1.0.0/QUALIFICATION_REPORT.md`, and compute `release/v1.0.0/SHA256SUMS.txt` across all packaged deliverables
- [X] T013 [US3] Add automated release-qualification integration test `test_v1_0_0_version_and_release_package_integrity` in `crates/replayfix-cli/tests/cli.rs`

---

## Phase 5: Final Verification & Release Qualification Gates

- [X] T014 Update `specs/100-v1.0.0-final-production-release/verification/requirement-traceability.md`, `release-checklist.md`, and `qualification-matrix.md`
- [X] T015 Execute full mandatory quality gates (`scripts/check-panics.ps1`, `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `git diff --check`)

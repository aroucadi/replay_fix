# Implementation Plan: Spec 100 — Final Production Release, Optimized Rust Distribution, Windows Authenticode Signing, Three-Act Judge Demonstration & Hackathon Submission

**Branch**: `main` (`100-v1.0.0-final-production-release`) | **Date**: 2026-10-30 | **Spec**: [spec.md](spec.md)

---

## 1. Summary

Transform the verified `v0.9.5` codebase (`81d5c64`) into the **ReplayFix v1.0.0 Minimum Marketable Product (MMP)**:
1. Consolidate historical capabilities (`v0.0.0`–`v0.9.5`) and update `README.md`, `CHANGELOG.md`, `docs/release-notes/v1.0.0.md`, and `docs/agent-state.md`.
2. Bump crate versions to `1.0.0` and compile the optimized production binary (`opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`).
3. Sign `replayfix.exe` with Windows Authenticode (`scripts/setup-dev-cert.ps1` + `scripts/sign-binary.ps1`) and record `Get-AuthenticodeSignature` verification and Smart App Control behavior.
4. Execute real Act 1, Act 2, and Act 3 CLI workflows with the signed `1.0.0` binary, capture their genuine receipts/triage reports, and synthesize the synchronized 180-second `replayfix-v1.0.0-demo.mp4` video, WebVTT captions, and `demo-evidence-manifest.json`.
5. Package and verify `release/v1.0.0/` with `SHA256SUMS.txt`, user documentation, and Meta Hackathon submission materials.

---

## 2. Technical Context

- **Language/Compiler**: Rust 2021 (`rustc 1.99.1`, `cargo 1.99.1`), Target `x86_64-pc-windows-msvc`
- **Release Profile**: `opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`
- **Signing Infrastructure**: PowerShell `Set-AuthenticodeSignature` / `Get-AuthenticodeSignature` with `CN=ReplayFix Local Developer` (`Cert:\CurrentUser\My` & `Cert:\CurrentUser\TrustedPublisher`)
- **Media Pipeline**: `ffmpeg` + `ffprobe` generating 1920x1080 30fps H.264/AAC MP4 (`180.0s`)

---

## 3. Constitution Check

All 10 Constitutional Laws (`Law I`–`Law X`) remain strictly enforced and verified via automated tests and `scripts/check-panics.ps1`.

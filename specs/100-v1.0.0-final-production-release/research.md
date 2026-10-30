# Research & Consolidation Decisions: Spec 100 (v1.0.0)

## 1. `panic = "abort"` Safety & Reliability Analysis
- **Decision**: Retain `panic = "abort"` in `[profile.release]`.
- **Rationale**:
  1. Production code across `replayfix-core` and `replayfix-cli` is compiled with `#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, clippy::unreachable))]` and audited by `scripts/check-panics.ps1` (`0` violations).
  2. All file mutations use two-phase atomic writes (`.tmp.<pid>.<counter>` $\rightarrow$ `fs::rename`), so an abrupt process termination can never leave half-written JSON receipts or corrupted repository files.
  3. Eliminating unwinding tables reduces binary footprint and prevents unwind-based control-flow side effects.

## 2. Windows 11 Authenticode & Smart App Control (`os error 4551`)
- **Decision**: Sign `target/release/replayfix.exe` using `scripts/sign-binary.ps1` (`CN=ReplayFix Local Developer` in `Cert:\CurrentUser`) and explicitly document the distinction between local developer trust and public EV/OV CA reputation.
- **Rationale**: On developer machines where `scripts/setup-dev-cert.ps1` has registered the public certificate in `Cert:\CurrentUser\TrustedPublisher`, `Get-AuthenticodeSignature` returns `Status: Valid` and local execution succeeds. On external third-party Windows 11 machines with Smart App Control in Enforce mode and no local cert installed, cloud ISG reputation or an Azure Trusted Signing / public CA certificate is required.

## 3. Synchronized Three-Act Video Production
- **Decision**: Execute the real `v1.0.0` `replayfix.exe` binary across Act 1 (`RF-01` grounded defect), Act 2 (No-Op `Exit 30` & Cheating `Exit 11` rejection), and Act 3 (Verified repair, 5-bug sweep & `proof_viewer.html`), record exact artifacts and hashes in `demo-evidence-manifest.json`, and render a 180.0-second (`1920x1080`, `30fps`, `libx264` + `aac`) 3-act presentation video with synchronized WebVTT captions via `ffmpeg`.

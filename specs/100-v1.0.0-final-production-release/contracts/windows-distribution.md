# Contract: Windows 11 Distribution & Packaging

## 1. Supported Target Matrix
- **Primary OS**: Microsoft Windows 11 / Windows 10 (64-bit, `x86_64-pc-windows-msvc`)
- **Cross-Platform Source Support**: macOS (`aarch64`/`x86_64`) and Linux (`x86_64`) via `cargo build --release`

## 2. Distribution Layout (`release/v1.0.0/`)
- `binaries/windows-x64/replayfix.exe` (Optimized & Authenticode-signed production executable)
- `binaries/windows-x64/SHA256SUMS.txt`
- `binaries/windows-x64/SIGNATURE-VERIFICATION.txt`
- `proof-portal/proof_viewer.html`
- `demo/replayfix-v1.0.0-demo.mp4`, `demo/demo-evidence-manifest.json`, `demo/captions/replayfix-v1.0.0-demo.vtt`
- `examples/sanitized-receipts/` (`audit_receipt.json`, `merkle_receipt.json`, `attestation.json`, `sweep_receipt.json`, `triage_noop.json`, `triage_cheat.json`)
- `documentation/` (`INSTALLATION.md`, `QUICKSTART.md`, `SECURITY.md`, `VERIFICATION.md`, `HACKATHON_SUBMISSION.md`)
- `RELEASE_NOTES.md`, `QUALIFICATION_REPORT.md`, `SHA256SUMS.txt`

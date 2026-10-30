# ADR-002: Windows 11 Standalone Distribution Packaging

## Status
Accepted

## Context
Developers and judges on Windows 11 need a zero-installer, self-contained distribution package containing the signed `replayfix.exe` binary, `proof_viewer.html`, sanitized receipts, demo video, and checksums.

## Decision
Distribute ReplayFix v1.0.0 as a structured directory and release package under `release/v1.0.0/` with:
- Standalone statically linked MSVC executable `binaries/windows-x64/replayfix.exe` (requiring only standard Windows system DLLs).
- `SHA256SUMS.txt` and `SIGNATURE-VERIFICATION.txt` for independent cryptographic verification.
- Self-contained `proof-portal/proof_viewer.html` and `examples/sanitized-receipts/`.

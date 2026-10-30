# ADR-003: Windows Authenticode Code Signing & Smart App Control (`os error 4551`)

## Status
Accepted

## Context
Windows 11 Smart App Control (SAC) and Windows Defender Application Control (WDAC) can block unsigned local binaries with `os error 4551`.

## Decision
1. Provision and sign `replayfix.exe` using `scripts/setup-dev-cert.ps1` and `scripts/sign-binary.ps1` (`CN=ReplayFix Local Developer` in `Cert:\CurrentUser\My` and `Cert:\CurrentUser\TrustedPublisher`).
2. Verify the Authenticode signature via `Get-AuthenticodeSignature` (intact signature under `CN=ReplayFix Local Developer`, Thumbprint `14FAB92131428EC6302B7D025E3611AC32BEE8F8`) and record the exact signer thumbprint and binary SHA-256 in `SIGNATURE-VERIFICATION.txt`.
3. Explicitly document that `CN=ReplayFix Local Developer` establishes trust on the local machine where `setup-dev-cert.ps1` was run, whereas external third-party Windows 11 machines with Smart App Control in Enforce mode require either running `setup-dev-cert.ps1` + `sign-binary.ps1` locally or an EV/OV public CA / Azure Trusted Signing certificate with Microsoft ISG reputation.

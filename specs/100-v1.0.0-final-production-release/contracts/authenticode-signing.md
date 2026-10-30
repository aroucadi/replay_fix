# Contract: Windows Authenticode Code Signing & Verification

## 1. Signing Mechanism
- **Certificate Subject**: `CN=ReplayFix Local Developer` (`CodeSigningCert`, 2048-bit RSA / SHA-256)
- **Certificate Stores**: `Cert:\CurrentUser\My` (DPAPI-protected private key) and `Cert:\CurrentUser\TrustedPublisher` (public certificate)
- **Signing Tool**: `scripts/sign-binary.ps1` invoking `Set-AuthenticodeSignature`
- **Verification Tool**: `Get-AuthenticodeSignature` asserting intact signature and `CN=ReplayFix Local Developer` (Thumbprint `14FAB92131428EC6302B7D025E3611AC32BEE8F8`)

## 2. Smart App Control (`os error 4551`) Honesty Rule
- A valid signature under `CN=ReplayFix Local Developer` satisfies Windows Defender / WDAC / Smart App Control on machines where `scripts/setup-dev-cert.ps1` has registered the public certificate in `Cert:\CurrentUser\TrustedPublisher`.
- On external Windows 11 machines with Smart App Control in Enforce mode and no local developer certificate installed, public CA / Azure Trusted Signing + Microsoft Intelligent Security Graph (ISG) reputation is required. Documentation MUST never claim that a local developer certificate bypasses Smart App Control on arbitrary third-party machines without installing the trust anchor.

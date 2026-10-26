# Windows Code Signing & Smart App Control Guide

This guide explains how ReplayFix handles Windows 11 Smart App Control (SAC), Windows Defender Application Control (WDAC), and Authenticode code-signing for developers.

---

## 1. Understanding Smart App Control (SAC) on Windows 11

### What is Smart App Control?
[Windows 11 Smart App Control](https://support.microsoft.com/fr-fr/windows/security/threat-malware-protection/smart-app-control-has-blocked-part-of-this-app) is an OS security feature that blocks unrecognized or untrusted executables and libraries from running unless they possess a known reputation in Microsoft's cloud Intelligent Security Graph (ISG) or are signed with a trusted digital certificate.

### Why Developers Encounter SAC Notifications
When you compile Rust code locally:
1. **Newly Compiled Binaries**: Binaries created on your local drive (`target\debug\replayfix.exe`, test runners) have zero cloud reputation because they were just created.
2. **Procedural Macro DLLs**: Crates such as `clap_derive` compile dynamically loaded libraries (`clap_derive-*.dll`) that `rustc.exe` loads in-memory during compilation.
3. **PowerShell Execution Wrappers**: If a build system wraps test execution inside `powershell.exe -ExecutionPolicy Bypass`, Smart App Control treats PowerShell as an untrusted script host executing unrecognized code, triggering toast notifications:
   > *"Part of this app has been blocked — Some features of Windows PowerShell may not work because we can't confirm who published..."*

---

## 2. ReplayFix Design: Zero Unnecessary Complexity

### Why We Removed the Forced Cargo Runner
Earlier iterations of ReplayFix used a custom runner in `.cargo/config.toml` that wrapped every test binary execution inside PowerShell to sign it on the fly. 

**This proved counterproductive:**
- Spawning PowerShell for every test binary introduced significant execution latency.
- Executing binaries through `powershell.exe -ExecutionPolicy Bypass` actively triggered Smart App Control's script host heuristics and caused notification popups.
- It created machine-specific dependencies that caused issues when cloning the repo on other systems.

**Current Architecture:**
- **Standard Local Testing**: `cargo test --workspace` runs **natively** via Win32 `CreateProcessW` without any custom runner. All tests run in milliseconds without spawning PowerShell.
- **Standalone Release Binaries**: Standalone executables (`target\release\replayfix.exe`) intended for distribution or manual execution can be signed on demand using our developer script.
- **Opt-in Tooling**: Dedicated scripts in `scripts/` provide reproducible certificate management without enforcing brittle global Cargo overrides.

---

## 3. Developer Onboarding & Certificate Setup

For developers running on Windows systems where Smart App Control or WDAC is strictly enforced, we provide a safe, repeatable bootstrap script:

### Step 1: Check Certificate Status
Verify whether your system already has a valid ReplayFix developer certificate:
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\setup-dev-cert.ps1 -Check
```

### Step 2: Generate and Install Developer Certificate
If not installed, create a self-signed developer certificate:
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\setup-dev-cert.ps1
```

**Security Guarantees:**
- **Zero Elevation Required**: Operates strictly within `Cert:\CurrentUser` (`My` and `TrustedPublisher`). Does not touch machine-wide certificate stores.
- **Private Key Isolation**: The private key is held securely in your Windows user profile key store (DPAPI-protected). It is **never** committed to Git or exported to disk.
- **Safe Public Trust**: Only the public key is registered in `Cert:\CurrentUser\TrustedPublisher` so Windows Defender recognizes binaries you compile locally.

### Step 3: Uninstalling / Clean Up
To remove the developer certificate at any time:
```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\setup-dev-cert.ps1 -Uninstall
```

---

## 4. Signing Release Binaries

When building production or release binaries for local testing outside Cargo:

```powershell
# 1. Build the release binary
cargo build --release

# 2. Sign the binary using your local developer certificate
powershell -ExecutionPolicy Bypass -File .\scripts\sign-binary.ps1 target\release\replayfix.exe
```

---

## 5. Separation of Environments (Dev vs. CI vs. Production)

| Environment | Mechanism | Details |
| :--- | :--- | :--- |
| **Local Development** | Native Execution / Self-Signed Dev Cert | Native `cargo test` runs directly. Standalone binaries signed with `CN=ReplayFix Local Developer`. |
| **Continuous Integration (CI)** | Native Linux / Windows Runners | CI runs on standard GitHub Actions runners where SAC is not enforcing untrusted user binaries. No local signing runner needed. |
| **Production Release** | Authenticode CA Certificate | Production distributions use a standard corporate/developer certificate from a public CA (e.g., DigiCert, Sectigo, or Azure Trusted Signing). |

---

## 6. Recommended Windows Configuration for Rust Developers

Microsoft officially recommends the following configurations for developers compiling code on Windows 11:

1. **Windows Developer Drive (Dev Drive)**:
   Formatting a development partition as **ReFS** with Dev Drive provides automatic performance optimizations and optimized antivirus scanning for source trees and `target/` directories.
2. **Defender Folder Exclusions**:
   If Smart App Control continues to generate notifications when compiling procedural macros (`clap_derive`, `serde_derive`), add the workspace `target\` directory to Windows Defender's folder exclusions:
   - *Windows Security* $\rightarrow$ *Virus & threat protection settings* $\rightarrow$ *Exclusions* $\rightarrow$ *Add target folder*.

---

## 7. Troubleshooting & FAQs

### Q: Why do I see Event ID 3077 in the Windows Event Viewer?
**A**: Event 3077 in `Microsoft-Windows-CodeIntegrity/Operational` indicates that Smart App Control evaluated an unsigned library or binary (such as a freshly compiled proc-macro DLL loaded by `rustc.exe`). In Evaluation mode, SAC logs these events to evaluate application safety without crashing your build.

### Q: Does running without a Cargo runner break anything?
**A**: No. Native Cargo execution is the standard, official way Rust software is built and tested across Windows, macOS, and Linux. Removing the forced runner eliminates unnecessary overhead and prevents PowerShell SAC warnings.

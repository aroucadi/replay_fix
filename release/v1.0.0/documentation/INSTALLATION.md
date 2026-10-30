# ReplayFix v1.0.0 — Installation & Windows 11 Setup Guide

## 1. Option A: Pre-Built Windows 11 64-Bit Executable (`replayfix.exe`)

1. Locate `release/v1.0.0/binaries/windows-x64/replayfix.exe`.
2. Verify its SHA-256 checksum (`f799f0f1f4182b2fc3d0e9a4871aae1bb68fec6399db0a519f0f7325c6ca7f74`):
   ```powershell
   Get-FileHash -Algorithm SHA256 .elease\v1.0.0\binaries\windows-x64eplayfix.exe
   ```
3. **Windows 11 Smart App Control (SAC) / WDAC Users**:
   If Windows 11 Smart App Control is in Enforce mode on a new machine, register the local developer trust anchor and sign the executable in `Cert:\CurrentUser` (zero administrator privileges required):
   ```powershell
   powershell -ExecutionPolicy Bypass -File .\scripts\setup-dev-cert.ps1
   powershell -ExecutionPolicy Bypass -File .\scripts\sign-binary.ps1 .elease\v1.0.0\binaries\windows-x64eplayfix.exe
   ```

## 2. Option B: Build from Source (`Windows`, `macOS`, `Linux`)

Requires Rust $\ge 1.80$ (tested on `rustc 1.99.1`):

```bash
cargo build --release --workspace
./target/release/replayfix --version
```

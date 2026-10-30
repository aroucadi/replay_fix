# Quickstart & Validation Guide: Spec 100 (v1.0.0)

## 1. Build & Sign the Optimized Production Binary
```powershell
cargo build --release --workspace
powershell -ExecutionPolicy Bypass -File .\scripts\setup-dev-cert.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\sign-binary.ps1 targeteleaseeplayfix.exe
Get-AuthenticodeSignature targeteleaseeplayfix.exe | Format-List Status,StatusMessage,SignerCertificate,Path
```

## 2. Execute Three-Act Verification with Production Binary
```powershell
# Act 1 & 3: Autonomous Verified Repair + Cryptographic Receipt
.	argeteleaseeplayfix.exe --mock-heal --output-dir .replayfix/v1_demo_repair
.	argeteleaseeplayfix.exe verify-receipt .replayfix/v1_demo_repair/audit_receipt.json --json

# Act 2: No-Op Rejection (Exit 30)
.	argeteleaseeplayfix.exe --mock-heal --mock-patch noop --output-dir .replayfix/v1_demo_noop

# Act 3: 5-Bug Benchmark Sweep (RF-01..RF-05)
.	argeteleaseeplayfix.exe sweep --mock-heal --output .replayfix/v1_demo_sweep --json
```

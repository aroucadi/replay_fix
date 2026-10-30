# Quickstart & Validation Guide: Spec 100 (v1.0.0)

## 1. Build & Sign the Optimized Production Binary
```powershell
cargo build --release --workspace
powershell -ExecutionPolicy Bypass -File ./scripts/setup-dev-cert.ps1
powershell -ExecutionPolicy Bypass -File ./scripts/sign-binary.ps1 target/release/replayfix.exe
Get-AuthenticodeSignature target/release/replayfix.exe | Format-List Status,StatusMessage,SignerCertificate,Path
```

## 2. Execute Three-Act Verification with Production Binary
```powershell
# Act 1 & 3: Autonomous Verified Repair + Cryptographic Receipt
./target/release/replayfix.exe run --bug RF-01 --mock-heal --output-dir .replayfix/v1_demo_repair
./target/release/replayfix.exe verify-receipt .replayfix/v1_demo_repair/audit_receipt.json --json

# Act 2: No-Op Rejection (Exit 30)
./target/release/replayfix.exe run --bug RF-01 --mock-heal --mock-patch noop --output-dir .replayfix/v1_demo_noop

# Act 3: 5-Bug Benchmark Sweep (RF-01..RF-05)
./target/release/replayfix.exe sweep --mock-heal --output .replayfix/v1_demo_sweep --json
```

# ReplayFix v1.0.0 — 60-Second Quickstart Guide

## 1. Check Environment Health (`doctor`)
```powershell
.elease\v1.0.0\binaries\windows-x64eplayfix.exe doctor
```

## 2. Run Autonomous Causal Repair on `RF-01`
```powershell
.elease\v1.0.0\binaries\windows-x64eplayfix.exe run --bug RF-01 --mock-heal --output-dir .replayfix/rf01
```

## 3. Cryptographically Verify the Generated Receipt (`verify-receipt`)
```powershell
.elease\v1.0.0\binaries\windows-x64eplayfix.exe verify-receipt .replayfix/rf01/audit_receipt.json --json
```

## 4. Run the Five-Bug MetaPulse Benchmark Sweep (`RF-01`..`RF-05`)
```powershell
.elease\v1.0.0\binaries\windows-x64eplayfix.exe sweep --mock-heal --output .replayfix/sweep
```

## 5. Open the Offline Judge Proof Portal (`proof_viewer.html`)
Open `release/v1.0.0/proof-portal/proof_viewer.html` in any web browser (`file://`) and drag-and-drop `.replayfix/rf01/merkle_receipt.json` or `.replayfix/sweep/sweep_receipt.json` (or click the built-in presets) to inspect the RFC 6962 Merkle proofs, Ed25519 signature, SAM 3.1 spatial overlays, and unified diff offline.

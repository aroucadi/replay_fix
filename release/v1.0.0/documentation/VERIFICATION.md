# ReplayFix v1.0.0 — Independent Verification Instructions

## 1. Verify Package SHA-256 Checksums
From the repository root in PowerShell:
```powershell
Get-Content release/v1.0.0/SHA256SUMS.txt | ForEach-Object {
    $parts = $_ -split "  ", 2
    $actual = (Get-FileHash -Algorithm SHA256 "release/v1.0.0/$($parts[1])").Hash.ToLower()
    if ($actual -ne $parts[0]) { throw "Hash mismatch on $($parts[1])" }
    Write-Output "[VERIFIED] $($parts[1]): $actual"
}
```

## 2. Verify Packaged Receipts Using the Signed Production Binary
```powershell
.elease\v1.0.0\binaries\windows-x64eplayfix.exe verify-receipt release/v1.0.0/examples/sanitized-receipts/act3_verified/audit_receipt.json --json
.elease\v1.0.0\binaries\windows-x64eplayfix.exe verify-receipt release/v1.0.0/examples/sanitized-receipts/act3_sweep/sweep_receipt.json --json
```

## 3. Run the Full Automated Workspace Test & Panic Audit Suite
```powershell
powershell -ExecutionPolicy Bypass -File scripts/check-panics.ps1
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

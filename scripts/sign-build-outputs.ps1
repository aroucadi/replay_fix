<#
.SYNOPSIS
    Signs every unsigned executable and DLL in a Cargo build output folder.

.DESCRIPTION
    Windows Smart App Control (SAC) blocks freshly built binaries that have no reputation.
    That includes proc-macro DLLs (error "can't find crate for `..._impl`") and the
    `replayfix.exe` that integration tests launch (os error 4551).
    This script signs whatever is still unsigned using the local developer certificate
    created by setup-dev-cert.ps1, then you simply re-run cargo.

    It never creates or trusts certificates and does nothing on a machine without the
    certificate; it prints how to set one up.

.PARAMETER Profile
    Cargo profile folder under target\ (default: debug).

.EXAMPLE
    .\scripts\sign-build-outputs.ps1
    cargo test --workspace

.EXAMPLE
    .\scripts\sign-build-outputs.ps1 -Profile release
#>

param(
    [string]$Profile = "debug"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$signer = Join-Path $PSScriptRoot "sign-binary.ps1"
$targets = @(
    (Join-Path $root "target\$Profile"),
    (Join-Path $root "target\$Profile\deps")
)

$files = foreach ($dir in $targets) {
    if (Test-Path $dir) {
        Get-ChildItem -Path $dir -File | Where-Object { $_.Extension -in ".exe", ".dll" }
    }
}

$unsigned = @($files | Where-Object { (Get-AuthenticodeSignature $_.FullName).Status -eq "NotSigned" })
if ($unsigned.Count -eq 0) {
    Write-Host "[OK] Nothing to sign in target\$Profile."
    exit 0
}

$signed = 0
foreach ($file in $unsigned) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File $signer -FilePath $file.FullName | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Could not sign $($file.Name). Run scripts\setup-dev-cert.ps1 -Check for certificate status."
        exit 1
    }
    $signed++
}
Write-Host "[OK] Signed $signed file(s) in target\$Profile. Re-run your cargo command."

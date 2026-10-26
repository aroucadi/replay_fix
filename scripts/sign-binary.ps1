<#
.SYNOPSIS
    Signs a ReplayFix executable with the local developer Authenticode certificate.

.DESCRIPTION
    Signs target executables (e.g. target\release\replayfix.exe) using the local
    'CN=ReplayFix Local Developer' certificate installed in Cert:\CurrentUser\My.

.PARAMETER FilePath
    Path to the executable or DLL to sign.

.EXAMPLE
    .\scripts\sign-binary.ps1 target\release\replayfix.exe
#>

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$FilePath
)

$SubjectName = "CN=ReplayFix Local Developer"
$ErrorActionPreference = "Stop"

if (-not (Test-Path $FilePath)) {
    Write-Error "Target file not found: $FilePath"
    exit 1
}

$cert = Get-ChildItem -Path "Cert:\CurrentUser\My" -CodeSigningCert -ErrorAction SilentlyContinue |
    Where-Object { ($_.Subject -eq $SubjectName -or $_.Subject -like "*$SubjectName*") -and ($_.NotAfter -gt (Get-Date)) } |
    Sort-Object NotAfter -Descending |
    Select-Object -First 1

if (-not $cert) {
    Write-Warning "No valid '$SubjectName' certificate found in Cert:\CurrentUser\My."
    Write-Host "Run '.\scripts\setup-dev-cert.ps1' to generate one, then re-run this script." -ForegroundColor Cyan
    exit 1
}

try {
    $sig = Set-AuthenticodeSignature -FilePath $FilePath -Certificate $cert
    Write-Host "[OK] Signed '$FilePath' with certificate $($cert.Thumbprint)" -ForegroundColor Green
    exit 0
} catch {
    Write-Error "Failed to sign file: $_"
    exit 1
}

<#
.SYNOPSIS
    Manages the local developer Authenticode code-signing certificate for ReplayFix on Windows.

.DESCRIPTION
    On Windows 11 systems with Smart App Control (SAC) or Windows Defender Application Control
    (WDAC) active, locally compiled development binaries are blocked unless signed by a trusted
    certificate. This script safely generates and installs a developer certificate into the
    CurrentUser store without requiring administrative privileges, or checks/removes it.

    Security Boundaries:
    - Never exports or exposes private keys outside the Windows user key store.
    - Installs solely into Cert:\CurrentUser (My and TrustedPublisher), never touching Machine store.
    - Zero administrative elevation required.

.PARAMETER Check
    Checks whether a valid, non-expired ReplayFix developer certificate is currently installed and trusted.

.PARAMETER Uninstall
    Removes the ReplayFix developer certificate from CurrentUser\My and CurrentUser\TrustedPublisher.

.PARAMETER Force
    Forces regeneration of the certificate even if one already exists.

.EXAMPLE
    .\scripts\setup-dev-cert.ps1
    # Sets up the certificate if not already present.

.EXAMPLE
    .\scripts\setup-dev-cert.ps1 -Check
    # Verifies certificate installation and trust status.

.EXAMPLE
    .\scripts\setup-dev-cert.ps1 -Uninstall
    # Removes the certificate from the current user store.
#>

[CmdletBinding()]
param(
    [switch]$Check,
    [switch]$Uninstall,
    [switch]$Force
)

$SubjectName = "CN=ReplayFix Local Developer"
$ErrorActionPreference = "Stop"

function Get-ReplayFixCert([string]$StoreLocation) {
    if (-not (Test-Path $StoreLocation)) {
        return $null
    }
    return Get-ChildItem -Path $StoreLocation -ErrorAction SilentlyContinue |
        Where-Object { $_.Subject -eq $SubjectName -or $_.Subject -like "*$SubjectName*" } |
        Sort-Object NotAfter -Descending |
        Select-Object -First 1
}

# --- UNINSTALL MODE ---
if ($Uninstall) {
    Write-Host "[ReplayFix] Removing local developer certificates ($SubjectName)..." -ForegroundColor Yellow
    $removedCount = 0

    $myCert = Get-ReplayFixCert "Cert:\CurrentUser\My"
    if ($myCert) {
        Remove-Item $myCert.PSPath -Force -ErrorAction SilentlyContinue
        Write-Host "  [OK] Removed private certificate from Cert:\CurrentUser\My ($($myCert.Thumbprint))" -ForegroundColor Green
        $removedCount++
    }

    $pubCert = Get-ReplayFixCert "Cert:\CurrentUser\TrustedPublisher"
    if ($pubCert) {
        Remove-Item $pubCert.PSPath -Force -ErrorAction SilentlyContinue
        Write-Host "  [OK] Removed public trust from Cert:\CurrentUser\TrustedPublisher ($($pubCert.Thumbprint))" -ForegroundColor Green
        $removedCount++
    }

    if ($removedCount -eq 0) {
        Write-Host "[ReplayFix] No existing certificates found to remove." -ForegroundColor Gray
    } else {
        Write-Host "[ReplayFix] Successfully uninstalled local developer certificate." -ForegroundColor Green
    }
    exit 0
}

# --- CHECK / DETECTION MODE ---
$existingMy = Get-ReplayFixCert "Cert:\CurrentUser\My"
$existingPub = Get-ReplayFixCert "Cert:\CurrentUser\TrustedPublisher"

$isValid = $false
if ($existingMy -and $existingPub) {
    $now = Get-Date
    if ($existingMy.NotAfter -gt $now) {
        $isValid = $true
    }
}

if ($Check) {
    if ($isValid) {
        Write-Host "[ReplayFix] Developer certificate is installed, trusted, and valid." -ForegroundColor Green
        Write-Host "  Subject:    $($existingMy.Subject)"
        Write-Host "  Thumbprint: $($existingMy.Thumbprint)"
        Write-Host "  Expires:    $($existingMy.NotAfter)"
        exit 0
    } else {
        if ($existingMy -and ($existingMy.NotAfter -le (Get-Date))) {
            Write-Warning "[ReplayFix] Certificate exists in CurrentUser\My but has EXPIRED on $($existingMy.NotAfter)."
        } elseif (-not $existingMy) {
            Write-Warning "[ReplayFix] No code-signing certificate found in Cert:\CurrentUser\My."
        } elseif (-not $existingPub) {
            Write-Warning "[ReplayFix] Certificate exists in CurrentUser\My but is NOT trusted in Cert:\CurrentUser\TrustedPublisher."
        }
        Write-Host "[ReplayFix] Run '.\scripts\setup-dev-cert.ps1' to generate and trust a valid certificate." -ForegroundColor Cyan
        exit 1
    }
}

# --- SETUP / GENERATION MODE ---
if ($isValid -and -not $Force) {
    Write-Host "[ReplayFix] Valid developer certificate is already installed and trusted." -ForegroundColor Green
    Write-Host "  Thumbprint: $($existingMy.Thumbprint)"
    Write-Host "  Expires:    $($existingMy.NotAfter)"
    Write-Host "Use -Force if you need to regenerate it." -ForegroundColor Gray
    exit 0
}

Write-Host "[ReplayFix] Creating local development code-signing certificate..." -ForegroundColor Cyan

try {
    # 1. Clean up expired / existing certificates if regenerating
    if ($Force) {
        if ($existingMy) { Remove-Item $existingMy.PSPath -Force -ErrorAction SilentlyContinue }
        if ($existingPub) { Remove-Item $existingPub.PSPath -Force -ErrorAction SilentlyContinue }
    }

    # 2. Generate self-signed Code Signing certificate (valid for 2 years)
    $newCert = New-SelfSignedCertificate `
        -Type CodeSigningCert `
        -Subject $SubjectName `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -NotAfter (Get-Date).AddYears(2) `
        -FriendlyName "ReplayFix Local Development Certificate"

    Write-Host "  [OK] Generated private certificate in Cert:\CurrentUser\My" -ForegroundColor Green
    Write-Host "       Thumbprint: $($newCert.Thumbprint)"

    # 3. Export public key only to temp CER file
    $tempCer = Join-Path $env:TEMP ("replayfix-dev-" + [System.Guid]::NewGuid().ToString("N") + ".cer")
    Export-Certificate -Cert $newCert -FilePath $tempCer | Out-Null

    # 4. Import public key into TrustedPublisher store
    Import-Certificate -CertStoreLocation "Cert:\CurrentUser\TrustedPublisher" -FilePath $tempCer | Out-Null
    Write-Host "  [OK] Registered public certificate in Cert:\CurrentUser\TrustedPublisher" -ForegroundColor Green

    # 5. Clean up temporary file
    if (Test-Path $tempCer) {
        Remove-Item $tempCer -Force -ErrorAction SilentlyContinue
    }

    Write-Host "[ReplayFix] Setup complete! Local development binaries will now run without SAC/WDAC blocks." -ForegroundColor Green
    exit 0
} catch {
    Write-Error "[ReplayFix] Failed to set up local certificate: $_"
    exit 1
}

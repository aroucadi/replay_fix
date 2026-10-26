<#
.SYNOPSIS
    Cargo test and execution runner for Windows MSVC targets in ReplayFix.

.DESCRIPTION
    Invoked by Cargo when executing test binaries or 'cargo run' on Windows.
    - If a valid developer certificate ('CN=ReplayFix Local Developer') is present,
      it automatically signs the binary before execution to prevent Smart App Control (SAC)
      or Windows Defender Application Control (WDAC) blocks.
    - If no certificate is present, it executes the binary directly (standard Windows / non-SAC).
    - If execution is blocked by Windows Code Integrity / SAC (os error 4551), it catches
      the block and outputs an actionable, clear diagnostic message instructing the developer
      how to bootstrap the local dev certificate with 'scripts\setup-dev-cert.ps1'.
    - Accurately forwards all command-line arguments and preserves the exit code.
#>

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Executable,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Arguments
)

$SubjectName = "CN=ReplayFix Local Developer"

# 1. Attempt to sign with local developer certificate if present
try {
    $cert = Get-ChildItem -Path "Cert:\CurrentUser\My" -CodeSigningCert -ErrorAction SilentlyContinue |
        Where-Object { ($_.Subject -eq $SubjectName -or $_.Subject -like "*$SubjectName*") -and ($_.NotAfter -gt (Get-Date)) } |
        Sort-Object NotAfter -Descending |
        Select-Object -First 1

    if ($cert) {
        # Check if already signed with a valid signature to avoid redundant signing
        $currentSig = Get-AuthenticodeSignature -FilePath $Executable -ErrorAction SilentlyContinue
        if (-not $currentSig -or $currentSig.Status -eq "NotSigned") {
            Set-AuthenticodeSignature -FilePath $Executable -Certificate $cert | Out-Null
        }
    }
} catch {
    # Non-fatal: if signing fails (e.g. read-only filesystem), continue and attempt direct execution
}

# 2. Execute binary with all forwarded arguments
$exitCode = 0
try {
    if ($Arguments -and $Arguments.Length -gt 0) {
        & $Executable @Arguments
    } else {
        & $Executable
    }
    $exitCode = $LASTEXITCODE
} catch {
    $errMessage = $_.Exception.Message
    # Detect Windows Smart App Control / WDAC policy block
    if ($errMessage -like "*Application Control*blocked*" -or $errMessage -like "*4551*" -or $_.FullyQualifiedErrorId -like "*NativeCommandFailed*") {
        Write-Host ""
        Write-Host "=========================================================================================" -ForegroundColor Red
        Write-Host " [ReplayFix Security Notice] Execution Blocked by Windows Application Control (SAC)       " -ForegroundColor Yellow
        Write-Host "=========================================================================================" -ForegroundColor Red
        Write-Host "Windows Smart App Control or Code Integrity policy blocked this development binary:" -ForegroundColor White
        Write-Host "  $Executable" -ForegroundColor Gray
        Write-Host ""
        Write-Host "To allow local development binaries to run smoothly on this Windows machine:" -ForegroundColor Cyan
        Write-Host "  1. Run the one-time local certificate setup script:" -ForegroundColor White
        Write-Host "     powershell -ExecutionPolicy Bypass -File .\scripts\setup-dev-cert.ps1" -ForegroundColor Green
        Write-Host "  2. Rerun your Cargo command." -ForegroundColor White
        Write-Host "=========================================================================================" -ForegroundColor Red
        Write-Host ""
        exit 126
    }

    # Re-throw any other unexpected error
    throw $_
}

exit $exitCode

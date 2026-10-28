param(
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Find-PanicViolationsInSource {
    param(
        [string]$Source,
        [string]$FilePath
    )

    $results = New-Object System.Collections.Generic.List[string]
    $tokens = New-Object System.Collections.Generic.List[PSObject]

    $len = $Source.Length
    $i = 0
    $line = 1

    while ($i -lt $len) {
        $ch = $Source[$i]

        # Track newlines
        if ($ch -eq "`n") {
            $line++
            $i++
            continue
        }
        if ($ch -eq "`r") {
            $i++
            continue
        }

        # Skip whitespace
        if ([char]::IsWhiteSpace($ch)) {
            $i++
            continue
        }

        # Line comment //
        if ($ch -eq '/' -and ($i + 1) -lt $len -and $Source[$i + 1] -eq '/') {
            $i += 2
            while ($i -lt $len -and $Source[$i] -ne "`n") {
                $i++
            }
            continue
        }

        # Nested block comment /* ... */
        if ($ch -eq '/' -and ($i + 1) -lt $len -and $Source[$i + 1] -eq '*') {
            $depth = 1
            $i += 2
            while ($i -lt $len -and $depth -gt 0) {
                if ($Source[$i] -eq "`n") {
                    $line++
                    $i++
                } elseif ($Source[$i] -eq '/' -and ($i + 1) -lt $len -and $Source[$i + 1] -eq '*') {
                    $depth++
                    $i += 2
                } elseif ($Source[$i] -eq '*' -and ($i + 1) -lt $len -and $Source[$i + 1] -eq '/') {
                    $depth--
                    $i += 2
                } else {
                    $i++
                }
            }
            continue
        }

        # Raw string literal r#*"..."#* or br#*"..."#*
        if (($ch -eq 'r' -or ($ch -eq 'b' -and ($i + 1) -lt $len -and $Source[$i + 1] -eq 'r'))) {
            $scan = if ($ch -eq 'b') { $i + 2 } else { $i + 1 }
            $hashes = 0
            while ($scan -lt $len -and $Source[$scan] -eq '#') {
                $hashes++
                $scan++
            }
            if ($scan -lt $len -and $Source[$scan] -eq '"') {
                $scan++
                while ($scan -lt $len) {
                    if ($Source[$scan] -eq "`n") {
                        $line++
                        $scan++
                    } elseif ($Source[$scan] -eq '"') {
                        $scan++
                        $matchedHashes = 0
                        while ($matchedHashes -lt $hashes -and $scan -lt $len -and $Source[$scan] -eq '#') {
                            $matchedHashes++
                            $scan++
                        }
                        if ($matchedHashes -eq $hashes) {
                            break
                        }
                    } else {
                        $scan++
                    }
                }
                $i = $scan
                $tokens.Add([PSCustomObject]@{ Kind = "Literal"; Text = '""'; Line = $line })
                continue
            }
        }

        # Normal string literal "..."
        if ($ch -eq '"') {
            $i++
            while ($i -lt $len) {
                if ($Source[$i] -eq "`n") {
                    $line++
                    $i++
                } elseif ($Source[$i] -eq '\') {
                    $i += 2
                } elseif ($Source[$i] -eq '"') {
                    $i++
                    break
                } else {
                    $i++
                }
            }
            $tokens.Add([PSCustomObject]@{ Kind = "Literal"; Text = '""'; Line = $line })
            continue
        }

        # Char literal vs lifetime: 'x' or '\...'
        if ($ch -eq '''') {
            if (($i + 1) -lt $len -and $Source[$i + 1] -eq '\') {
                $i += 2
                while ($i -lt $len -and $Source[$i] -ne '''' -and $Source[$i] -ne "`n") {
                    $i++
                }
                if ($i -lt $len -and $Source[$i] -eq '''') { $i++ }
                $tokens.Add([PSCustomObject]@{ Kind = "Literal"; Text = "''"; Line = $line })
                continue
            } elseif (($i + 2) -lt $len -and $Source[$i + 2] -eq '''' -and $Source[$i + 1] -ne "`n") {
                $i += 3
                $tokens.Add([PSCustomObject]@{ Kind = "Literal"; Text = "''"; Line = $line })
                continue
            }
            $i++
            continue
        }

        # Attribute #[...] or #![...]
        if ($ch -eq '#' -and ($i + 1) -lt $len -and ($Source[$i + 1] -eq '[' -or ($Source[$i + 1] -eq '!' -and ($i + 2) -lt $len -and $Source[$i + 2] -eq '['))) {
            $attrLine = $line
            $start = $i
            $bracketDepth = 0
            while ($i -lt $len) {
                if ($Source[$i] -eq "`n") { $line++ }
                if ($Source[$i] -eq '[') { $bracketDepth++ }
                elseif ($Source[$i] -eq ']') {
                    $bracketDepth--
                    if ($bracketDepth -eq 0) {
                        $i++
                        break
                    }
                }
                $i++
            }
            $attrText = $Source.Substring($start, $i - $start)
            $tokens.Add([PSCustomObject]@{ Kind = "Attr"; Text = $attrText; Line = $attrLine })
            continue
        }

        # Identifier
        if ([char]::IsLetter($ch) -or $ch -eq '_') {
            $start = $i
            $tokLine = $line
            $i++
            while ($i -lt $len -and ([char]::IsLetterOrDigit($Source[$i]) -or $Source[$i] -eq '_')) {
                $i++
            }
            $ident = $Source.Substring($start, $i - $start)
            $tokens.Add([PSCustomObject]@{ Kind = "Ident"; Text = $ident; Line = $tokLine })
            continue
        }

        # Single-character punctuation token
        $tokens.Add([PSCustomObject]@{ Kind = "Punct"; Text = [string]$ch; Line = $line })
        $i++
    }

    # Walk token stream, tracking brace depth and #[cfg(test)] items accurately
    $braceDepth = 0
    $pendingTestDepth = -1
    $inTestBlock = $false
    $testBaseDepth = -1
    $count = $tokens.Count

    for ($t = 0; $t -lt $count; $t++) {
        $tok = $tokens[$t]

        if ($tok.Kind -eq "Attr") {
            $compactAttr = ($tok.Text -replace '\s+', '')
            if (-not $inTestBlock -and $compactAttr -match '^#\[cfg\((test|all\(.*test.*\))\)\]$') {
                $pendingTestDepth = $braceDepth
            }
            continue
        }

        if ($tok.Kind -eq "Punct") {
            if ($tok.Text -eq '{') {
                if ($pendingTestDepth -ge 0 -and -not $inTestBlock -and $braceDepth -eq $pendingTestDepth) {
                    $inTestBlock = $true
                    $testBaseDepth = $braceDepth
                    $pendingTestDepth = -1
                }
                $braceDepth++
                continue
            }
            if ($tok.Text -eq '}') {
                if ($braceDepth -gt 0) { $braceDepth-- }
                if ($inTestBlock -and $braceDepth -le $testBaseDepth) {
                    $inTestBlock = $false
                    $testBaseDepth = -1
                }
                continue
            }
            if ($tok.Text -eq ';') {
                # A #[cfg(test)] attribute on a semicolon item (e.g. `#[cfg(test)] use foo;`)
                # ends at `;` without opening a `{` block.
                if ($pendingTestDepth -ge 0 -and -not $inTestBlock -and $braceDepth -eq $pendingTestDepth) {
                    $pendingTestDepth = -1
                }
                continue
            }
        }

        if ($inTestBlock -or $pendingTestDepth -ge 0) {
            continue
        }

        # Check for `.unwrap(`, `.expect(`, `.unwrap_err(`, `.expect_err(`, `.unwrap_unchecked(`
        if ($tok.Kind -eq "Punct" -and $tok.Text -eq '.' -and ($t + 2) -lt $count) {
            $next1 = $tokens[$t + 1]
            $next2 = $tokens[$t + 2]
            if ($next1.Kind -eq "Ident" -and $next1.Text -in @("unwrap", "expect", "unwrap_err", "expect_err", "unwrap_unchecked") -and $next2.Kind -eq "Punct" -and $next2.Text -eq '(') {
                $results.Add("${FilePath}:$($next1.Line): .$($next1.Text)()")
            }
        }

        # Check for `panic!(`, `todo!(`, `unimplemented!(`, `unreachable!(`
        if ($tok.Kind -eq "Ident" -and $tok.Text -in @("panic", "todo", "unimplemented", "unreachable") -and ($t + 2) -lt $count) {
            $next1 = $tokens[$t + 1]
            $next2 = $tokens[$t + 2]
            if ($next1.Kind -eq "Punct" -and $next1.Text -eq '!' -and $next2.Kind -eq "Punct" -and ($next2.Text -eq '(' -or $next2.Text -eq '[' -or $next2.Text -eq '{')) {
                $results.Add("${FilePath}:$($tok.Line): $($tok.Text)!()")
            }
        }

        # Check for `panic_any(`
        if ($tok.Kind -eq "Ident" -and $tok.Text -eq "panic_any" -and ($t + 1) -lt $count) {
            $next1 = $tokens[$t + 1]
            if ($next1.Kind -eq "Punct" -and $next1.Text -eq '(') {
                $results.Add("${FilePath}:$($tok.Line): panic_any()")
            }
        }
    }

    return ,$results.ToArray()
}

if ($SelfTest) {
    # 1. Semicolon #[cfg(test)] item followed by production unwrap must be caught
    $case1 = @"
#[cfg(test)]
use std::collections::BTreeMap;

pub fn prod_after_cfg_use(x: Option<i32>) -> i32 {
    x.unwrap()
}
"@
    $v1 = Find-PanicViolationsInSource -Source $case1 -FilePath "case1.rs"
    if ($v1.Count -ne 1) {
        Write-Error "SelfTest case 1 failed: expected 1 violation, got $($v1.Count)"
        exit 1
    }

    # 2. Braces inside string literals / comments inside #[cfg(test)] followed by production code
    $case2 = @"
#[cfg(test)]
mod tests {
    const BRACE_STR: &str = "}}}}}";
    // unmatched closing brace in comment: }
    /* nested /* comment } */ still comment } */
    #[test]
    fn t() {
        let a: Option<i32> = Some(1);
        assert_eq!(a.unwrap(), 1);
    }
}

pub fn prod_after_test_mod(y: Result<i32, &str>) -> i32 {
    y.expect("must be caught")
}
"@
    $v2 = Find-PanicViolationsInSource -Source $case2 -FilePath "case2.rs"
    if ($v2.Count -ne 1 -or $v2[0] -notmatch "\.expect\(\)") {
        Write-Error "SelfTest case 2 failed: expected 1 .expect() violation, got $($v2 -join ', ')"
        exit 1
    }

    # 3. Multiline .unwrap(), .unwrap_err(), panic!(), todo!(), unimplemented!(), unreachable!()
    $case3 = @"
pub fn prod_multiline(x: Result<i32, i32>) {
    let _ = x
        // comment between receiver and dot
        .
        unwrap_err();
    panic ! (
        "boom"
    );
    todo!();
    unimplemented!();
    unreachable!();
}
"@
    $v3 = Find-PanicViolationsInSource -Source $case3 -FilePath "case3.rs"
    if ($v3.Count -ne 5) {
        Write-Error "SelfTest case 3 failed: expected 5 violations, got $($v3.Count): $($v3 -join ', ')"
        exit 1
    }

    Write-Host "SelfTest passed: all 3 adversarial panic-scanner fixtures verified."
    exit 0
}

$repoRoot = $PSScriptRoot
while ($repoRoot -and -not (Test-Path (Join-Path $repoRoot "Cargo.toml"))) {
    $parent = Split-Path -Parent $repoRoot
    if ($parent -eq $repoRoot) { break }
    $repoRoot = $parent
}
if (-not $repoRoot -or -not (Test-Path (Join-Path $repoRoot "Cargo.toml"))) {
    $repoRoot = (Get-Location).Path
}
$cratesDir = Join-Path $repoRoot "crates"
$files = Get-ChildItem -Path $cratesDir -Filter "*.rs" -Recurse
$violations = 0

foreach ($file in $files) {
    if ($file.FullName -match "\\tests\\") { continue }
    $source = [System.IO.File]::ReadAllText($file.FullName)
    $fileViolations = Find-PanicViolationsInSource -Source $source -FilePath $file.FullName
    foreach ($v in $fileViolations) {
        Write-Host $v
        $violations++
    }
}

Write-Host "Total non-test violations: $violations"
if ($violations -gt 0) {
    exit 1
}
exit 0

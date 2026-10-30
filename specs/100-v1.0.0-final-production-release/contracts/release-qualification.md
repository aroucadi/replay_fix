# Contract: Release Qualification & Final Submission Package

## 1. Mandatory Quality Gates
1. `powershell -ExecutionPolicy Bypass -File scripts/check-panics.ps1` -> `Total non-test violations: 0`
2. `cargo fmt --all -- --check` -> Exit `0`
3. `cargo check --workspace --all-targets --all-features` -> Exit `0`
4. `cargo clippy --workspace --all-targets --all-features -- -D warnings` -> Exit `0` (0 warnings)
5. `cargo test --workspace` -> 100% pass rate
6. `cargo build --release --workspace` -> Exit `0`
7. `Get-AuthenticodeSignature release/v1.0.0/binaries/windows-x64/replayfix.exe` -> `Status: Valid`
8. `SHA256SUMS.txt` verification -> 100% match across all packaged files

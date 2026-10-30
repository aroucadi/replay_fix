# Contract: Production Release Build (`v1.0.0`)

## 1. Cargo Release Profile
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
```

## 2. Build & Safety Invariants
1. **Zero Non-Test Panics**: `scripts/check-panics.ps1` MUST report `Total non-test violations: 0` across all `crates/*/src/**/*.rs`.
2. **Atomic File Persistence**: Every file written by `replayfix` uses two-phase `.tmp` -> `fs::rename`, ensuring `panic = "abort"` can never corrupt repository state or receipts.
3. **Target Deliverable**: `target/release/replayfix.exe` (`x86_64-pc-windows-msvc`), reporting `replayfix 1.0.0`.

# ReplayFix

Deterministic Rust-based verification harness for visual bug reproduction, causal verification, constrained repair, regression testing, and cryptographic audit reporting for modern web applications.

## Status: v0.0.0 (Workspace Genesis)

ReplayFix is being developed under a clean-room protocol for the Meta Global AI Developer Hackathon. The codebase is structured as a vertical micro-slice pipeline, ensuring every commit compiles, passes tests, and delivers verifiable empirical evidence.

Current maturity: **v0.0.0 Baseline** (toolchain lock-in, 2-crate architecture, and platform abstractions established).

## Architecture

ReplayFix enforces a strict 2-crate architecture:

- **`crates/replayfix-core`**: Core library containing platform abstractions, error taxonomies, and upcoming verification oracles and perception adapters.
- **`crates/replayfix-cli`**: Command-line binary (`replayfix`) providing environment diagnostics, progress rendering, and headed replay execution.

## Prerequisites & Toolchain

- **Rust**: `1.80+` (compiled and verified on `1.99.1`)
- **Native Host Toolchain**:
  - Windows: MSVC C++ Build Tools (`x86_64-pc-windows-msvc`)
  - macOS / Linux: Supported native toolchain
- **Git**: `2.30+`

## Verification

Verify the baseline workspace using native Cargo tooling:

```bash
# Check compilation across both crates
cargo check --workspace

# Run core unit tests
cargo test --workspace

# Run CLI version smoke test
cargo run --release -p replayfix-cli -- --version

# Run initial environment diagnostics stub
cargo run --release -p replayfix-cli -- doctor
```

## License

Licensed under the [MIT License](LICENSE.md).

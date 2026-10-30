# Contract: Release Qualification & Verification Matrix

## 1. Scope & Verification Criteria
Every release candidate for v0.9.5 must satisfy the following gates prior to tagging:
1. **Zero-Panic Guarantee**: `powershell -ExecutionPolicy Bypass -File scripts/check-panics.ps1` returns 0 violations.
2. **Code Hygiene**: `cargo fmt --all -- --check` clean, `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
3. **Workspace Tests**: 100% pass rate on `cargo test --workspace`.
4. **Portal Conformance**: `proof_viewer.html` loads offline in standard browsers without external network calls, correctly verifies valid and tampered receipts, and renders Merkle tree, multimodal evidence, diffs, and sweeps.
5. **Gateway Resilience**: Unit and integration tests verify token-bucket rate limits, circuit breaker transitions, and fallback switching under simulated outages.

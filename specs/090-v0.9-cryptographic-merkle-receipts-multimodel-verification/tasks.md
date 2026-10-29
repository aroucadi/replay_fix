# 📋 Tasks: Spec 090 — Cryptographic Merkle Receipts, Ed25519 Attestation, Five-Bug Sweep & Tri-Model Verification

## Phase 1: Cryptographic Merkle Tree Engine
- [x] T090-01: Add `ed25519-dalek` dependency to `crates/replayfix-core/Cargo.toml`.
- [x] T090-02: Implement `crates/replayfix-core/src/merkle.rs` with RFC 6962 domain-separated hashing (`0x00` leaf, `0x01` internal).
- [x] T090-03: Implement `MerkleTree` construction, root calculation, and `MerkleInclusionProof` generation.
- [x] T090-04: Implement `verify_inclusion_proof` and unit test against known vectors, empty trees, odd/even leaf counts, and tampered proofs.

## Phase 2: Ed25519 Attestation & Receipt Verifier
- [x] T090-05: Implement `crates/replayfix-core/src/crypto.rs` with key generation, signing, and verification.
- [x] T090-06: Define `AttestationRecord` and signing payload binding Merkle root, schema version `"0.9"`, and timestamp.
- [x] T090-07: Wire Merkle receipt and Ed25519 attestation into `AuditReceipt::write_disaggregated_artifacts` emitting `merkle_receipt.json`.
- [x] T090-08: Implement CLI subcommand `verify-receipt` in `crates/replayfix-cli` with `--trusted-key` flag and exit code 40 on failure.

## Phase 3: Authoritative Five-Bug Fixtures Suite
- [x] T090-09: Implement `RF-02` (Stale Discount Calculation) fixture in `crates/replayfix-core/src/fixtures.rs`.
- [x] T090-10: Implement `RF-03` (Inventory Underflow) fixture in `crates/replayfix-core/src/fixtures.rs`.
- [x] T090-11: Implement `RF-04` (Cart State Reset on Tab Switch) fixture in `crates/replayfix-core/src/fixtures.rs`.
- [x] T090-12: Implement `RF-05` (Double Submit Idempotency) fixture in `crates/replayfix-core/src/fixtures.rs`.
- [x] T090-13: Update `Fixture::supported_tasks()` to return `&["RF-01", "RF-02", "RF-03", "RF-04", "RF-05"]`.

## Phase 4: Tri-Model Orchestration & Sweep Engine
- [x] T090-14: Implement `ModelPool` in `crates/replayfix-core/src/repair.rs` for concurrent candidate synthesis with cancellation.
- [x] T090-15: Implement competitive evaluation selecting the first passing verified patch and aborting competing tasks.
- [x] T090-16: Implement `replayfix sweep` subcommand in `crates/replayfix-cli` executing `RF-01` through `RF-05` in isolated sandboxes.
- [x] T090-17: Emit `.replayfix/sweep_receipt.json` with overall Merkle root, per-bug leaves, and sub-60s execution telemetry.

## Phase 5: Verification & Quality Gates
- [x] T090-18: Add integration tests in `crates/replayfix-cli/tests/cli.rs` for `verify-receipt` and `sweep`.
- [x] T090-19: Verify zero panics via `scripts/check-panics.ps1`.
- [x] T090-20: Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] T090-21: Run `cargo test --workspace` and assert 100% pass rate.
- [x] T090-22: Update documentation (`CHANGELOG.md`, `docs/release-notes/v0.9.0.md`, `docs/agent-state.md`).
- [x] T090-23: Bump crate versions to `0.9.0`.

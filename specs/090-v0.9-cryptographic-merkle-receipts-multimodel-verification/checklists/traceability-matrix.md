# Traceability Matrix: Spec 090 Requirements to Tests & Evidence

| Requirement | Implementation Module | Automated Tests | Evidence Artifact |
|---|---|---|---|
| **REQ-090-01**: RFC 6962 Leaf & Node Domain Separation | `replayfix_core::merkle` | `merkle::tests::test_domain_separation` | `merkle_receipt.json` |
| **REQ-090-02**: RFC 6962 Inclusion Proof Generation & Verification | `replayfix_core::merkle` | `merkle::tests::test_inclusion_proofs_and_tamper_detection` | `merkle_receipt.json` |
| **REQ-090-03**: Pure Ed25519 Signing & Payload Binding | `replayfix_core::crypto` | `crypto::tests::test_sign_and_verify_roundtrip` | `audit_receipt.json` |
| **REQ-090-04**: Pinned Key Trust Policy Evaluation | `replayfix_core::crypto` | `crypto::tests::test_trust_policy_pinned_keys` | `verify_receipt` output |
| **REQ-090-05**: Verifier CLI `verify-receipt` | `replayfix_cli::commands` | `tests/cli.rs::test_verify_receipt_valid_and_tampered` | CLI exit code 0 / 40 |
| **REQ-090-06**: Five-Bug Fixture Suite (`RF-01`..`RF-05`) | `replayfix_core::fixtures` | `fixtures::tests::test_all_five_fixtures` | Fixtures in memory / test |
| **REQ-090-07**: Tri-Model Pooling with Cancellation | `replayfix_core::repair` | `repair::tests::test_model_pool_first_verified_green` | `RepairTelemetry` |
| **REQ-090-08**: Benchmark Sweep CLI `replayfix sweep` | `replayfix_cli::commands` | `tests/cli.rs::test_sweep_all_five_bugs` | `sweep_receipt.json` |
| **REQ-090-09**: Sub-60s Sweep Performance Target | `replayfix_cli::sweep` | `tests/cli.rs::test_sweep_latency_budget` | `elapsed_wall_clock_ms` |

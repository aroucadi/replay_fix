# Checklist: Spec 090 Release Qualification Gate

- [x] RFC 6962 leaf hashing: `SHA-256(0x00 || leaf)`
- [x] RFC 6962 internal hashing: `SHA-256(0x01 || left || right)`
- [x] RFC 6962 empty tree handling
- [x] Merkle inclusion proof generation for all leaves
- [x] Merkle inclusion proof verification rejecting altered paths or leaves
- [x] Pure Ed25519 signing over versioned attestation payload
- [x] Ed25519 signature verification rejecting modified payloads or wrong keys
- [x] `replayfix verify-receipt` CLI implementation with `--trusted-key` policy
- [x] Authoritative `RF-01` through `RF-05` fixtures implemented
- [x] Tri-model orchestration with concurrent dispatch and cancellation propagation
- [x] First-verified-green acceptance rule preventing model consensus bypass
- [x] `replayfix sweep` CLI implementation running all 5 bugs in isolation
- [x] Sweep execution under 60 seconds wall-clock time
- [x] Zero non-test panics (`scripts/check-panics.ps1`)
- [x] Zero Clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`)
- [x] 100% test pass rate (`cargo test --workspace`)
- [x] Clean working tree and remote tag verification

# Release Checklist: Spec 100 (v1.0.0)

- [x] `v0.9.5` baseline commit (`81d5c64`) verified clean
- [x] Historical specifications (`000` through `095`) audited and consolidated
- [x] Spec 100 (`specs/100-v1.0.0-final-production-release/`) specified, planned, and tasked
- [x] Workspace versions bumped to `1.0.0`
- [x] `README.md`, `CHANGELOG.md`, `docs/release-notes/v1.0.0.md`, and `docs/agent-state.md` updated
- [x] Production binary compiled (`cargo build --release --workspace`) and signed (`scripts/sign-binary.ps1`)
- [x] Real Act 1, Act 2 (`Exit 30` & `Exit 11`), and Act 3 (`Exit 0` repair & 5-bug sweep) executed with signed `1.0.0` binary
- [x] Ephemeral private keys purged from `release/v1.0.0/examples/sanitized-receipts/`
- [x] 180.0-second `1920x1080` 30fps H.264/AAC video (`replayfix-v1.0.0-demo.mp4`), `.vtt` captions, and `demo-evidence-manifest.json` generated
- [x] `release/v1.0.0/SHA256SUMS.txt` generated and verified across all packaged files
- [x] Zero non-test panics (`scripts/check-panics.ps1`), 0 Clippy warnings, clean `cargo fmt`, 100% tests passing

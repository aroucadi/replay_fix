# Release Checklist: ReplayFix v0.8.0

- [ ] All unit and integration tests pass (`cargo test --workspace`).
- [ ] No Clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- [ ] Code formatted (`cargo fmt --check`).
- [ ] Zero unwrap check passes (`powershell -File scripts/check-panics.ps1`).
- [ ] SpecKit documents complete (`spec.md`, `plan.md`, `tasks.md`, `data-model.md`).
- [ ] JSON contracts match test payloads.
- [ ] Release notes written in `docs/release-notes/v0.8.0.md`.
- [ ] `CHANGELOG.md` updated with v0.8.0 entry.
- [ ] Git commit and tag `v0.8.0` created and pushed.

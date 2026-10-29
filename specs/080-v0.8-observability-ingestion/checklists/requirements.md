# Requirements Checklist: ReplayFix v0.8.0

- [ ] **FR-001**: `--trace` CLI flag accepted on `replayfix repro` and `replayfix run`.
- [ ] **FR-002**: Playwright trace zip file correctly generated and saved to `.replayfix/artifacts/`.
- [ ] **FR-003**: Playwright trace viewer launched via `npx playwright show-trace` if interactive and trace is enabled; skipped in CI or non-TTY.
- [ ] **FR-004**: `--issue <url>` CLI option accepted and parsed.
- [ ] **FR-005**: Strict URL validation for `https://github.com/:owner/:repo/issues/:id`, rejecting pull requests, commit hashes, or invalid schemes.
- [ ] **FR-006**: Issue payload fetched with timeout and rate limit mapping.
- [ ] **FR-007**: Step-to-reproduce extraction from issue markdown body.
- [ ] **FR-008**: Unified diff syntax highlighting rendered cleanly in terminal.
- [ ] **FR-009**: Interactive approval prompt requiring affirmative confirmation before modifying base repo; `--yes` flag support; fail-closed on non-interactive.
- [ ] **FR-010**: Disaggregated artifacts written atomically to `.replayfix/artifacts/`.

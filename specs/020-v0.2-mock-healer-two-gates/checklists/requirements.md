# Specification Quality Checklist: 020-v0.2-mock-healer-two-gates

- [x] Scope statement separates simulated control flow from real perception
- [x] Every user story has an independent test and acceptance scenarios
- [x] Requirements are testable and numbered (FR-001 … FR-012)
- [x] Success criteria are measurable (SC-001 … SC-005)
- [x] Edge cases cover unknown task, bad patch file, unsafe output dir, CRLF
- [x] Exit codes align with canonical result schemas (0/10/11/30) and the v0.1 usage code (2)
- [x] No requirement depends on unimplemented capabilities (browser, models, worktrees)
- [x] Out-of-scope items named with owning spec (030, 040, 070)
- [x] No absolute local paths or external workspace references

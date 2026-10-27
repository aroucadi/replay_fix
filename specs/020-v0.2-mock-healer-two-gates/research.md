# Research & Decisions: 020-v0.2-mock-healer-two-gates

## D1 — Where does `--mock-heal` live?
**Decision**: A flag on the existing `repro` command, not a new `run` subcommand.
**Rationale**: The project rules fix `replayfix repro` as the CLI entry point and the manifest-aligned flags already exist. A second entry point would duplicate manifest ingestion.
**Rejected**: `replayfix run --bug RF-01 --mock-heal` (appears in early planning notes) — would fork ingestion and break the "strict alignment with `task_manifest`" rule.

## D2 — What does the simulated oracle check?
**Decision**: Static inspection of the fixture source for the unguarded `cart.items.length` dereference, producing a four-event stream.
**Rationale**: Deterministic, dependency-free, and honest about being a stand-in. A JavaScript interpreter would be disproportionate and misleading.

## D3 — Patch semantics
**Decision**: Single-file, exact-offset, exact-context unified diffs; no fuzz, no offset search.
**Rationale**: Fail-closed determinism. A repair that only applies "approximately" must not be verifiable.

## D4 — Canonical contract digest
**Decision**: Recursively sorted-key compact JSON (UTF-16 key order), integers/strings/bools/null/arrays/objects only, hashed with SHA-256.
**Rationale**: Matches RFC 8785 for the value types the contract uses without depending on `serde_json` map ordering (which changes if any crate enables `preserve_order`). Floats are rejected rather than mis-serialized.

## D5 — Exit codes
**Decision**: Reuse the canonical sets: `0` verified, `10` general failure (Gate 1 not RED, contract mismatch), `11` security violation, `30` Gate 2 / repair rejection, `2` usage error (existing v0.1 convention).

## D6 — Stale artifact handling
**Decision**: Each run removes the opposite terminal artifact before returning.
**Rationale**: A rejected run must never leave a previous success on disk that tooling could mistake for the current result.

## D7 — Technical debt removed in this release
- `scripts/cargo-runner.ps1`: no references remain after v0.1.1 removed the Cargo runner.
- Unused dependencies, confirmed by source search and a clean build after removal.
- Hardcoded `0.1.1` version literals in the CLI.
- Mojibake in `docs/windows-code-signing.md`.

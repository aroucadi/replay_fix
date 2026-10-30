# Implementation Plan: Spec 095 — Judge Proof Portal, Cryptographic Evidence Verification, Live Multimodal Cloud Gateway & Bounded Tri-Model Orchestration

**Branch**: `main` (`095-v0.9.5-judge-proof-portal-live-multimodal-gateway`) | **Date**: 2026-10-30 | **Spec**: [spec.md](spec.md)  
**Input**: Feature specification from `specs/095-v0.9.5-judge-proof-portal-live-multimodal-gateway/spec.md`

---

## Summary

Deliver **ReplayFix v0.9.5**, extending the independently qualified `v0.9.1` foundation (`2988bc3`) across three tightly coupled deliverables:
1. **Offline Judge Proof Portal (`proof_viewer.html`)**: A 100% self-contained, single-file HTML5/CSS3/ES2022 application with zero external network or CDN dependencies (`default-src 'none'`). It loads `audit_receipt.json`, `merkle_receipt.json`, and `sweep_receipt.json` offline (`file://`), executes client-side RFC 8785 UTF-16 canonical JSON serialization, RFC 6962 SHA-256 leaf (`0x00`) and internal node (`0x01`) Merkle hashing via Web Crypto (`SubtleCrypto`), pure RFC 8032 Ed25519 signature verification, interactive Merkle tree exploration, multimodal video/DOM/SAM 3.1 coordinate overlays, and syntax-highlighted unified diffs with active regression parity ($F_{	ext{patched}} \setminus F_{	ext{baseline}} = \emptyset$).
2. **Live Multimodal Cloud Gateway (`crates/replayfix-core/src/gateway.rs`)**: A bounded, observable provider gateway supporting Meta Model API (`https://api.meta.ai/v1/responses` across `muse-wonder-1.0`, `muse-spark-1.3`, `sam-3-1`, and `muse-voice-transcribe-1.0`), Baseten open-weight fallback (`https://inference.baseten.co/v1` with `meta-models/muse-spark-open`), and deterministic offline mock adapters. Enforces token-bucket rate limiting (RPM/TPM), HTTP 429 `Retry-After` + exponential backoff with jitter, a 3-state circuit breaker (`Closed`, `Open`, `HalfOpen`), the Zero-Audio Rule on text tasks, and live-vs-fixture provenance.
3. **CLI Export & Adversarial Qualification (`replayfix export-proof` / `proof_viewer.html` tests)**: CLI support to export or bundle self-contained proof packages and automated Playwright / Rust verification tests proving XSS immunity, tamper detection, offline execution, and gateway resilience.

---

## Technical Context

- **Language/Version**: Rust 2021 Edition ($\ge 1.80$, tested on `rustc 1.99.1`) + Vanilla ES2022 / HTML5 / CSS3 in `proof_viewer.html`
- **Primary Dependencies**: `serde`, `serde_json`, `sha2`, `ed25519-dalek`, `chrono`, `thiserror`, `tokio`, `clap` (zero new unvetted crates required)
- **Storage**: Ephemeral scratch worktrees + atomic filesystem artifacts (`ArtifactStore` / `atomic_write_file`)
- **Testing**: `cargo test --workspace`, `scripts/check-panics.ps1`, `cargo clippy -- -D warnings`, plus automated verification of `proof_viewer.html`
- **Target Platform**: Windows 11 (MSVC), macOS, Linux, and modern offline desktop/mobile browsers (`file://` and static hosting)
- **Project Type**: Rust Workspace (`replayfix-core` + `replayfix-cli`) + Single-File Offline Web Portal (`proof_viewer.html`)
- **Performance Goals**: Portal initial load $< 100	ext{ms}$ offline; receipt verification $< 50	ext{ms}$; 5-bug benchmark sweep $< 60	ext{s}$
- **Constraints**: Air-gapped browser operation (`connect-src 'none'`); max upload $50	ext{MB}$; zero `.unwrap()`/`.expect()` in non-test Rust code; strict $\Delta	ext{repo}=0$

---

## Constitution Check

*GATE: Evaluated before Phase 0 and re-verified after Phase 1 design.*

| Constitutional Law | Enforcement in Spec 095 | Gate Status |
|---|---|---|
| **Law I (Substrate Primacy)** | Portal and gateway never certify repairs from model claims; only deterministic Gate 1/Gate 2 + regression parity qualify a fix. | **PASS** |
| **Law II (Contract Invariance)** | RFC 8785 UTF-16 canonical JSON and SHA-256 contract digests preserved unchanged. | **PASS** |
| **Law III ($\Delta	ext{repo}=0$ Isolation)** | All sweep and repair runs execute in `ScratchSandbox` with pre/post `WorktreeManifest` checks. | **PASS** |
| **Law IV (Ephemeral Port `:0`)** | Inherited `SandboxServer` and `PlaywrightBridge` isolation preserved intact. | **PASS** |
| **Law V (Untrusted Input Fence)** | Portal renders all receipt fields via `textContent` / safe DOM nodes (`innerHTML` forbidden on untrusted data); gateway escapes untrusted context. | **PASS** |
| **Law VI (RFC 6962 & Ed25519 Receipts)** | Exact byte-level parity between `replayfix-core::merkle` / `crypto` and `proof_viewer.html` verifier. | **PASS** |
| **Law VII & VIII (Test Freeze & Regression Parity)** | `TestFreezeGuard`, `DiffGuard`, and $F_{	ext{patched}} \setminus F_{	ext{baseline}} = \emptyset$ verified and displayed in portal. | **PASS** |
| **Law IX & X (Exit Taxonomy & Zero-Panic)** | Predictable exit codes preserved; 0 non-test panics enforced via `scripts/check-panics.ps1`. | **PASS** |

---

## Project Structure

### Documentation (this feature)

```text
specs/095-v0.9.5-judge-proof-portal-live-multimodal-gateway/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── tasks.md
├── adrs/
│   ├── ADR-01-offline-portal-architecture.md
│   ├── ADR-02-receipt-parsing-and-verification.md
│   ├── ADR-03-multimodal-evidence-model.md
│   ├── ADR-04-cloud-gateway-and-provider-adapters.md
│   ├── ADR-05-rate-pooling-and-circuit-breakers.md
│   ├── ADR-06-audio-transcription-and-alignment.md
│   └── ADR-07-release-and-hosting.md
├── checklists/
│   └── requirements.md
├── contracts/
│   ├── proof-portal.md
│   ├── receipt-verification.md
│   ├── multimodal-evidence.md
│   ├── provider-gateway.md
│   ├── rate-limits-and-circuit-breakers.md
│   ├── audio-transcription.md
│   └── release-qualification.md
├── security/
│   ├── threat-model.md
│   └── adversarial-test-plan.md
└── verification/
    └── requirement-traceability.md
```

### Source Code (repository root)

```text
proof_viewer.html                        # Primary offline Judge Proof Portal deliverable
crates/
├── replayfix-core/
│   └── src/
│       ├── gateway.rs                   # Live Meta Multimodal Gateway, RateLimiter, CircuitBreaker, Fallback
│       ├── portal.rs                    # Portal bundle/verification helper & embedded sample receipts
│       └── lib.rs                       # Exports gateway and portal modules
└── replayfix-cli/
    ├── src/
    │   └── main.rs                      # CLI wiring for gateway telemetry & portal verification
    └── tests/
        └── cli.rs                       # End-to-end CLI, gateway, and portal verification tests
```

---

## Complexity Tracking

No constitutional violations introduced. Zero external crates added; `proof_viewer.html` uses zero external runtime dependencies.

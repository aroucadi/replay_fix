# 📚 ReplayFix Documentation Hub

Welcome to the official technical documentation for **ReplayFix** — the autonomous, repository-agnostic, deterministic Rust harness for visual bug reproduction, causal verification, constrained AST repair, and cryptographic audit reporting.

---

## 🧭 Navigation Matrix

| Guide | Description | Target Audience |
| :--- | :--- | :--- |
| [**Architecture & Two-Gate Oracle**](architecture.md) | Deep-dive into the Two-Gate counterfactual state machine, CDP spatial perception, and Meta Muse neural loop. | Core Contributors, Hackathon Judges |
| [**CLI & Operator Guide**](cli-reference.md) | Comprehensive command-line reference, arguments, environment configuration, and exit codes. | SREs, QA Engineers, Developers |
| [**Trust & Cryptographic Model**](trust-architecture.md) | RFC 6962 Merkle tree ledger, Ed25519 multi-signatures, and four-layer receipt verification. | Security Auditors, CSO |
| [**Engineering Principles (BeautifulRust)**](engineering-principles-rust.md) | The 10 Inviolable Laws, `#![forbid(unsafe_code)]`, zero-panic discipline, and async task hygiene. | Rust Developers |
| [**Windows Code Signing & WDAC**](windows-code-signing.md) | Authenticode signing, Smart App Control compatibility, and CI/CD release packaging. | Release Engineers, Devops |
| [**Agent State Machine**](agent-state.md) | Internals of the multi-turn agent execution lifecycle and state convergence. | AI Researchers, Core Devs |
| [**Release History & Changelogs**](release-notes/) | Complete version release notes spanning `v0.0.0` through `v2.0.0`. | All Users |

---

## ⚡ The ReplayFix Lifecycle at a Glance

```text
[Video .mp4 / RRWeb JSON]
           │
           ▼
  [Spatial Perception] ──▶ SAM 3.1 & CDP ariaSnapshot Coordinate Calibration
           │
           ▼
   [Gate 1: RED]       ──▶ Synthesizes Playwright Test (Must FAIL on unpatched codebase)
           │
           ▼
  [Meta Muse Wonder]   ──▶ Generates Bounded AST Search/Replace Patch (Δrepo = 0)
           │
           ▼
  [Gate 2: GREEN]      ──▶ 2-Run Stability Pass + Stress Quorum on Ephemeral Sandbox
           │
           ▼
  [RFC 6962 Notary]    ──▶ Merkle Receipt Appended to transparency_log.json & Signed (Ed25519)
```

---

## 💡 Key Architectural Invariants

1. **Substrate Primacy**: The deterministic Rust substrate (`crates/replayfix-core`) is the sole verification authority. Unverified model text never passes gates.
2. **Repository Immutability ($\Delta\text{repo}=0$)**: The host repository is strictly read-only. All execution, test generation, and repair patching occur inside isolated, ephemeral scratch clones on dynamic loopback ports (`:0`).
3. **Anti-Cheating Test Freeze**: Reproduction tests are hashed via SHA-256 before repair; any patch attempting to modify or relax test assertions is rejected with exit code `11` (`SECURITY_VIOLATION`).
4. **Zero-Panic Systems Discipline**: Non-test production code contains zero `.unwrap()`, `.expect()`, or `panic!()` invocations, strictly enforced via `.specify/scripts/check-panics.ps1`.

# ReplayFix Engineering Principles — Rust

**Status**: Authoritative Operational Charter  
**Version**: 1.0.0  
**Target Environment**: Dual-Crate Rust Harness (`crates/replayfix-core`, `crates/replayfix-cli`)

---

## 1. Substrate Primacy & The BeautifulRust Baseline

ReplayFix is a security-sensitive verification and repair harness for web applications. The Rust substrate serves as the sole judicial authority: no LLM response, prompt injection, or heuristic claim can declare an oracle satisfied or a bug verified.

All development in this repository adheres to the **BeautifulRust** discipline:
- **`rust-best-practices`**: The baseline guide for writing idiomatic, zero-panic, maintainable Rust.
- **`rust-review`**: The mandatory audit gate applied to all diffs before merging.
- **`rust-port`**: Applied when porting algorithms or fixtures from TypeScript, Python, or shell scripts into idiomatic Rust.

### Mandatory Tooling Invariants
- **`cargo fmt --check`**: 100% compliant; zero formatting drift allowed.
- **`cargo clippy --workspace --all-targets --all-features -- -D warnings`**: Zero warning tolerance across the entire workspace.
- **Workspace Linter Lock**: `unsafe_code = "forbid"` and `missing_debug_implementations = "warn"` are enforced at the compiler level via root `Cargo.toml`.
- **Zero Fabrication**: Never claim a check was run or passed without empirical evidence.

---

## 2. Clean Code, Craftsmanship & Expressive Clarity

Code must communicate intent cleanly and make incorrect behavior difficult to introduce.

- **Small, Cohesive Functions**: Functions must do one coherent task and do it completely. If a function parses, validates, and writes to disk, decompose it into focused steps.
- **Intentional Naming**: Use evocative domain terminology (`BehavioralContract`, `UnifiedPatcher`, `PhaseMachine`, `AuditReceipt`) rather than vague generic terms (`Data`, `Manager`, `Helper`, `Processor`).
- **Small Public APIs**: Keep implementation details `pub(crate)` or private. Only expose what downstream consumers or CLI commands strictly require.
- **Self-Documenting Invariants**: Non-obvious trade-offs, security assumptions, and invariants must be documented with brief, direct comments explaining *why*, never merely narrating *what* the syntax does.
- **Readability Over Line Counts**: Do not compress complex logic into dense one-liners or convoluted iterator chains when an explicit loop or match arm makes ownership and failure paths obvious.

---

## 3. Architecture & SOLID Principles Pragmatically Applied

Apply Clean and Hexagonal Architecture principles pragmatically without importing enterprise Java boilerplate:

- **Separation of Core Decisions from External Side Effects**:
  - `crates/replayfix-core`: Pure domain logic, state machines, diffing algorithms, canonical serialization, and verification oracles. It must remain decoupled from CLI argument parsers, terminal color formatting, and user interaction.
  - `crates/replayfix-cli`: The presentation and orchestration adapter. Responsible for flag parsing, progress rendering, exit code mapping, and calling core engines.
- **Concrete Types by Default**: Start with concrete structs and enums. Do not introduce speculative trait hierarchies (`RepositoryTrait`, `ServiceFactory`, `IHealer`) until multiple distinct implementations or genuine test isolation boundaries demand them.
- **Composition over Inheritance Emulation**: Never emulate OOP inheritance using `Deref` tricks. Compose small types and use enum dispatch for bounded polymorphisms.
- **Unidirectional Dependency Direction**: Dependencies strictly flow inward (`replayfix-cli` $\rightarrow$ `replayfix-core`). Core never imports CLI or presentation dependencies.

---

## 4. Idiomatic Rust & Type-Driven Design

Leverage the Rust type system and ownership model as primary design and verification tools:

- **Make Invalid States Unrepresentable**:
  - Model lifecycle phases as dedicated enum variants (`Phase::Init`, `Phase::Gate1Red`, `Phase::Repair`, `Phase::Gate2Green`, `Phase::Rejected`) with strict state machine transition guards.
  - Use validated constructors (`TaskManifest::from_file`, `assert_safe_path`) that enforce boundary invariants once upon entry.
- **The Zero-Panic Rule in Production Paths**:
  - Zero `.unwrap()` calls are permitted outside unit tests (`#[cfg(test)]`) and top-level `fn main()`.
  - `.expect("...")` is strictly reserved for statically proven infallible operations (such as initializing pre-compiled `Regex` singletons in `OnceLock`).
  - Zero `panic!()`, `todo!()`, or `unimplemented!()` in production execution paths.
- **Deliberate Ownership and Borrowing**:
  - Accept the most general borrowed type (`&str` rather than `&String`, `&[T]` rather than `&Vec<T>`, `&Path` / `impl AsRef<Path>`).
  - Avoid unnecessary cloning. Never clone merely to quiet the borrow checker.
  - Return owned data (`String`, `PathBuf`, `Vec<T>`) from transformations.
- **Derive Standard Traits**:
  - All public types must implement or derive `Debug`.
  - Derived value types should implement `Clone, PartialEq, Eq` where appropriate.
  - Zero-sized service types should derive `Debug, Clone, Copy, PartialEq, Eq, Default`.

---

## 5. Security & Trust Boundaries

Treat security and evidence integrity as correctness requirements, not optional enhancements:

- **Defense-in-Depth Path Validation (`assert_safe_path`)**:
  - **Null Byte Defense**: Immediately reject strings containing `\0`.
  - **UNC Network Path Defense**: Reject `\\host\share` and `//host/share` to prevent SMB credential coercion and NTLM reflection attacks.
  - **DOS Device Defense**: Reject Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9`) across *all* path components (including subdirectories like `project/CON/test`).
  - **Alternate Data Stream (ADS) Defense**: Reject colons `:` outside the valid Windows drive letter prefix (e.g. `C:`), preventing hidden NTFS stream smuggling.
  - **Path Traversal Defense**: Track component depth across normalized components; immediately fail if relative depth drops below root.
  - **Path Normalization**: Normalize backslashes (`\`) to forward slashes (`/`) so security audits evaluate identically on Windows, Linux, and macOS.
- **Fail-Closed Verification**:
  - Any unhandled error, process crash, unexpected exit, or timeout automatically fails closed (`Rejected` / exit code non-zero).
  - Gate 1 requires an explicit counterfactual defect reproduction (RED). A passing baseline or unhandled server crash does not constitute a valid defect reproduction.
  - Gate 2 requires identical contract evaluation (SHA-256 match) and a verified GREEN outcome.
- **Strict Size and Scope Limits**:
  - Patches are bounded to 1 MiB (`MAX_PATCH_BYTES`).
  - Strict unified diffs: single file, exact hunk line offsets and context, zero fuzz matching, no multi-file patches, no file creation/deletion.

---

## 6. Error Taxonomy & System Boundaries

Design failure handling deliberately and transparently:

- **Domain-Specific Errors with `thiserror`**:
  - All fallible operations return `replayfix_core::Result<T>`, alias for `std::result::Result<T, CoreError>`.
  - `CoreError` variants categorize failures cleanly: `Io`, `Serialization`, `Execution`, `Environment`, `Manifest`, `SecurityViolation`, `Verification`, `Patch`.
- **Categorical Failure Distinctions**:
  - Differentiate between:
    1. *Expected verification verdicts* (e.g., Gate 1 not RED, patch rejected as noop, Gate 2 still failing) $\rightarrow$ modeled as `TriageStatus` and captured in `triage_report.json`.
    2. *Security boundary violations* (e.g., directory traversal, UNC path, oversized patch) $\rightarrow$ `CoreError::SecurityViolation` / exit code 11.
    3. *CLI usage errors* (e.g., missing mandatory flags, invalid arguments) $\rightarrow$ exit code 2.
    4. *System / environmental failures* (e.g., disk full, missing binary) $\rightarrow$ exit code 1 or 10.
- **Context Preservation**:
  - Format error messages with the exact failing path, key, line number, or status code.
  - Never swallow underlying I/O or serialization causes.

---

## 7. Determinism, Isolation & Testability

Every verification result must be reproducible and mathematically auditable:

- **RFC 8785 Canonical Serialization**:
  - Contracts and receipts are serialized into key-sorted, compact JSON before cryptographic digesting (`sha256_hex`).
  - Eliminates whitespace, key order, and formatting nondeterminism.
- **Clean Repository Isolation ($\Delta\text{repo} = 0$)**:
  - Target repositories remain strictly read-only.
  - All test executions and code patches occur within ephemeral scratch environments in `std::env::temp_dir()`.
- **Deterministic Port & Process Allocation**:
  - Background servers bind to OS-assigned ephemeral ports (`:0`) to eliminate port collision flakes.
- **Negative, Adversarial & Property Testing**:
  - Unit tests must exercise happy paths, adversarial edge cases (traversal, malicious diffs, zero hunks, context mismatches), and property invariants (e.g., serde serialization exact string parity).

---

## 8. Maintainability, Simplicity & Scope Discipline

Before introducing an abstraction, architectural pattern, or third-party dependency, ask the **Five Forcing Questions**:

1. *What concrete, immediate problem does this solve?*
2. *Is this complexity necessary now, or is it speculative scaffolding?*
3. *Does the standard library or an existing workspace dependency already provide this capability?*
4. *Will this make correctness, testing, or maintenance meaningfully easier?*
5. *Is there a simpler, more direct design with equivalent guarantees?*

- **Aggressive Debloating**: Continuously prune dead code, unused dependencies, and redundant wrapper functions.
- **Vertical Micro-Slices**: Implement coherent end-to-end functionality rather than broad horizontal layers.

---

## 9. Engineering Workflow & Quality Verification

Before committing or releasing any specification increment:

1. **Pre-Implementation**:
   - Verify existing code, contracts, and boundary invariants.
   - Design the smallest sufficient patch.
2. **Implementation**:
   - Write clean, idiomatic code adhering to the BeautifulRust guidelines.
   - Add automated regression and adversarial unit tests.
3. **Audit Gate**:
   - `cargo fmt --check`: formatting compliance.
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: zero warnings.
   - `cargo test --workspace`: 100% pass rate.
   - Inspect `git diff --stat` and `git diff --check` to eliminate accidental edits and whitespace defects.
4. **Binary Verification**:
   - Compile release artifact: `cargo build --release`.
   - On Windows, ensure binaries are signed (`scripts/sign-build-outputs.ps1`) for Smart App Control compliance.
   - Execute CLI smoke tests against known fixtures.

---

## 10. Definition of Quality

A change in ReplayFix is complete if and only if:
- It demonstrably satisfies all functional and non-functional requirements.
- It preserves all constitutional invariants ($\Delta\text{repo} = 0$, fail-closed oracle, contract invariance).
- It introduces zero compiler warnings, zero clippy lints, and zero unsafe blocks.
- It includes automated tests covering both valid execution and adversarial rejection.
- Its public APIs, error types, and documentation are clear, cohesive, and maintainable by any competent systems engineer.

# Data Model: 000-v0.0-workspace-genesis

**Feature**: Clean-Room Workspace Genesis & Toolchain Lock-In  
**Date**: 2026-10-26  

---

## 1. Domain Entities & Enums

### `OsFamily`
Identifies the operating system family at runtime.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsFamily {
    Windows,
    Unix,
}
```

- **Validation Rules**:
  - Deterministically maps from `cfg!(windows)` to `OsFamily::Windows`, else `OsFamily::Unix`.
- **Usage**:
  - Guides platform-specific process execution and path canonicalization.

---

### `Platform`
Stateless utility struct providing cross-platform operations.

```rust
pub struct Platform;

impl Platform {
    pub const fn os_family() -> OsFamily;
    pub fn create_command(program: &str) -> std::process::Command;
    pub fn canonicalize_path(path: impl AsRef<std::path::Path>) -> Result<std::path::PathBuf>;
}
```

- **Methods**:
  - `os_family()`: Returns current `OsFamily`.
  - `create_command(program)`: Instantiates a standard `std::process::Command` without shell wrapper leakage.
  - `canonicalize_path(path)`: Safely resolves absolute paths, catching non-existent paths or traversal errors.

---

### `CoreError`
Central error taxonomy for `replayfix-core`.

```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Platform I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Command execution error: {0}")]
    Execution(String),

    #[error("Environment doctor error: {0}")]
    Environment(String),

    #[error("Verification failed: {0}")]
    Verification(String),
}
```

- **Validation Rules**:
  - Provides bidirectional conversions from standard `std::io::Error` and `serde_json::Error`.
  - Enables clean propagation using the `?` operator throughout the core library.

# 📊 Data Model: ReplayFix v0.8.0

**Feature Identity**: `080-v0.8-observability-ingestion`

---

## 1. Domain Entities & Rust Type Definitions

### 1.1 Ingestion (`crates/replayfix-core/src/ingestion.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubIssueRef {
    pub owner: String,
    pub repo: String,
    pub issue_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssuePayload {
    pub url: String,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub user: GitHubUser,
    pub labels: Vec<GitHubLabel>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLabel {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionResult {
    pub issue_ref: GitHubIssueRef,
    pub title: String,
    pub raw_body: String,
    pub extracted_steps: Vec<String>,
    pub untrusted_context_block: String,
    pub retrieved_at: String,
}
```

### 1.2 Observability & Tracing (`crates/replayfix-core/src/trace.rs`)

```rust
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfig {
    pub enabled: bool,
    pub output_path: PathBuf,
    pub screenshots: bool,
    pub snapshots: bool,
    pub sources: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceLaunchOutcome {
    pub launched: bool,
    pub pid: Option<u32>,
    pub reason: String,
}
```

### 1.3 Human Approval Gate (`crates/replayfix-cli/src/approval.rs`)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalMode {
    ManualAffirmative,
    AutoBypassFlag,
    NonInteractiveRejected,
    ManualNegative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalAudit {
    pub timestamp: String,
    pub patch_sha256: String,
    pub files_modified: Vec<String>,
    pub insertions: usize,
    pub deletions: usize,
    pub decision: ApprovalDecision,
    pub mode: ApprovalMode,
    pub tty_detected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Approved,
    Rejected,
}
```

---

## 2. JSON Schema Mapping

| Domain Type | Contract Schema | Target Output Path |
|-------------|-----------------|--------------------|
| `IngestionResult` | `contracts/issue_ingestion.schema.json` | `.replayfix/artifacts/issue_ingestion.json` |
| `ApprovalAudit` | `contracts/approval_audit.schema.json` | `.replayfix/artifacts/approval_audit.json` |

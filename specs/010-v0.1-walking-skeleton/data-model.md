# Data Model: 010-v0.1-walking-skeleton

**Feature**: Environment Diagnostics & Repro Manifest Ingestion  
**Date**: 2026-10-26  

---

## 1. Doctor Diagnostic Entities

### `DependencyStatus`
Represents the compatibility status of an external dependency.

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DependencyStatus {
    Compatible,
    IncompatibleVersion { found: String, required: String },
    NotFound,
    PermissionDenied,
}
```

### `DependencyCheck`
Represents the evaluated diagnostic result for a single tool.

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DependencyCheck {
    pub name: String,
    pub executable: String,
    pub found_version: Option<String>,
    pub required_version: String,
    pub status: DependencyStatus,
    pub remediation: Option<String>,
}
```

### `DoctorReport`
Aggregate health report emitted by `replayfix doctor`.

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DoctorReport {
    pub overall_status: bool,
    pub host_os: String,
    pub checks: Vec<DependencyCheck>,
    pub timestamp: String,
}
```

---

## 2. Task Manifest Ingestion Entities

### `EnvironmentAdapter`
Application server launch and healthcheck configuration.

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnvironmentAdapter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_cmd: Option<String>,
    #[serde(default = "default_port_env_var")]
    pub port_env_var: String,
    #[serde(default = "default_healthcheck_path")]
    pub healthcheck_path: String,
    #[serde(default = "default_startup_timeout_ms")]
    pub startup_timeout_ms: u64,
}
```

### `TaskManifest`
Canonical representation conforming to `task_manifest.schema.json`.

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskManifest {
    pub task_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub repo_path: String,
    pub bug_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clean_ref: Option<String>,
    pub video_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_behavior: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_adapter: Option<EnvironmentAdapter>,
    #[serde(default)]
    pub headed: bool,
}
```

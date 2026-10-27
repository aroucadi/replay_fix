# Data Model: 030-v0.3-ephemeral-scratch-isolation

## FileEntry (`replayfix_core::worktree`)
| Field | Type | Notes |
| :--- | :--- | :--- |
| `path` | `String` | Normalized `/` relative path from worktree root |
| `size` | `u64` | File size in bytes |
| `sha256` | `String` | Lowercase hex SHA-256 of file contents |

## WorktreeManifest (`replayfix_core::worktree`)
| Field | Type | Notes |
| :--- | :--- | :--- |
| `tree_sha256` | `String` | Lowercase hex SHA-256 over canonical JSON of `entries` |
| `entries` | `Vec<FileEntry>` | Sorted lexicographically by `path` |
| `git_porcelain` | `Option<String>` | Output of `git status --porcelain=v1 -uall` when `.git` exists |

## ScratchSandbox (`replayfix_core::worktree`)
| Field | Type | Notes |
| :--- | :--- | :--- |
| `root` | `PathBuf` | Unique directory inside `std::env::temp_dir()` |
| `child_pids` | `Vec<u32>` | Child process IDs terminated before directory removal |
| `cleaned` | `bool` | Tracks whether explicit `teardown()` already ran before `Drop` |

## EphemeralPort (`replayfix_core::platform`)
| Field | Type | Notes |
| :--- | :--- | :--- |
| `port` | `u16` | Non-zero localhost TCP port assigned by the OS kernel (`127.0.0.1:0`) |

## AuditReceipt (`audit_receipt.json`, schema `"0.3"`)
Extends v0.2 `AuditReceipt` with:
| Field | Type | Notes |
| :--- | :--- | :--- |
| `schema_version` | `String` | `"0.3"` |
| `base_tree_sha256` | `String` | Pre/post verified SHA-256 tree digest of the base repository |
| `sandbox_tree_sha256` | `String` | Post-repair SHA-256 tree digest of the scratch sandbox |
| `base_repo_unmodified_verified` | `bool` | `true` strictly when pre and post `WorktreeManifest` match |
| `ephemeral_port` | `u16` | Allocated ephemeral port (`> 0`) |

## TriageStatus (`triage_report.json`, schema `"0.3"`)
Adds variant:
- `BaseRepoDirtyMutationDetected` → serializes as `"BASE_REPO_DIRTY_MUTATION_DETECTED"`, `exit_code() == 11`.

//! Persistent Agent Session State & Workflow Continuity.
//!
//! Tracks multi-turn conversational activity, active task checkpoints,
//! and audit records to ensure seamless session resumption and recovery.

use crate::permissions::PermissionManager;
use replayfix_core::checkpoint::SessionStateCheckpoint;
use replayfix_core::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A single conversational interaction turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTurn {
    pub timestamp: String,
    pub user_input: String,
    pub agent_response: String,
    pub task_id: Option<String>,
}

/// Persistent agent session record saved to `.replayfix/session_state.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub started_at: String,
    pub last_updated: String,
    pub repo_path: String,
    pub active_task: Option<String>,
    pub turns: Vec<SessionTurn>,
    pub checkpoint: Option<SessionStateCheckpoint>,
}

/// Active interactive session manager.
#[derive(Debug)]
pub struct AgentSession {
    pub record: SessionRecord,
    pub permissions: PermissionManager,
}

impl AgentSession {
    #[must_use]
    pub fn new(repo_path: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let session_id = format!("rf-sess-{}", chrono::Utc::now().timestamp());
        Self {
            record: SessionRecord {
                session_id,
                started_at: now.clone(),
                last_updated: now,
                repo_path: repo_path.into(),
                active_task: None,
                turns: Vec::new(),
                checkpoint: None,
            },
            permissions: PermissionManager::new(),
        }
    }

    /// Load existing session from `.replayfix/session_state.json` or create a new one.
    pub fn load_or_init(output_dir: &Path, repo_path: &Path) -> Self {
        let session_file = output_dir.join("session_state.json");
        if session_file.exists() {
            if let Ok(content) = fs::read_to_string(&session_file) {
                if let Ok(record) = serde_json::from_str::<SessionRecord>(&content) {
                    return Self {
                        record,
                        permissions: PermissionManager::new(),
                    };
                }
            }
        }
        Self::new(repo_path.display().to_string())
    }

    /// Record a conversation turn and persist state.
    pub fn record_turn(
        &mut self,
        user_input: impl Into<String>,
        agent_response: impl Into<String>,
        task_id: Option<String>,
    ) {
        let turn = SessionTurn {
            timestamp: chrono::Utc::now().to_rfc3339(),
            user_input: user_input.into(),
            agent_response: agent_response.into(),
            task_id,
        };
        self.record.last_updated = turn.timestamp.clone();
        self.record.turns.push(turn);
    }

    /// Link active stage checkpoint.
    pub fn set_checkpoint(&mut self, checkpoint: SessionStateCheckpoint) {
        self.record.active_task = Some(checkpoint.bug_id.clone());
        self.record.checkpoint = Some(checkpoint);
        self.record.last_updated = chrono::Utc::now().to_rfc3339();
    }

    /// Save session state atomically to file.
    pub fn save(&self, output_dir: &Path) -> Result<()> {
        if !output_dir.exists() {
            fs::create_dir_all(output_dir)?;
        }
        let session_file = output_dir.join("session_state.json");
        let serialized = serde_json::to_string_pretty(&self.record)?;
        fs::write(&session_file, serialized)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_lifecycle_and_serialization() {
        let mut session = AgentSession::new(".");
        assert!(session.record.turns.is_empty());
        assert_eq!(session.record.repo_path, ".");

        session.record_turn(
            "fix cart bug",
            "Formulated execution plan for RF-01",
            Some("RF-01".to_string()),
        );
        assert_eq!(session.record.turns.len(), 1);
        assert_eq!(session.record.turns[0].task_id.as_deref(), Some("RF-01"));

        let serialized = serde_json::to_string(&session.record).expect("must serialize");
        let restored: SessionRecord = serde_json::from_str(&serialized).expect("must deserialize");
        assert_eq!(restored.session_id, session.record.session_id);
        assert_eq!(restored.turns.len(), 1);
    }
}

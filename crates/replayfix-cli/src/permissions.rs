//! Context-Aware Tiered Permission & Approval Engine.
//!
//! Enforces granular, risk-based access control protecting the host environment
//! and repository integrity while preserving frictionless developer interaction.

use crate::ui::{Symbols, Theme};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{self, BufRead, Write};

/// Risk categorization for agent-driven actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    /// Safe, read-only operations (file reading, inspection, scratch keyframe extraction).
    Low,
    /// Operations requiring local resources or external network (spawning child servers, calling AI API).
    Medium,
    /// Consequential write operations (mutating base repository files, applying patches).
    High,
}

impl RiskLevel {
    #[must_use]
    pub const fn badge(self) -> &'static str {
        match self {
            Self::Low => "[LOW RISK]",
            Self::Medium => "[MEDIUM RISK]",
            Self::High => "[HIGH RISK]",
        }
    }
}

/// Action requiring authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionAction {
    /// Reading workspace repository files.
    InspectWorkspace { path: String },
    /// Spawning a child dev server process.
    SpawnServer { command: String, port_hint: String },
    /// Dispatching an outbound HTTPS request to Meta AI gateway.
    CallModelGateway { model: String, role: String },
    /// Applying a verified candidate patch to the base repository.
    ApplyPatchToBaseRepo {
        files: Vec<String>,
        insertions: usize,
        deletions: usize,
        patch_sha256: String,
    },
}

impl PermissionAction {
    #[must_use]
    pub fn risk_level(&self) -> RiskLevel {
        match self {
            Self::InspectWorkspace { .. } => RiskLevel::Low,
            Self::SpawnServer { .. } | Self::CallModelGateway { .. } => RiskLevel::Medium,
            Self::ApplyPatchToBaseRepo { .. } => RiskLevel::High,
        }
    }

    #[must_use]
    pub fn title(&self) -> &'static str {
        match self {
            Self::InspectWorkspace { .. } => "Read Workspace Files",
            Self::SpawnServer { .. } => "Launch Ephemeral Application Server",
            Self::CallModelGateway { .. } => "Outbound Meta AI API Request",
            Self::ApplyPatchToBaseRepo { .. } => "Apply Verified Patch to Base Repository",
        }
    }

    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::InspectWorkspace { path } => {
                format!("Inspect source code and configurations at '{path}'")
            }
            Self::SpawnServer { command, port_hint } => {
                format!("Spawn dev server '{command}' listening on ephemeral port {port_hint}")
            }
            Self::CallModelGateway { model, role } => {
                format!("Send encrypted prompt payload to model '{model}' for {role}")
            }
            Self::ApplyPatchToBaseRepo {
                files,
                insertions,
                deletions,
                patch_sha256,
            } => {
                let short_hash = patch_sha256.get(..12).unwrap_or(patch_sha256);
                format!(
                    "Mutate base repository: modify {} file(s) (+{insertions} -{deletions}) [patch {short_hash}]",
                    files.len()
                )
            }
        }
    }

    #[must_use]
    pub fn permission_key(&self) -> String {
        match self {
            Self::InspectWorkspace { .. } => "inspect_workspace".to_string(),
            Self::SpawnServer { .. } => "spawn_server".to_string(),
            Self::CallModelGateway { model, .. } => format!("call_model:{model}"),
            Self::ApplyPatchToBaseRepo { patch_sha256, .. } => {
                format!("apply_patch:{patch_sha256}")
            }
        }
    }
}

/// Authorization outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionDecision {
    AllowOnce,
    AllowSession,
    Deny,
}

/// Session-scoped permission manager tracking persistent grants.
#[derive(Debug, Default, Clone)]
pub struct PermissionManager {
    session_grants: HashSet<String>,
}

impl PermissionManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            session_grants: HashSet::new(),
        }
    }

    /// Check if action is already authorized by session policy.
    #[must_use]
    pub fn is_session_granted(&self, action: &PermissionAction) -> bool {
        self.session_grants.contains(&action.permission_key())
    }

    /// Grant permission for the lifetime of this session.
    pub fn grant_session(&mut self, action: &PermissionAction) {
        self.session_grants.insert(action.permission_key());
    }

    /// Evaluate permission request against policy, auto-confirm flags, and user prompt.
    pub fn request_permission(
        &mut self,
        action: &PermissionAction,
        auto_confirm: bool,
        is_interactive: bool,
        theme: &Theme,
    ) -> PermissionDecision {
        // Low risk actions are auto-approved
        if action.risk_level() == RiskLevel::Low {
            return PermissionDecision::AllowOnce;
        }

        // Check active session grant
        if self.is_session_granted(action) {
            return PermissionDecision::AllowSession;
        }

        // Non-interactive sessions without auto_confirm fail closed
        if !is_interactive {
            return if auto_confirm {
                PermissionDecision::AllowOnce
            } else {
                PermissionDecision::Deny
            };
        }

        // If auto-confirm is enabled via CLI flag (--yes)
        if auto_confirm {
            return PermissionDecision::AllowOnce;
        }

        // Render interactive permission card
        self.prompt_user(action, theme)
    }

    fn prompt_user(&mut self, action: &PermissionAction, theme: &Theme) -> PermissionDecision {
        println!();
        let bar = "─".repeat(60);
        println!("  {}", theme.muted(&bar));

        let (badge, title_style) = match action.risk_level() {
            RiskLevel::Low => (
                theme.info(RiskLevel::Low.badge()),
                theme.info(action.title()),
            ),
            RiskLevel::Medium => (
                theme.warning(RiskLevel::Medium.badge()),
                theme.warning(action.title()),
            ),
            RiskLevel::High => (
                theme.error(RiskLevel::High.badge()),
                theme.bold(action.title()),
            ),
        };

        println!("  {badge} {title_style}");
        println!(
            "    {} {}",
            theme.muted("Description:"),
            action.description()
        );

        if let PermissionAction::ApplyPatchToBaseRepo { files, .. } = action {
            println!(
                "    {} Target file(s): {}",
                theme.muted("Impact:"),
                theme.bold(&files.join(", "))
            );
            println!(
                "    {} {}",
                theme.warning(Symbols::WARN),
                theme.warning("Base repository files will be permanently modified.")
            );
        }

        println!("  {}", theme.muted(&bar));

        if action.risk_level() == RiskLevel::High {
            print!("  Authorize this action? [y: Yes / n: No]: ");
        } else {
            print!("  Authorize action? [a: Allow Once / s: Allow for Session / d: Deny]: ");
        }
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().lock().read_line(&mut input).is_err() {
            return PermissionDecision::Deny;
        }

        let choice = input.trim().to_lowercase();
        match choice.as_str() {
            "y" | "yes" | "a" | "allow" => PermissionDecision::AllowOnce,
            "s" | "session" => {
                self.grant_session(action);
                PermissionDecision::AllowSession
            }
            _ => PermissionDecision::Deny,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_levels_and_defaults() {
        let inspect = PermissionAction::InspectWorkspace {
            path: ".".to_string(),
        };
        assert_eq!(inspect.risk_level(), RiskLevel::Low);

        let spawn = PermissionAction::SpawnServer {
            command: "npm run dev".to_string(),
            port_hint: ":0".to_string(),
        };
        assert_eq!(spawn.risk_level(), RiskLevel::Medium);

        let patch = PermissionAction::ApplyPatchToBaseRepo {
            files: vec!["src/app.js".to_string()],
            insertions: 5,
            deletions: 1,
            patch_sha256: "abc123456789".to_string(),
        };
        assert_eq!(patch.risk_level(), RiskLevel::High);
    }

    #[test]
    fn test_permission_manager_session_grant() {
        let mut pm = PermissionManager::new();
        let action = PermissionAction::SpawnServer {
            command: "npm run dev".to_string(),
            port_hint: ":0".to_string(),
        };

        assert!(!pm.is_session_granted(&action));
        pm.grant_session(&action);
        assert!(pm.is_session_granted(&action));

        let theme = Theme::for_stdout(crate::ui::ColorOption::Never, crate::ui::OutputMode::Quiet);
        // Evaluating session granted action returns AllowSession without prompt
        let decision = pm.request_permission(&action, false, true, &theme);
        assert_eq!(decision, PermissionDecision::AllowSession);
    }

    #[test]
    fn test_non_interactive_refusal() {
        let mut pm = PermissionManager::new();
        let patch = PermissionAction::ApplyPatchToBaseRepo {
            files: vec!["index.js".to_string()],
            insertions: 1,
            deletions: 1,
            patch_sha256: "sha123".to_string(),
        };
        let theme = Theme::for_stdout(crate::ui::ColorOption::Never, crate::ui::OutputMode::Quiet);

        // Non-interactive without auto_confirm -> Deny
        let decision = pm.request_permission(&patch, false, false, &theme);
        assert_eq!(decision, PermissionDecision::Deny);

        // Non-interactive with auto_confirm -> AllowOnce
        let decision = pm.request_permission(&patch, true, false, &theme);
        assert_eq!(decision, PermissionDecision::AllowOnce);
    }
}

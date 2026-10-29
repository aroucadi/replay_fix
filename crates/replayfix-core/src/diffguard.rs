//! DiffGuard: Anti-cheat patch validator enforcing runner configuration immutability
//! and surgical `package.json` test script integrity per Spec 070 (FR-003, FR-004).

use crate::error::{CoreError, Result};
use crate::repair::{StructuredRepairPatch, TestFreezeGuard};
use std::collections::BTreeMap;

/// Cryptographic and structural boundary guard rejecting tampering with
/// test runner configurations or test verification scripts.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiffGuard;

impl DiffGuard {
    /// Validate that a candidate patch targets a legitimate application source file,
    /// rejecting any test files, runner configurations, or `package.json` test script alterations.
    pub fn validate_patch(
        patch: &StructuredRepairPatch,
        freeze_guard: &TestFreezeGuard,
    ) -> Result<String> {
        let trimmed_target = patch.target_file.trim();

        // 1. Direct runner config check with CONFIG_MUTATION_PROHIBITED
        if Self::is_runner_config(trimmed_target) {
            return Err(CoreError::SecurityViolation(format!(
                "CONFIG_MUTATION_PROHIBITED: repair patch attempted to modify protected test runner configuration '{trimmed_target}'"
            )));
        }

        // 2. Validate against TestFreezeGuard file-level boundary
        let safe_target = freeze_guard.validate_target_path(trimmed_target)?;

        // 3. Surgical package.json structural validation
        if safe_target == "package.json" {
            Self::validate_package_json_diff(&patch.search_block, &patch.replace_block)?;
        }

        Ok(safe_target)
    }

    /// Check if a filename or path is a protected test runner configuration file.
    #[must_use]
    pub fn is_runner_config(raw_path: &str) -> bool {
        let normalized = raw_path.trim().replace('\\', "/");
        let filename = normalized
            .split('/')
            .next_back()
            .unwrap_or(&normalized)
            .trim_end_matches(['.', ' ']);
        let lower = filename.to_ascii_lowercase();

        lower.starts_with("playwright.config.")
            || lower.starts_with("jest.config.")
            || lower.starts_with("vitest.config.")
            || lower.starts_with("cypress.config.")
            || (lower.starts_with("tsconfig") && lower.ends_with(".json"))
            || (lower.starts_with("jsconfig") && lower.ends_with(".json"))
    }

    /// Structurally inspect a `package.json` search and replace block.
    /// Permits benign changes to dependencies, build scripts, or metadata,
    /// but rejects any modification to `"scripts.test*"` keys.
    pub fn validate_package_json_diff(search_block: &str, replace_block: &str) -> Result<()> {
        let norm_search = search_block.replace("\r\n", "\n");
        let norm_replace = replace_block.replace("\r\n", "\n");

        // Fast lexical scan for test script keys
        let has_test_key = |block: &str| -> bool {
            let lower = block.to_ascii_lowercase();
            lower.contains("\"test\"")
                || lower.contains("'test'")
                || lower.contains("\"test:")
                || lower.contains("'test:")
                || lower.contains("\"test_")
                || lower.contains("'test_")
        };

        if has_test_key(&norm_search) || has_test_key(&norm_replace) {
            return Err(CoreError::SecurityViolation(
                "CONFIG_MUTATION_PROHIBITED: package.json verification test scripts altered"
                    .to_string(),
            ));
        }

        // Attempt JSON object parsing if search or replace represents complete objects
        if let (Ok(s_val), Ok(r_val)) = (
            serde_json::from_str::<serde_json::Value>(&norm_search),
            serde_json::from_str::<serde_json::Value>(&norm_replace),
        ) {
            if let (Some(s_obj), Some(r_obj)) = (s_val.as_object(), r_val.as_object()) {
                let s_scripts = s_obj.get("scripts").and_then(|v| v.as_object());
                let r_scripts = r_obj.get("scripts").and_then(|v| v.as_object());

                if let (Some(s_map), Some(r_map)) = (s_scripts, r_scripts) {
                    let s_test_scripts: BTreeMap<_, _> = s_map
                        .iter()
                        .filter(|(k, _)| k.starts_with("test"))
                        .collect();
                    let r_test_scripts: BTreeMap<_, _> = r_map
                        .iter()
                        .filter(|(k, _)| k.starts_with("test"))
                        .collect();

                    if s_test_scripts != r_test_scripts {
                        return Err(CoreError::SecurityViolation(
                            "CONFIG_MUTATION_PROHIBITED: package.json verification test scripts altered"
                                .to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_runner_config() {
        assert!(DiffGuard::is_runner_config("playwright.config.ts"));
        assert!(DiffGuard::is_runner_config("./jest.config.js"));
        assert!(DiffGuard::is_runner_config("configs/vitest.config.mjs"));
        assert!(DiffGuard::is_runner_config("tsconfig.json"));
        assert!(DiffGuard::is_runner_config("tsconfig.app.json"));
        assert!(!DiffGuard::is_runner_config("src/app.tsx"));
        assert!(!DiffGuard::is_runner_config("package.json"));
    }

    #[test]
    fn test_package_json_benign_dependency_addition_allowed() {
        let search = r#""dependencies": { "react": "^18.0.0" }"#;
        let replace = r#""dependencies": { "react": "^18.0.0", "lodash": "^4.17.21" }"#;
        assert!(DiffGuard::validate_package_json_diff(search, replace).is_ok());
    }

    #[test]
    fn test_package_json_test_script_mutation_rejected() {
        let search = r#""test": "playwright test""#;
        let replace = r#""test": "exit 0""#;
        let res = DiffGuard::validate_package_json_diff(search, replace);
        assert!(res.is_err());
        let err = res.unwrap_err().to_string();
        assert!(err.contains("CONFIG_MUTATION_PROHIBITED"));
    }

    #[test]
    fn test_validate_patch_blocks_runner_configs() {
        let dir = std::env::temp_dir().join(format!("replayfix-dg-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let guard = TestFreezeGuard::capture(&dir).unwrap();
        let patch = StructuredRepairPatch {
            explanation: "tamper".to_string(),
            target_file: "playwright.config.ts".to_string(),
            search_block: "retries: 2".to_string(),
            replace_block: "retries: 0".to_string(),
            touched_symbols: vec!["retries".to_string()],
        };

        let res = DiffGuard::validate_patch(&patch, &guard);
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("CONFIG_MUTATION_PROHIBITED"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

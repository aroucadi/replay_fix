//! Universal Repository Target Source Code Discovery (`RepoSourceMapper`).
//!
//! Provides zero-shot correlation between runtime browser signals (uncaught exceptions,
//! DOM selectors, class names, test IDs, and accessibility nodes) and actual source files
//! in arbitrary, previously unseen web repositories.

use crate::error::{CoreError, Result};
use crate::locator::DomCandidate;
use crate::manifest::TaskManifest;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Relevant candidate target source file discovered in the target repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedTargetSource {
    pub relative_path: PathBuf,
    pub absolute_path: PathBuf,
    pub confidence_score_bps: u32,
    pub match_reason: String,
    pub source_content: String,
}

/// Owned specification for a dynamically discovered repository task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicTaskSpec {
    pub task_id: String,
    pub target_path: String,
    pub buggy_source: String,
    pub builtin_diff: String,
    pub index_html: String,
    pub click_x: i32,
    pub click_y: i32,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub expected_output_text: String,
    pub contract: crate::engine::BehavioralContract,
}

/// Query signals used to locate relevant source code files.
#[derive(Debug, Clone, Default)]
pub struct SourceQuery<'a> {
    pub runtime_errors: &'a [String],
    pub dom_candidates: &'a [DomCandidate],
    pub selector: Option<&'a str>,
    pub hint_tokens: &'a [&'a str],
}

/// Scans and ranks source files in an arbitrary repository using runtime telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RepoSourceMapper;

impl RepoSourceMapper {
    /// Supported source code extensions to scan.
    const SOURCE_EXTENSIONS: &'static [&'static str] = &[
        "tsx", "ts", "jsx", "js", "vue", "svelte", "html", "mjs", "cjs",
    ];

    /// Standard directory names to skip during recursive source discovery.
    const EXCLUDED_DIRS: &'static [&'static str] = &[
        "node_modules",
        ".git",
        "dist",
        "build",
        "target",
        ".replayfix",
        ".next",
        "coverage",
        ".cache",
    ];

    /// Locate candidate target files in `repo_root` based on runtime query signals.
    pub fn locate_target_file(
        repo_root: &Path,
        query: &SourceQuery<'_>,
    ) -> Result<ResolvedTargetSource> {
        let safe_repo =
            TaskManifest::assert_safe_path(repo_root.to_str().unwrap_or("."), "repo_root")?;

        // 1. Stack trace & runtime error frame extraction (Highest confidence: 10,000 bps)
        for err in query.runtime_errors {
            if let Some(target) = Self::find_file_from_error_trace(&safe_repo, err)? {
                if target
                    .relative_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("html"))
                {
                    if let Some(script_target) =
                        Self::follow_html_script_src(&safe_repo, &target.source_content)
                    {
                        return Ok(script_target);
                    }
                }
                return Ok(target);
            }
        }

        // 2. Collect searchable tokens from DOM candidates, selectors, and hints
        let mut tokens = Vec::new();
        if let Some(sel) = query.selector {
            tokens.extend(Self::extract_tokens_from_selector(sel));
        }
        for cand in query.dom_candidates {
            if let Some(tid) = &cand.test_id {
                tokens.push(tid.as_str());
            }
            if !cand.accessible_name.is_empty() {
                tokens.push(cand.accessible_name.as_str());
            }
        }
        tokens.extend_from_slice(query.hint_tokens);

        // 3. Scan all source files in the repository
        let all_sources = Self::scan_source_files(&safe_repo)?;
        if all_sources.is_empty() {
            return Err(CoreError::Grounding(format!(
                "No source code files found in repository '{}'",
                safe_repo.display()
            )));
        }

        // Rank files by token match density
        let mut best_match: Option<ResolvedTargetSource> = None;
        let mut best_score = 0u32;

        for (rel, abs) in all_sources {
            let Ok(content) = fs::read_to_string(&abs) else {
                continue;
            };

            let mut match_count = 0u32;
            for token in &tokens {
                if !token.is_empty() && content.contains(token) {
                    match_count = match_count.saturating_add(1);
                }
            }

            // Bonus for components in lib/ or src/ when at least one token matched
            let rel_str = rel
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();
            let mut score = match_count.saturating_mul(1000);
            if match_count > 0 && (rel_str.starts_with("src/") || rel_str.starts_with("lib/")) {
                score = score.saturating_add(500);
            }

            if score > best_score {
                best_score = score;
                best_match = Some(ResolvedTargetSource {
                    relative_path: rel,
                    absolute_path: abs,
                    confidence_score_bps: score.min(9_500),
                    match_reason: format!("Matched {match_count} runtime tokens across AST"),
                    source_content: content,
                });
            }
        }

        if let Some(target) = best_match {
            if target
                .relative_path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("html"))
            {
                if let Some(script_target) =
                    Self::follow_html_script_src(&safe_repo, &target.source_content)
                {
                    return Ok(script_target);
                }
            }
            return Ok(target);
        }

        // 4. Fallback to primary entrypoint
        Self::resolve_primary_entrypoint(&safe_repo)
    }

    /// Extract filename and line number from an uncaught error message or stack trace.
    fn find_file_from_error_trace(
        repo_root: &Path,
        error_msg: &str,
    ) -> Result<Option<ResolvedTargetSource>> {
        for ext in Self::SOURCE_EXTENSIONS {
            let pattern = format!(".{ext}");
            for word in error_msg
                .split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == '\'' || c == '"')
            {
                if let Some(idx) = word.find(&pattern) {
                    let end_pos = idx + pattern.len();
                    let file_candidate = &word[..end_pos];
                    let trimmed = file_candidate
                        .trim_start_matches("http://")
                        .trim_start_matches("https://");
                    let path_part = if let Some(slash) = trimmed.find('/') {
                        &trimmed[slash + 1..]
                    } else {
                        trimmed
                    };
                    let filename = Path::new(path_part).file_name().and_then(|n| n.to_str());

                    if let Some(fname) = filename {
                        let sources = Self::scan_source_files(repo_root)?;
                        for (rel, abs) in sources {
                            if rel.file_name().and_then(|n| n.to_str()) == Some(fname) {
                                let content = fs::read_to_string(&abs)?;
                                return Ok(Some(ResolvedTargetSource {
                                    relative_path: rel,
                                    absolute_path: abs,
                                    confidence_score_bps: 10_000,
                                    match_reason: format!(
                                        "Direct stack trace error reference: {fname}"
                                    ),
                                    source_content: content,
                                }));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Scan repository recursively for all supported source files, ignoring excluded directories.
    pub fn scan_source_files(repo_root: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
        let mut results = Vec::new();
        Self::scan_dir_recursive(repo_root, repo_root, &mut results)?;
        results.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(results)
    }

    fn scan_dir_recursive(
        base: &Path,
        current: &Path,
        results: &mut Vec<(PathBuf, PathBuf)>,
    ) -> Result<()> {
        let entries = match fs::read_dir(current) {
            Ok(iter) => iter,
            Err(_) => return Ok(()),
        };

        for entry_res in entries {
            let Ok(entry) = entry_res else {
                continue;
            };
            let path = entry.path();
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                if Self::EXCLUDED_DIRS.iter().any(|d| *d == name_str) {
                    continue;
                }
                Self::scan_dir_recursive(base, &path, results)?;
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_ascii_lowercase();
                    if Self::SOURCE_EXTENSIONS.contains(&ext_lower.as_str()) {
                        let Ok(rel) = path.strip_prefix(base) else {
                            continue;
                        };
                        results.push((rel.to_path_buf(), path));
                    }
                }
            }
        }
        Ok(())
    }

    fn extract_tokens_from_selector(selector: &str) -> Vec<&str> {
        const PLAYWRIGHT_NOISE: &[&str] = &[
            "page",
            "getByRole",
            "getByTestId",
            "getByText",
            "getByLabel",
            "locator",
            "name",
            "exact",
            "true",
            "false",
            "button",
        ];
        let mut out = Vec::new();
        for tok in selector
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .filter(|s| {
                s.len() >= 3
                    && !s.chars().all(|c| c.is_ascii_digit())
                    && !PLAYWRIGHT_NOISE.contains(s)
            })
        {
            out.push(tok);
            if let Some((prefix, _)) = tok.rsplit_once('-') {
                if prefix.len() >= 3 {
                    out.push(prefix);
                }
            }
            for sub in tok.split(['-', '_']) {
                if sub.len() >= 4 && !sub.chars().all(|c| c.is_ascii_digit()) {
                    out.push(sub);
                }
            }
        }
        out
    }

    /// Determine the project's primary entrypoint file dynamically via `package.json` and standard conventions.
    fn resolve_primary_entrypoint(repo_root: &Path) -> Result<ResolvedTargetSource> {
        let pkg_path = repo_root.join("package.json");
        if pkg_path.is_file() {
            if let Ok(raw_pkg) = fs::read_to_string(&pkg_path) {
                if let Ok(pkg_val) = serde_json::from_str::<serde_json::Value>(&raw_pkg) {
                    for field in ["source", "module", "main"] {
                        if let Some(entry_rel) = pkg_val.get(field).and_then(|v| v.as_str()) {
                            let clean_rel = entry_rel.trim_start_matches("./");
                            let entry_path = repo_root.join(clean_rel);
                            if entry_path.is_file() {
                                if let Ok(entry_content) = fs::read_to_string(&entry_path) {
                                    if let Some(resolved_reexport) =
                                        Self::follow_barrel_reexport(repo_root, &entry_content)
                                    {
                                        return Ok(resolved_reexport);
                                    }
                                    return Ok(ResolvedTargetSource {
                                        relative_path: PathBuf::from(clean_rel),
                                        absolute_path: entry_path,
                                        confidence_score_bps: 6_000,
                                        match_reason: format!(
                                            "Resolved package.json '{field}' entrypoint: {clean_rel}"
                                        ),
                                        source_content: entry_content,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        let candidates = [
            "src/App.tsx",
            "src/App.jsx",
            "src/App.js",
            "src/App.vue",
            "src/App.svelte",
            "src/index.tsx",
            "src/index.js",
            "src/main.ts",
            "src/main.js",
            "public/app.js",
            "server.js",
            "app.js",
            "index.html",
        ];

        for cand in candidates {
            let p = repo_root.join(cand);
            if p.is_file() {
                let content = fs::read_to_string(&p)?;
                return Ok(ResolvedTargetSource {
                    relative_path: PathBuf::from(cand),
                    absolute_path: p,
                    confidence_score_bps: 5_000,
                    match_reason: format!("Resolved standard entrypoint candidate: {cand}"),
                    source_content: content,
                });
            }
        }

        // Final fallback: take the first discovered source file in src/ or lib/, or workspace root
        let sources = Self::scan_source_files(repo_root)?;
        if let Some((rel, abs)) = sources
            .iter()
            .find(|(r, _)| r.starts_with("src") || r.starts_with("lib"))
            .cloned()
            .or_else(|| sources.into_iter().next())
        {
            let content = fs::read_to_string(&abs)?;
            return Ok(ResolvedTargetSource {
                relative_path: rel,
                absolute_path: abs,
                confidence_score_bps: 1_000,
                match_reason: "First available source file in workspace".to_string(),
                source_content: content,
            });
        }

        Err(CoreError::Grounding(format!(
            "Unable to resolve any candidate source files in repository '{}'",
            repo_root.display()
        )))
    }

    fn follow_barrel_reexport(
        repo_root: &Path,
        barrel_content: &str,
    ) -> Option<ResolvedTargetSource> {
        if barrel_content.lines().count() > 25 {
            return None;
        }
        for token in barrel_content.split(['\'', '"']) {
            let trimmed = token.trim().trim_start_matches("./");
            if trimmed.is_empty() || trimmed.contains(' ') {
                continue;
            }
            let stem = trimmed
                .strip_prefix("build/")
                .or_else(|| trimmed.strip_prefix("dist/"))
                .or_else(|| trimmed.strip_prefix("lib/"))
                .or_else(|| trimmed.strip_prefix("src/"))
                .unwrap_or(trimmed);
            for dir in ["src", "lib"] {
                for ext in ["tsx", "ts", "jsx", "js"] {
                    let candidate_rel = format!("{dir}/{stem}.{ext}");
                    let candidate_abs = repo_root.join(&candidate_rel);
                    if candidate_abs.is_file() {
                        if let Ok(content) = fs::read_to_string(&candidate_abs) {
                            return Some(ResolvedTargetSource {
                                relative_path: PathBuf::from(&candidate_rel),
                                absolute_path: candidate_abs,
                                confidence_score_bps: 7_500,
                                match_reason: format!(
                                    "Followed barrel re-export to source module: {candidate_rel}"
                                ),
                                source_content: content,
                            });
                        }
                    }
                }
            }
        }
        None
    }

    fn follow_html_script_src(
        repo_root: &Path,
        html_content: &str,
    ) -> Option<ResolvedTargetSource> {
        for line in html_content.lines() {
            if !line.to_ascii_lowercase().contains("<script") {
                continue;
            }
            for token in line.split(['\'', '"']) {
                let clean = token
                    .trim()
                    .trim_start_matches("./")
                    .trim_start_matches('/');
                if clean.is_empty()
                    || clean.contains(' ')
                    || clean.starts_with("http://")
                    || clean.starts_with("https://")
                {
                    continue;
                }
                if Self::SOURCE_EXTENSIONS.iter().any(|ext| {
                    *ext != "html" && clean.to_ascii_lowercase().ends_with(&format!(".{ext}"))
                }) {
                    let candidate_abs = repo_root.join(clean);
                    if candidate_abs.is_file() {
                        if let Ok(content) = fs::read_to_string(&candidate_abs) {
                            return Some(ResolvedTargetSource {
                                relative_path: PathBuf::from(clean),
                                absolute_path: candidate_abs,
                                confidence_score_bps: 8_500,
                                match_reason: format!(
                                    "Followed HTML <script src> entrypoint to source module: {clean}"
                                ),
                                source_content: content,
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_source_mapper_discovers_source_from_error_trace() {
        let tmp =
            std::env::temp_dir().join(format!("replayfix-discovery-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();

        let target_file = tmp.join("src").join("Cart.tsx");
        fs::write(
            &target_file,
            "export const Cart = () => { return <div>Cart</div>; };",
        )
        .unwrap();

        let query = SourceQuery {
            runtime_errors: &[
                "TypeError: item is undefined at http://127.0.0.1:3000/src/Cart.tsx:12:4"
                    .to_string(),
            ],
            dom_candidates: &[],
            selector: None,
            hint_tokens: &[],
        };

        let resolved = RepoSourceMapper::locate_target_file(&tmp, &query).unwrap();
        assert_eq!(
            resolved.relative_path,
            PathBuf::from("src").join("Cart.tsx")
        );
        assert_eq!(resolved.confidence_score_bps, 10_000);
        assert!(resolved.source_content.contains("export const Cart"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_repo_source_mapper_token_ranking() {
        let tmp = std::env::temp_dir().join(format!("replayfix-token-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("lib")).unwrap();

        fs::write(tmp.join("lib").join("Other.js"), "const unused = 1;").unwrap();
        fs::write(
            tmp.join("lib").join("DraggablePanel.tsx"),
            "export class DraggablePanel extends React.Component { render() { return <div className='draggable-panel-handle-nw' />; } }",
        )
        .unwrap();

        let query = SourceQuery {
            runtime_errors: &[],
            dom_candidates: &[],
            selector: Some(".draggable-panel-handle-nw"),
            hint_tokens: &["DraggablePanel"],
        };

        let resolved = RepoSourceMapper::locate_target_file(&tmp, &query).unwrap();
        assert_eq!(
            resolved.relative_path,
            PathBuf::from("lib").join("DraggablePanel.tsx")
        );
        assert!(resolved.confidence_score_bps > 1000);

        let _ = fs::remove_dir_all(&tmp);
    }
}

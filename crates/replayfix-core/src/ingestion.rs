use crate::error::{CoreError, Result};
use crate::manifest::TaskManifest;
use crate::platform::Platform;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

/// Maximum size allowed for a fetched GitHub issue payload (512 KB).
pub const MAX_ISSUE_PAYLOAD_BYTES: usize = 512 * 1024;

/// Reference to a specific GitHub issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubIssueRef {
    pub owner: String,
    pub repo: String,
    pub issue_number: u64,
}

static ISSUE_URL_REGEX: OnceLock<Option<Regex>> = OnceLock::new();

fn get_issue_url_regex() -> Option<&'static Regex> {
    ISSUE_URL_REGEX
        .get_or_init(|| {
            // Strict pattern matching https://github.com/{owner}/{repo}/issues/{id}
            // Disallows pull requests, commit hashes, subpaths, and arbitrary schemas.
            Regex::new(r"^https://github\.com/(?P<owner>[a-zA-Z0-9_.-]+)/(?P<repo>[a-zA-Z0-9_.-]+)/issues/(?P<number>\d+)/?$").ok()
        })
        .as_ref()
}

impl GitHubIssueRef {
    /// Parse and validate a GitHub issue URL string.
    pub fn parse_url(url: &str) -> Result<Self> {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Manifest(
                "GitHub issue URL cannot be empty".to_string(),
            ));
        }

        let regex = get_issue_url_regex().ok_or_else(|| {
            CoreError::Execution(
                "Failed to initialize GitHub issue URL regular expression".to_string(),
            )
        })?;
        let captures = regex.captures(trimmed).ok_or_else(|| {
            CoreError::Manifest(format!(
                "Invalid GitHub issue URL '{}': must match 'https://github.com/{{owner}}/{{repo}}/issues/{{number}}'",
                trimmed
            ))
        })?;

        let owner = captures
            .name("owner")
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| CoreError::Manifest("Missing owner in GitHub issue URL".to_string()))?;

        let repo = captures
            .name("repo")
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| CoreError::Manifest("Missing repo in GitHub issue URL".to_string()))?;

        // Sanity checks on owner and repo: prevent path traversal or dot segments
        if owner == "." || owner == ".." || repo == "." || repo == ".." {
            return Err(CoreError::SecurityViolation(
                "GitHub repository coordinates cannot be '.' or '..'".to_string(),
            ));
        }

        let number_str = captures
            .name("number")
            .map(|m| m.as_str())
            .ok_or_else(|| CoreError::Manifest("Missing issue number in URL".to_string()))?;

        let issue_number = number_str.parse::<u64>().map_err(|e| {
            CoreError::Manifest(format!("Invalid issue number '{}': {}", number_str, e))
        })?;

        if issue_number == 0 {
            return Err(CoreError::Manifest(
                "GitHub issue number must be greater than 0".to_string(),
            ));
        }

        Ok(Self {
            owner,
            repo,
            issue_number,
        })
    }

    /// Canonical GitHub issue web URL
    #[must_use]
    pub fn web_url(&self) -> String {
        format!(
            "https://github.com/{}/{}/issues/{}",
            self.owner, self.repo, self.issue_number
        )
    }

    /// Canonical GitHub REST API endpoint URL
    #[must_use]
    pub fn api_url(&self) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/issues/{}",
            self.owner, self.repo, self.issue_number
        )
    }
}

/// Raw issue payload retrieved from GitHub API or CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssuePayload {
    pub url: Option<String>,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: Option<String>,
    pub user: Option<GitHubUser>,
    pub labels: Option<Vec<GitHubLabel>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLabel {
    pub name: String,
}

/// Normalized result of an issue ingestion operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionResult {
    pub issue_ref: GitHubIssueRef,
    pub title: String,
    pub raw_body: String,
    pub extracted_steps: Vec<String>,
    pub untrusted_context_block: String,
    pub retrieved_at: String,
}

/// Trait abstracting issue retrieval to enable deterministic testing without network.
pub trait IssueFetcher: Send + Sync {
    fn fetch(&self, issue_ref: &GitHubIssueRef) -> Result<GitHubIssuePayload>;
}

/// System issue fetcher using `curl` or `gh` with secure timeouts and size limits.
#[derive(Debug, Default)]
pub struct SystemIssueFetcher;

impl IssueFetcher for SystemIssueFetcher {
    fn fetch(&self, issue_ref: &GitHubIssueRef) -> Result<GitHubIssuePayload> {
        let api_url = issue_ref.api_url();

        // Attempt 1: Fetch via curl using Platform::create_command
        let mut cmd = Platform::create_command("curl");
        cmd.arg("-s")
            .arg("-S")
            .arg("-L")
            .arg("--max-time")
            .arg("10")
            .arg("-H")
            .arg("User-Agent: ReplayFix-Harness/0.8.0")
            .arg("-H")
            .arg("Accept: application/vnd.github.v3+json")
            .arg(&api_url);

        let output = cmd.output().map_err(|e| {
            CoreError::Execution(format!("Failed to execute curl for issue fetching: {}", e))
        })?;

        if output.status.success() {
            let body_str = String::from_utf8_lossy(&output.stdout);
            if body_str.len() > MAX_ISSUE_PAYLOAD_BYTES {
                return Err(CoreError::Execution(format!(
                    "GitHub issue payload exceeds maximum limit of {} bytes (got {})",
                    MAX_ISSUE_PAYLOAD_BYTES,
                    body_str.len()
                )));
            }

            // Check for API rate limit or error responses
            if body_str.contains("API rate limit exceeded") {
                return Err(CoreError::Execution(
                    "GitHub API rate limit exceeded. Please configure GH_TOKEN or retry later."
                        .to_string(),
                ));
            }
            if body_str.contains("Not Found") && body_str.contains("documentation_url") {
                return Err(CoreError::Execution(format!(
                    "GitHub issue not found at: {}",
                    api_url
                )));
            }

            if let Ok(payload) = serde_json::from_str::<GitHubIssuePayload>(&body_str) {
                return Ok(payload);
            }
        }

        // Attempt 2: Fallback to gh CLI if installed
        let mut gh_cmd = Platform::create_command("gh");
        gh_cmd
            .arg("issue")
            .arg("view")
            .arg(issue_ref.web_url())
            .arg("--json")
            .arg("number,title,body,state,author,labels");

        if let Ok(gh_output) = gh_cmd.output() {
            if gh_output.status.success() {
                let gh_str = String::from_utf8_lossy(&gh_output.stdout);
                if let Ok(payload) = serde_json::from_str::<GitHubIssuePayload>(&gh_str) {
                    return Ok(payload);
                }
            }
        }

        Err(CoreError::Execution(format!(
            "Failed to fetch GitHub issue from '{}'. Check your network connection and issue visibility.",
            issue_ref.web_url()
        )))
    }
}

/// Mock issue fetcher for deterministic unit and integration tests.
#[derive(Debug, Clone)]
pub struct MockIssueFetcher {
    pub title: String,
    pub body: String,
}

impl MockIssueFetcher {
    #[must_use]
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }
}

impl IssueFetcher for MockIssueFetcher {
    fn fetch(&self, issue_ref: &GitHubIssueRef) -> Result<GitHubIssuePayload> {
        Ok(GitHubIssuePayload {
            url: Some(issue_ref.web_url()),
            number: issue_ref.issue_number,
            title: self.title.clone(),
            body: Some(self.body.clone()),
            state: Some("open".to_string()),
            user: Some(GitHubUser {
                login: "mockuser".to_string(),
            }),
            labels: Some(vec![GitHubLabel {
                name: "bug".to_string(),
            }]),
        })
    }
}

/// Extract reproduction steps from issue markdown text.
pub fn extract_repro_steps(body: &str) -> Vec<String> {
    let mut steps = Vec::new();
    let mut in_repro_section = false;

    for line in body.lines() {
        let trimmed = line.trim();

        // Check for headers signaling reproduction steps
        let lower = trimmed.to_lowercase();
        if lower.starts_with('#')
            && (lower.contains("step")
                || lower.contains("repro")
                || lower.contains("how to reproduce"))
        {
            in_repro_section = true;
            continue;
        } else if lower.starts_with('#') && in_repro_section {
            // Reached next section
            in_repro_section = false;
        }

        if in_repro_section || !in_repro_section {
            // Check for numbered list: "1. Click foo" or "1) Click foo"
            if let Some(rest) = parse_list_item(trimmed) {
                if in_repro_section || is_action_oriented(rest) {
                    steps.push(rest.to_string());
                }
            }
        }
    }

    // Deduplicate and cap steps to reasonable count (max 20)
    steps.truncate(20);
    steps
}

fn parse_list_item(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Numbered item: "1. ", "2. ", "1) "
    if let Some(pos) = trimmed.find(['.', ')']) {
        let prefix = &trimmed[..pos];
        if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
            let remainder = trimmed[pos + 1..].trim();
            if !remainder.is_empty() {
                return Some(remainder);
            }
        }
    }

    // Bullet item: "- ", "* "
    if (trimmed.starts_with("- ") || trimmed.starts_with("* ")) && trimmed.len() > 2 {
        return Some(trimmed[2..].trim());
    }

    None
}

fn is_action_oriented(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.starts_with("click")
        || lower.starts_with("open")
        || lower.starts_with("navigate")
        || lower.starts_with("press")
        || lower.starts_with("enter")
        || lower.starts_with("type")
        || lower.starts_with("select")
        || lower.starts_with("scroll")
        || lower.starts_with("go to")
}

/// Enclose issue text in protective untrusted context markers.
pub fn build_untrusted_context_block(
    issue_ref: &GitHubIssueRef,
    title: &str,
    raw_body: &str,
    steps: &[String],
) -> String {
    let mut block = String::new();
    block.push_str("<UNTRUSTED_ISSUE_METADATA>\n");
    block.push_str(&format!("Source: {}\n", issue_ref.web_url()));
    block.push_str(&format!("Title: {}\n", title));
    block.push_str("Extracted Reproduction Steps:\n");
    if steps.is_empty() {
        block.push_str("  (None automatically extracted)\n");
    } else {
        for (i, step) in steps.iter().enumerate() {
            block.push_str(&format!("  {}. {}\n", i + 1, step));
        }
    }
    block.push_str("Raw Body:\n");
    block.push_str(raw_body);
    block.push_str("\n</UNTRUSTED_ISSUE_METADATA>\n");
    block
}

/// High-level function to ingest a GitHub issue using an injected or default fetcher.
pub fn ingest_issue(url: &str, fetcher: &dyn IssueFetcher) -> Result<IngestionResult> {
    let issue_ref = GitHubIssueRef::parse_url(url)?;
    let payload = fetcher.fetch(&issue_ref)?;

    let raw_body = payload.body.unwrap_or_default();
    let extracted_steps = extract_repro_steps(&raw_body);
    let untrusted_context_block =
        build_untrusted_context_block(&issue_ref, &payload.title, &raw_body, &extracted_steps);

    let retrieved_at = chrono::Utc::now().to_rfc3339();

    Ok(IngestionResult {
        issue_ref,
        title: payload.title,
        raw_body,
        extracted_steps,
        untrusted_context_block,
        retrieved_at,
    })
}

/// Convert an IngestionResult into a TaskManifest for the ReplayFix verification engine.
pub fn ingestion_to_manifest(
    ingestion: &IngestionResult,
    repo_path: impl AsRef<Path>,
    bug_ref: Option<String>,
) -> TaskManifest {
    let task_id = format!(
        "GH-{}-{}",
        ingestion.issue_ref.repo, ingestion.issue_ref.issue_number
    );
    let repo_str = repo_path.as_ref().to_string_lossy().to_string();
    let bug_ref_str = bug_ref.unwrap_or_else(|| "HEAD".to_string());

    let mut expected = String::new();
    if !ingestion.extracted_steps.is_empty() {
        expected.push_str("Steps to reproduce:\n");
        for (i, s) in ingestion.extracted_steps.iter().enumerate() {
            expected.push_str(&format!("{}. {}\n", i + 1, s));
        }
    }

    TaskManifest {
        task_id,
        title: ingestion.title.clone(),
        category: Some("github-issue".to_string()),
        repo_path: repo_str,
        bug_ref: bug_ref_str,
        clean_ref: None,
        video_path: String::new(),
        audio_path: None,
        expected_behavior: if expected.is_empty() {
            None
        } else {
            Some(expected)
        },
        environment_adapter: None,
        headed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_github_issue_url() {
        let parsed =
            GitHubIssueRef::parse_url("https://github.com/owner-name/repo.name/issues/42").unwrap();
        assert_eq!(parsed.owner, "owner-name");
        assert_eq!(parsed.repo, "repo.name");
        assert_eq!(parsed.issue_number, 42);
        assert_eq!(
            parsed.web_url(),
            "https://github.com/owner-name/repo.name/issues/42"
        );
        assert_eq!(
            parsed.api_url(),
            "https://api.github.com/repos/owner-name/repo.name/issues/42"
        );
    }

    #[test]
    fn rejects_pull_requests_and_non_issues() {
        assert!(GitHubIssueRef::parse_url("https://github.com/owner/repo/pull/42").is_err());
        assert!(GitHubIssueRef::parse_url("https://github.com/owner/repo/commit/abc").is_err());
        assert!(GitHubIssueRef::parse_url("https://gitlab.com/owner/repo/issues/42").is_err());
        assert!(GitHubIssueRef::parse_url("http://github.com/owner/repo/issues/42").is_err());
        assert!(GitHubIssueRef::parse_url("https://github.com/owner/repo/issues/0").is_err());
    }

    #[test]
    fn extracts_steps_and_builds_untrusted_block() {
        let body = r#"
### Description
The counter overflows unexpectedly.

### Steps to Reproduce
1. Navigate to /counter
2. Click the increment button 5 times
3. Notice NaN appears

### Expected behavior
Counter should show 5.
"#;
        let fetcher = MockIssueFetcher::new("Counter NaN Bug", body);

        let result = ingest_issue("https://github.com/owner/repo/issues/101", &fetcher).unwrap();
        assert_eq!(result.title, "Counter NaN Bug");
        assert_eq!(result.extracted_steps.len(), 3);
        assert_eq!(result.extracted_steps[0], "Navigate to /counter");
        assert_eq!(
            result.extracted_steps[1],
            "Click the increment button 5 times"
        );
        assert_eq!(result.extracted_steps[2], "Notice NaN appears");
        assert!(result
            .untrusted_context_block
            .contains("<UNTRUSTED_ISSUE_METADATA>"));
        assert!(result
            .untrusted_context_block
            .contains("</UNTRUSTED_ISSUE_METADATA>"));

        let manifest = ingestion_to_manifest(&result, ".", None);
        assert_eq!(manifest.task_id, "GH-repo-101");
        assert_eq!(manifest.title, "Counter NaN Bug");
        assert!(manifest.expected_behavior.unwrap().contains("1. Navigate"));
    }
}

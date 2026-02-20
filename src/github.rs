//! GitHub REST API v3 client.
//!
//! All functions operate on public data and work without authentication.
//! Set `GITHUB_TOKEN` in the environment for higher rate limits (5 000/hr vs 60/hr)
//! and access to private repositories.

use anyhow::{Context, bail};
use serde::Deserialize;

const API_BASE: &str = "https://api.github.com";
const VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Response types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub company: Option<String>,
    pub blog: Option<String>,
    pub email: Option<String>,
    pub public_repos: u64,
    pub followers: u64,
    pub following: u64,
    pub created_at: String,
    pub html_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GitHubRepo {
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub pushed_at: Option<String>,
    pub updated_at: Option<String>,
    pub fork: bool,
    #[serde(default)]
    pub open_issues_count: u64,
    #[serde(default)]
    pub size: u64, // in KB
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubEvent {
    #[serde(rename = "type")]
    pub kind: String,
    pub repo: EventRepo,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct EventRepo {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitDetail {
    pub sha: String,
    pub html_url: String,
    pub commit: CommitInfo,
    #[serde(default)]
    pub files: Vec<CommitFile>,
}

#[derive(Debug, Deserialize)]
pub struct CommitInfo {
    pub message: String,
    pub author: CommitAuthor,
}

#[derive(Debug, Deserialize)]
pub struct CommitAuthor {
    pub name: String,
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitFile {
    pub filename: String,
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
    pub patch: Option<String>,
}

// ── Client helpers ──────────────────────────────────────────────────────────────

fn build_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(format!("gitprint/{VERSION}"))
        .build()
        .context("failed to build HTTP client")
}

fn auth_header(token: Option<&str>) -> Option<String> {
    token.map(|t| format!("Bearer {t}"))
}

async fn get_json<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    url: &str,
    token: Option<&str>,
) -> anyhow::Result<T> {
    let mut req = client
        .get(url)
        .header("Accept", "application/vnd.github+json");
    if let Some(auth) = auth_header(token) {
        req = req.header("Authorization", auth);
    }
    let resp = req.send().await.with_context(|| format!("GET {url}"))?;
    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        bail!("not found: {url}");
    }
    if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        bail!(
            "GitHub API rate limit exceeded. Set GITHUB_TOKEN to increase limits:\n  \
             export GITHUB_TOKEN=ghp_your_token_here"
        );
    }
    if !status.is_success() {
        bail!("GitHub API error {status}: {url}");
    }
    resp.json::<T>()
        .await
        .with_context(|| format!("parsing response from {url}"))
}

// ── Public API functions ────────────────────────────────────────────────────────

/// Fetch a user's public profile.
pub async fn get_user(username: &str, token: Option<&str>) -> anyhow::Result<GitHubUser> {
    let client = build_client()?;
    let url = format!("{API_BASE}/users/{username}");
    get_json::<GitHubUser>(&client, &url, token)
        .await
        .with_context(|| format!("fetching user '{username}'"))
}

/// Wrapper for the GitHub search/repositories response.
#[derive(Debug, Deserialize)]
struct SearchReposResponse {
    items: Vec<GitHubRepo>,
}

/// Fetch a user's top starred repositories via the Search API.
///
/// Uses `/search/repositories` because `/users/{u}/repos` does not support `sort=stars`.
pub async fn get_user_starred_repos(
    username: &str,
    limit: usize,
    token: Option<&str>,
) -> anyhow::Result<Vec<GitHubRepo>> {
    let client = build_client()?;
    let per_page = limit.min(100);
    let url = format!(
        "{API_BASE}/search/repositories?q=user:{username}+fork:false&sort=stars&order=desc&per_page={per_page}"
    );
    get_json::<SearchReposResponse>(&client, &url, token)
        .await
        .map(|r| r.items)
        .with_context(|| format!("fetching starred repos for '{username}'"))
}

/// Fetch a user's own repositories sorted by `sort` (`pushed` or `updated`).
///
/// `limit` is capped at 100 (GitHub's maximum per-page).
/// Only returns repos the user owns directly (`type=owner`).
pub async fn get_user_repos(
    username: &str,
    sort: &str,
    limit: usize,
    token: Option<&str>,
) -> anyhow::Result<Vec<GitHubRepo>> {
    let client = build_client()?;
    let per_page = limit.min(100);
    let url = format!(
        "{API_BASE}/users/{username}/repos?type=owner&sort={sort}&direction=desc&per_page={per_page}"
    );
    get_json::<Vec<GitHubRepo>>(&client, &url, token)
        .await
        .with_context(|| format!("fetching repos for '{username}' (sort={sort})"))
}

/// Fetch a user's recent public events (max 100, GitHub returns up to 90 days).
pub async fn get_user_events(
    username: &str,
    limit: usize,
    token: Option<&str>,
) -> anyhow::Result<Vec<GitHubEvent>> {
    let client = build_client()?;
    let per_page = limit.min(100);
    let url = format!("{API_BASE}/users/{username}/events/public?per_page={per_page}");
    get_json::<Vec<GitHubEvent>>(&client, &url, token)
        .await
        .with_context(|| format!("fetching events for '{username}'"))
}

/// Response envelope for the commits search endpoint.
#[derive(Deserialize)]
struct CommitSearchResponse {
    items: Vec<CommitSearchItem>,
}

#[derive(Deserialize)]
struct CommitSearchItem {
    sha: String,
    repository: CommitSearchRepo,
    commit: CommitSearchMeta,
}

#[derive(Deserialize)]
struct CommitSearchRepo {
    full_name: String,
}

#[derive(Deserialize)]
struct CommitSearchMeta {
    message: String,
}

/// Search for the `limit` most recent public commits authored by `username` across all repos.
///
/// Uses `GET /search/commits?q=author:{username}` (stable since GitHub API v3 2022+).
/// Returns `(owner/repo, sha, first-line-of-message)` tuples, newest first.
/// Returns an empty Vec on error so the caller can degrade gracefully.
pub async fn search_user_commits(
    username: &str,
    limit: usize,
    token: Option<&str>,
) -> anyhow::Result<Vec<(String, String, String)>> {
    let client = build_client()?;
    let per_page = limit.min(100);
    let url = format!(
        "{API_BASE}/search/commits?q=author:{username}&sort=committer-date&order=desc&per_page={per_page}"
    );
    get_json::<CommitSearchResponse>(&client, &url, token)
        .await
        .map(|r| {
            r.items
                .into_iter()
                .map(|item| {
                    let msg = item
                        .commit
                        .message
                        .lines()
                        .next()
                        .unwrap_or(&item.commit.message)
                        .to_string();
                    (item.repository.full_name, item.sha, msg)
                })
                .collect()
        })
        .with_context(|| format!("searching commits by '{username}'"))
}

/// Fetch a single commit with its file patches.
pub async fn get_commit_detail(
    owner_repo: &str,
    sha: &str,
    token: Option<&str>,
) -> anyhow::Result<CommitDetail> {
    let client = build_client()?;
    let url = format!("{API_BASE}/repos/{owner_repo}/commits/{sha}");
    get_json::<CommitDetail>(&client, &url, token)
        .await
        .with_context(|| format!("fetching commit {sha} in {owner_repo}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_header_some() {
        assert_eq!(auth_header(Some("tok")), Some("Bearer tok".to_string()));
    }

    #[test]
    fn auth_header_none() {
        assert_eq!(auth_header(None), None);
    }

    #[test]
    fn build_client_succeeds() {
        assert!(build_client().is_ok());
    }
}

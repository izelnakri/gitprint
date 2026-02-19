use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use anyhow::bail;
use tokio::process::Command;

use crate::types::{Config, RepoMetadata};

/// Returns `true` if `s` looks like a remote git URL.
///
/// Recognised schemes: `https://`, `http://`, `git://`, `ssh://`,
/// and SCP-style `git@host:path` used by GitHub/GitLab.
pub fn is_remote_url(s: &str) -> bool {
    s.starts_with("https://")
        || s.starts_with("http://")
        || s.starts_with("git://")
        || s.starts_with("ssh://")
        || (s.contains('@') && s.contains(':') && !s.starts_with('/'))
}

/// Extracts the repository name from a remote URL.
///
/// `https://github.com/user/repo.git` → `"repo"`
/// `git@github.com:user/repo`         → `"repo"`
pub fn repo_name_from_url(url: &str) -> String {
    url.split(['/', ':'])
        .next_back()
        .unwrap_or("repo")
        .trim_end_matches(".git")
        .to_string()
}

/// A temporary directory that deletes itself on drop.
pub struct TempCloneDir(PathBuf);

impl TempCloneDir {
    pub async fn new() -> anyhow::Result<Self> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("gitprint-{nanos}"));
        tokio::fs::create_dir_all(&dir).await?;
        Ok(Self(dir))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempCloneDir {
    fn drop(&mut self) {
        // Drop is synchronous by design — tokio async cannot be used here.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Clones a remote git repository into `dest`.
///
/// Uses `--depth=1` (shallow) for speed unless `commit` is specified, in which
/// case a full clone is required to access arbitrary history.
pub async fn clone_repo(
    url: &str,
    dest: &Path,
    branch: Option<&str>,
    commit: Option<&str>,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("clone");

    if commit.is_none() {
        cmd.arg("--depth=1");
        if let Some(b) = branch {
            cmd.args(["--branch", b, "--single-branch"]);
        }
    } else if let Some(b) = branch {
        cmd.args(["--branch", b]);
    }

    let status = cmd
        .arg(url)
        .arg(dest)
        .stderr(std::process::Stdio::inherit())
        .status()
        .await
        .map_err(|e| anyhow::anyhow!("failed to run git: {e}"))?;

    if !status.success() {
        bail!("git clone failed for {url}");
    }
    Ok(())
}

async fn run_git(repo_path: &Path, args: &[&str]) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["-C", &repo_path.to_string_lossy()])
        .args(args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{}", stderr.trim());
    }

    Ok(String::from_utf8(output.stdout)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
}

/// Describes what the user-supplied path resolves to.
#[derive(Debug)]
pub struct RepoInfo {
    /// Git repo root (git mode) or canonical directory path (plain-dir mode).
    pub root: PathBuf,
    /// Whether `root` is inside a git repository.
    pub is_git: bool,
    /// Subdirectory scope within the git repo (relative to `root`).
    /// Only set when the user supplied a strict subdirectory of the repo root.
    pub scope: Option<PathBuf>,
    /// When the user supplied a single file, its path relative to `root`.
    pub single_file: Option<PathBuf>,
}

/// Resolves a user-supplied path into a [`RepoInfo`].
///
/// Handles four cases:
///
/// - File inside a git repo → `single_file` is set, `root` is the repo root.
/// - Subdirectory inside a git repo → `scope` is set relative to `root`.
/// - Git repo root → `root` is the repo root, no scope.
/// - Plain directory or file outside git → `is_git` is `false`.
///
/// # Errors
///
/// Returns an error if the path does not exist.
///
/// # Examples
///
/// ```ignore
/// use gitprint::git::verify_repo;
/// use std::path::Path;
///
/// let info = verify_repo(Path::new(".")).await.unwrap();
/// println!("repo root: {}", info.root.display());
/// println!("is git: {}", info.is_git);
/// ```
pub async fn verify_repo(path: &Path) -> anyhow::Result<RepoInfo> {
    // Use async canonicalize to avoid blocking tokio worker threads.
    let canonical = tokio::fs::canonicalize(path)
        .await
        .map_err(|_| anyhow::anyhow!("{}: path not found", path.display()))?;

    // Fetch metadata once (async stat) and reuse is_file/is_dir throughout —
    // avoids multiple blocking stat() calls on the same already-resolved path.
    let meta = tokio::fs::metadata(&canonical)
        .await
        .map_err(|_| anyhow::anyhow!("{}: cannot stat path", canonical.display()))?;
    let is_file = meta.is_file();
    let is_dir = meta.is_dir();

    // Git must be invoked from a directory; use parent when the path is a file.
    let git_dir = if is_file {
        canonical
            .parent()
            .ok_or_else(|| anyhow::anyhow!("file has no parent directory"))?
            .to_path_buf()
    } else {
        canonical.clone()
    };

    let output = Command::new("git")
        .args(["-C", &git_dir.to_string_lossy()])
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("failed to run git: {e}"))?;

    if output.status.success() {
        let root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim().to_string());

        if is_file {
            let rel = canonical
                .strip_prefix(&root)
                .map_err(|_| anyhow::anyhow!("file is outside the git repository"))?
                .to_path_buf();
            return Ok(RepoInfo {
                root,
                is_git: true,
                scope: None,
                single_file: Some(rel),
            });
        }

        let scope = (canonical != root)
            .then(|| canonical.strip_prefix(&root).ok().map(|p| p.to_path_buf()))
            .flatten();
        return Ok(RepoInfo {
            root,
            is_git: true,
            scope,
            single_file: None,
        });
    }

    // Not inside a git repo.
    if is_file {
        let parent = canonical
            .parent()
            .ok_or_else(|| anyhow::anyhow!("file has no parent directory"))?
            .to_path_buf();
        return Ok(RepoInfo {
            root: parent,
            is_git: false,
            scope: None,
            single_file: Some(PathBuf::from(canonical.file_name().unwrap())),
        });
    }

    if is_dir {
        return Ok(RepoInfo {
            root: canonical,
            is_git: false,
            scope: None,
            single_file: None,
        });
    }

    bail!(
        "{}: not a git repository, directory, or file",
        path.display()
    )
}

/// Fetches repository metadata: branch, last commit hash/date/message, and name.
///
/// For non-git directories, returns a `RepoMetadata` with empty git fields.
/// Branch detection and commit log are fetched concurrently.
///
/// # Errors
///
/// Returns an error if the git command fails (git repos only).
pub async fn get_metadata(
    repo_path: &Path,
    config: &Config,
    is_git: bool,
    scope: Option<&Path>,
) -> anyhow::Result<RepoMetadata> {
    let base = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let name = match scope {
        Some(s) => format!("{}/{}", base, s.display()),
        None => base,
    };

    if !is_git {
        return Ok(RepoMetadata {
            name,
            branch: String::new(),
            commit_hash: String::new(),
            commit_hash_short: String::new(),
            commit_date: String::new(),
            commit_message: String::new(),
            commit_author: String::new(),
            commit_author_email: String::new(),
            file_count: 0,
            total_lines: 0,
            fs_owner: None,
            fs_group: None,
            generated_at: String::new(),
            repo_size: String::new(),
            fs_size: String::new(),
            detected_remote_url: None,
            repo_absolute_path: None,
        });
    }

    let rev = match (&config.commit, &config.branch) {
        (Some(c), _) => c.clone(),
        (_, Some(b)) => b.clone(),
        _ => "HEAD".to_string(),
    };

    // Run branch detection, commit log, and remote URL detection in parallel.
    // Format: hash, date, subject, author name, author email (one per line, %n separated).
    let log_args = ["log", "-1", "--format=%H%n%ci%n%s%n%an%n%ae", &rev];
    let (branch, log_output, detected_remote_url) = tokio::join!(
        async {
            match &config.branch {
                Some(b) => b.clone(),
                None => run_git(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
                    .await
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "detached".to_string()),
            }
        },
        run_git(repo_path, &log_args),
        git_remote_url(repo_path),
    );
    let log_output = log_output?;

    let mut lines = log_output.trim().lines();
    let commit_hash = lines.next().unwrap_or("").to_string();
    let commit_hash_short = commit_hash[..7.min(commit_hash.len())].to_string();
    let commit_date = lines.next().unwrap_or("").to_string();
    // Remaining: subject lines, then author name, then author email (last two lines).
    let remaining: Vec<&str> = lines.collect();
    let (commit_message, commit_author, commit_author_email) = match remaining.as_slice() {
        [] => (String::new(), String::new(), String::new()),
        [.., author, email] => {
            let subject_lines = &remaining[..remaining.len().saturating_sub(2)];
            (
                subject_lines.join("\n"),
                author.to_string(),
                email.to_string(),
            )
        }
        [author] => (String::new(), author.to_string(), String::new()),
    };

    Ok(RepoMetadata {
        name,
        branch,
        commit_hash,
        commit_hash_short,
        commit_date,
        commit_message,
        commit_author,
        commit_author_email,
        file_count: 0,
        total_lines: 0,
        fs_owner: None,
        fs_group: None,
        generated_at: String::new(),
        repo_size: String::new(),
        fs_size: String::new(),
        detected_remote_url,
        repo_absolute_path: None,
    })
}

/// Lists all files to be included in the PDF.
///
/// In git mode: uses `git ls-files` (working tree) or `git ls-tree` (specific
/// branch/commit). In plain-directory mode: recursively walks the filesystem.
///
/// # Errors
///
/// Returns an error if the git command or directory walk fails.
pub async fn list_tracked_files(
    repo_path: &Path,
    config: &Config,
    is_git: bool,
    scope: Option<&Path>,
) -> anyhow::Result<Vec<PathBuf>> {
    if !is_git {
        return walk_files_async(repo_path.to_path_buf()).await;
    }

    let scope_str = scope.and_then(|p| p.to_str());
    let output = match (&config.commit, &config.branch) {
        (Some(commit), _) => match scope_str {
            Some(s) => {
                run_git(
                    repo_path,
                    &["ls-tree", "-r", "--name-only", commit, "--", s],
                )
                .await?
            }
            None => run_git(repo_path, &["ls-tree", "-r", "--name-only", commit]).await?,
        },
        (_, Some(branch)) => match scope_str {
            Some(s) => {
                run_git(
                    repo_path,
                    &["ls-tree", "-r", "--name-only", branch, "--", s],
                )
                .await?
            }
            None => run_git(repo_path, &["ls-tree", "-r", "--name-only", branch]).await?,
        },
        _ => match scope_str {
            Some(s) => run_git(repo_path, &["ls-files", "--", s]).await?,
            None => run_git(repo_path, &["ls-files"]).await?,
        },
    };

    Ok(output
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect())
}

/// Returns a map of file path → last modified date (YYYY-MM-DD).
/// In git mode: parsed from `git log`. In directory mode: from filesystem mtime.
pub async fn file_last_modified_dates(
    repo_path: &Path,
    config: &Config,
    is_git: bool,
    scope: Option<&Path>,
) -> anyhow::Result<HashMap<PathBuf, String>> {
    if !is_git {
        return walk_dates_async(repo_path.to_path_buf()).await;
    }

    let rev = match (&config.commit, &config.branch) {
        (Some(c), _) => c.clone(),
        (_, Some(b)) => b.clone(),
        _ => "HEAD".to_string(),
    };

    let scope_str = scope.and_then(|p| p.to_str());
    let output = match scope_str {
        Some(s) => {
            run_git(
                repo_path,
                &["log", "--format=COMMIT:%ci", "--name-only", &rev, "--", s],
            )
            .await?
        }
        None => {
            run_git(
                repo_path,
                &["log", "--format=COMMIT:%ci", "--name-only", &rev],
            )
            .await?
        }
    };

    let mut map = HashMap::new();
    let mut current_date = String::new();

    output.lines().for_each(|line| {
        if let Some(date_str) = line.strip_prefix("COMMIT:") {
            current_date = date_str.chars().take(10).collect();
        } else if !line.is_empty() && !current_date.is_empty() {
            map.entry(PathBuf::from(line))
                .or_insert_with(|| current_date.clone());
        }
    });

    Ok(map)
}

/// Returns the last-modified date (YYYY-MM-DD) for a single file.
/// In git mode: from `git log`. In plain mode: from filesystem mtime.
pub async fn file_last_modified(root: &Path, file: &Path, config: &Config, is_git: bool) -> String {
    if is_git {
        let rev = config
            .commit
            .as_deref()
            .or(config.branch.as_deref())
            .unwrap_or("HEAD");
        let file_str = file.to_string_lossy();
        run_git(
            root,
            &["log", "-1", "--format=%ci", rev, "--", file_str.as_ref()],
        )
        .await
        .ok()
        .map(|s| s.trim().chars().take(10).collect())
        .unwrap_or_default()
    } else {
        tokio::fs::metadata(root.join(file))
            .await
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                let (y, m, d) = unix_secs_to_ymd(secs);
                format!("{y:04}-{m:02}-{d:02}")
            })
            .unwrap_or_default()
    }
}

pub async fn read_file_content(
    repo_path: &Path,
    file_path: &Path,
    config: &Config,
) -> anyhow::Result<String> {
    let rev = config.commit.as_deref().or(config.branch.as_deref());
    match rev {
        Some(rev) => {
            let spec = format!("{rev}:{}", file_path.display());
            run_git(repo_path, &["show", &spec]).await
        }
        None => tokio::fs::read_to_string(repo_path.join(file_path))
            .await
            .map_err(Into::into),
    }
}

// ── Private helpers for plain-directory mode ──────────────────────────────────

/// Converts Unix timestamp (seconds since epoch) to (year, month, day).
/// Uses Howard Hinnant's date algorithm.
fn unix_secs_to_ymd(secs: u64) -> (u32, u32, u32) {
    let z = (secs / 86400) as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m, d)
}

/// Recursive async walk returning all file paths relative to `root`.
/// Each directory immediately spawns tasks for its subdirectories — no
/// level-by-level BFS barriers, maximum concurrency throughout the tree.
fn walk_files_inner(
    root: Arc<PathBuf>,
    dir: PathBuf,
) -> Pin<Box<dyn Future<Output = anyhow::Result<Vec<PathBuf>>> + Send>> {
    Box::pin(async move {
        let mut rd = tokio::fs::read_dir(&dir).await?;
        let mut files: Vec<PathBuf> = Vec::new();
        let mut set: tokio::task::JoinSet<anyhow::Result<Vec<PathBuf>>> =
            tokio::task::JoinSet::new();

        while let Some(entry) = rd.next_entry().await? {
            let ft = entry.file_type().await?;
            if ft.is_dir() {
                set.spawn(walk_files_inner(Arc::clone(&root), entry.path()));
            } else if ft.is_file()
                && let Ok(rel) = entry.path().strip_prefix(root.as_ref())
            {
                files.push(rel.to_path_buf());
            }
        }

        set.join_all()
            .await
            .into_iter()
            .try_for_each(|res| res.map(|sub| files.extend(sub)))?;

        Ok(files)
    })
}

async fn walk_files_async(root: PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    walk_files_inner(Arc::new(root.clone()), root).await
}

/// Walk the tree (via `walk_files_async`) then fetch all file mtimes concurrently.
async fn walk_dates_async(root: PathBuf) -> anyhow::Result<HashMap<PathBuf, String>> {
    let files = walk_files_async(root.clone()).await?;
    let mut set: tokio::task::JoinSet<Option<(PathBuf, String)>> = tokio::task::JoinSet::new();

    files.into_iter().for_each(|rel| {
        let abs = root.join(&rel);
        set.spawn(async move {
            let date = tokio::fs::metadata(&abs)
                .await
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let secs = t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                    let (y, m, d) = unix_secs_to_ymd(secs);
                    format!("{y:04}-{m:02}-{d:02}")
                })?;
            Some((rel, date))
        });
    });

    Ok(set.join_all().await.into_iter().flatten().collect())
}

/// Returns the filesystem owner username and group name for `path`.
///
/// Tries GNU `stat -c "%U\n%G"` (Linux/coreutils) then BSD `stat -f "%Su\n%Sg"` (macOS).
/// Returns `(None, None)` if both fail or the path is inaccessible.
pub async fn fs_owner_group(path: &Path) -> (Option<String>, Option<String>) {
    for args in [
        &["-c", "%U\n%G"][..],   // GNU stat (Linux)
        &["-f", "%Su\n%Sg"][..], // BSD stat (macOS)
    ] {
        if let Ok(out) = Command::new("stat").args(args).arg(path).output().await {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                let mut lines = text.trim().lines();
                return (
                    lines.next().filter(|s| !s.is_empty()).map(str::to_string),
                    lines.next().filter(|s| !s.is_empty()).map(str::to_string),
                );
            }
        }
    }
    (None, None)
}

/// Returns the filesystem disk usage of `path` as a human-readable string (e.g. `"4.2 MB"`).
///
/// Uses `du -sh` which is available on both Linux and macOS.
/// Falls back to an empty string on failure.
pub async fn fs_size(path: &Path) -> String {
    Command::new("du")
        .args(["-sh", &path.to_string_lossy()])
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .and_then(|s| s.split_whitespace().next().map(str::to_string))
        })
        .unwrap_or_default()
}

/// Returns the total size of all git-tracked files as a human-readable string (e.g. `"3.8 MB"`).
///
/// Uses `git ls-tree -r -l` to sum blob sizes from the object database — this
/// reflects actual tracked content without `.git/` overhead.
/// Falls back to an empty string on failure.
pub async fn git_tracked_size(repo_path: &Path, config: &Config) -> String {
    let rev = config
        .commit
        .as_deref()
        .or(config.branch.as_deref())
        .unwrap_or("HEAD");
    let output = run_git(repo_path, &["ls-tree", "-r", "-l", rev])
        .await
        .unwrap_or_default();
    let total_bytes: u64 = output
        .lines()
        .filter_map(|line| line.split_whitespace().nth(3)?.parse::<u64>().ok())
        .sum();
    if total_bytes == 0 {
        return String::new();
    }
    if total_bytes < 1024 {
        format!("{total_bytes} B")
    } else if total_bytes < 1_048_576 {
        format!("{:.1} KB", total_bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", total_bytes as f64 / 1_048_576.0)
    }
}

/// Normalizes a git remote URL to an `https://` URL.
///
/// Handles SCP-style (`git@github.com:user/repo`) and `ssh://` URLs, converting
/// them to their `https://` equivalents so they can be used in clickable links.
/// Plain `https://` or `http://` URLs are returned unchanged.
fn normalize_to_https(url: &str) -> String {
    if url.starts_with("https://") || url.starts_with("http://") {
        return url.to_string();
    } else if let Some(rest) = url.strip_prefix("git@") {
        // SCP-style: git@github.com:user/repo.git
        if let Some(colon_pos) = rest.find(':') {
            return format!("https://{}/{}", &rest[..colon_pos], &rest[colon_pos + 1..]);
        }
    } else if let Some(rest) = url.strip_prefix("ssh://git@") {
        return format!("https://{rest}");
    } else if let Some(rest) = url.strip_prefix("ssh://") {
        return format!("https://{rest}");
    }
    url.to_string()
}

/// Returns the remote URL for `origin`, if one is configured.
///
/// Runs `git remote get-url origin` — if the repo has no remote or the command
/// fails, returns `None`. SCP-style and ssh:// URLs are normalized to https://.
pub async fn git_remote_url(repo_path: &Path) -> Option<String> {
    run_git(repo_path, &["remote", "get-url", "origin"])
        .await
        .ok()
        .map(|s| normalize_to_https(s.trim()))
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_https_passthrough() {
        assert_eq!(
            normalize_to_https("https://github.com/user/repo.git"),
            "https://github.com/user/repo.git"
        );
        assert_eq!(
            normalize_to_https("http://example.com/repo"),
            "http://example.com/repo"
        );
    }

    #[test]
    fn normalize_scp_style() {
        assert_eq!(
            normalize_to_https("git@github.com:user/repo.git"),
            "https://github.com/user/repo.git"
        );
        assert_eq!(
            normalize_to_https("git@gitlab.com:org/project"),
            "https://gitlab.com/org/project"
        );
    }

    #[test]
    fn normalize_ssh_git_at_style() {
        assert_eq!(
            normalize_to_https("ssh://git@github.com/user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn normalize_ssh_style() {
        assert_eq!(
            normalize_to_https("ssh://github.com/user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }

    #[test]
    fn is_remote_url_https() {
        assert!(is_remote_url("https://github.com/user/repo"));
        assert!(is_remote_url("https://github.com/user/repo.git"));
        assert!(is_remote_url("http://example.com/repo.git"));
    }

    #[test]
    fn is_remote_url_git_schemes() {
        assert!(is_remote_url("git://github.com/user/repo.git"));
        assert!(is_remote_url("ssh://git@github.com/user/repo.git"));
    }

    #[test]
    fn is_remote_url_scp_style() {
        assert!(is_remote_url("git@github.com:user/repo.git"));
        assert!(is_remote_url("git@gitlab.com:org/repo"));
    }

    #[test]
    fn is_remote_url_rejects_local() {
        assert!(!is_remote_url("."));
        assert!(!is_remote_url("/home/user/repo"));
        assert!(!is_remote_url("relative/path"));
        assert!(!is_remote_url("src/main.rs"));
    }

    #[test]
    fn repo_name_from_url_https() {
        assert_eq!(
            repo_name_from_url("https://github.com/user/repo.git"),
            "repo"
        );
        assert_eq!(repo_name_from_url("https://github.com/user/repo"), "repo");
    }

    #[test]
    fn repo_name_from_url_scp() {
        assert_eq!(repo_name_from_url("git@github.com:user/repo.git"), "repo");
        assert_eq!(
            repo_name_from_url("git@gitlab.com:org/myproject"),
            "myproject"
        );
    }

    #[tokio::test]
    async fn temp_clone_dir_creates_and_cleans_up() {
        let path = {
            let t = TempCloneDir::new().await.unwrap();
            let p = t.path().to_path_buf();
            assert!(p.exists());
            p
        };
        assert!(!path.exists());
    }
}

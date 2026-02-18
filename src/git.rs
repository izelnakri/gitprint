use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use anyhow::bail;
use tokio::process::Command;

use crate::types::{Config, RepoMetadata};

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
            file_count: 0,
            total_lines: 0,
        });
    }

    let rev = match (&config.commit, &config.branch) {
        (Some(c), _) => c.clone(),
        (_, Some(b)) => b.clone(),
        _ => "HEAD".to_string(),
    };

    // Run branch detection and commit log in parallel — both are independent git calls.
    let log_args = ["log", "-1", "--format=%H%n%ci%n%s", &rev];
    let (branch, log_output) = tokio::join!(
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
    );
    let log_output = log_output?;

    let mut lines = log_output.trim().lines();
    let commit_hash = lines.next().unwrap_or("").to_string();
    let commit_hash_short = commit_hash[..7.min(commit_hash.len())].to_string();
    let commit_date = lines.next().unwrap_or("").to_string();
    let commit_message = lines.collect::<Vec<_>>().join("\n");

    Ok(RepoMetadata {
        name,
        branch,
        commit_hash,
        commit_hash_short,
        commit_date,
        commit_message,
        file_count: 0,
        total_lines: 0,
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

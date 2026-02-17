use std::path::{Path, PathBuf};

use tokio::process::Command;

use crate::error::Error;
use crate::types::{Config, RepoMetadata};

async fn run_git(repo_path: &Path, args: &[&str]) -> Result<String, Error> {
    let output = Command::new("git")
        .args(["-C", &repo_path.to_string_lossy()])
        .args(args)
        .output()
        .await
        .map_err(|e| Error::Git(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(stderr.trim().to_string()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn verify_repo(path: &Path) -> Result<PathBuf, Error> {
    let output = run_git(path, &["rev-parse", "--show-toplevel"]).await?;
    Ok(PathBuf::from(output.trim()))
}

pub async fn get_metadata(repo_path: &Path, config: &Config) -> Result<RepoMetadata, Error> {
    let name = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let rev = match (&config.commit, &config.branch) {
        (Some(c), _) => c.clone(),
        (_, Some(b)) => b.clone(),
        _ => "HEAD".to_string(),
    };

    let branch = match &config.branch {
        Some(b) => b.clone(),
        None => run_git(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
            .await
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "detached".to_string()),
    };

    // Single git call for commit hash, date, and message
    let log_output = run_git(repo_path, &["log", "-1", "--format=%H%n%ci%n%s", &rev]).await?;

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

pub async fn list_tracked_files(repo_path: &Path, config: &Config) -> Result<Vec<PathBuf>, Error> {
    let output = match (&config.commit, &config.branch) {
        (Some(commit), _) => run_git(repo_path, &["ls-tree", "-r", "--name-only", commit]).await?,
        (_, Some(branch)) => run_git(repo_path, &["ls-tree", "-r", "--name-only", branch]).await?,
        _ => run_git(repo_path, &["ls-files"]).await?,
    };

    Ok(output
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect())
}

pub async fn read_file_content(
    repo_path: &Path,
    file_path: &Path,
    config: &Config,
) -> Result<String, Error> {
    let rev = config.commit.as_deref().or(config.branch.as_deref());
    match rev {
        Some(rev) => {
            let spec = format!("{rev}:{}", file_path.display());
            run_git(repo_path, &["show", &spec]).await
        }
        None => tokio::fs::read_to_string(repo_path.join(file_path))
            .await
            .map_err(Error::Io),
    }
}

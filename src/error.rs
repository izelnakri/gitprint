use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a git repository: {0}")]
    NotARepo(PathBuf),

    #[error("git command failed: {0}")]
    Git(String),

    #[error("branch not found: {0}")]
    BranchNotFound(String),

    #[error("commit not found: {0}")]
    CommitNotFound(String),

    #[error("theme not found: {0} (use --list-themes to see available themes)")]
    ThemeNotFound(String),

    #[error("filter pattern error: {0}")]
    Filter(String),

    #[error("pdf generation failed: {0}")]
    Pdf(String),

    #[error("font loading failed: {0}")]
    Font(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

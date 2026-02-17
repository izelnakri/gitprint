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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_not_a_repo() {
        let err = Error::NotARepo(PathBuf::from("/tmp/foo"));
        assert_eq!(err.to_string(), "not a git repository: /tmp/foo");
    }

    #[test]
    fn display_git_error() {
        let err = Error::Git("fatal: bad revision".to_string());
        assert_eq!(err.to_string(), "git command failed: fatal: bad revision");
    }

    #[test]
    fn display_branch_not_found() {
        let err = Error::BranchNotFound("feature-x".to_string());
        assert!(err.to_string().contains("feature-x"));
    }

    #[test]
    fn display_commit_not_found() {
        let err = Error::CommitNotFound("deadbeef".to_string());
        assert!(err.to_string().contains("deadbeef"));
    }

    #[test]
    fn display_theme_not_found() {
        let err = Error::ThemeNotFound("BadTheme".to_string());
        let msg = err.to_string();
        assert!(msg.contains("BadTheme"));
        assert!(msg.contains("--list-themes"));
    }

    #[test]
    fn display_filter_error() {
        let err = Error::Filter("invalid glob".to_string());
        assert!(err.to_string().contains("invalid glob"));
    }

    #[test]
    fn display_pdf_error() {
        let err = Error::Pdf("render failed".to_string());
        assert!(err.to_string().contains("render failed"));
    }

    #[test]
    fn display_font_error() {
        let err = Error::Font("bad ttf".to_string());
        assert!(err.to_string().contains("bad ttf"));
    }

    #[test]
    fn io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn all_variants_are_debug() {
        let variants: Vec<Box<dyn std::fmt::Debug>> = vec![
            Box::new(Error::NotARepo(PathBuf::from("/tmp"))),
            Box::new(Error::Git("fail".into())),
            Box::new(Error::BranchNotFound("x".into())),
            Box::new(Error::CommitNotFound("x".into())),
            Box::new(Error::ThemeNotFound("x".into())),
            Box::new(Error::Filter("x".into())),
            Box::new(Error::Pdf("x".into())),
            Box::new(Error::Font("x".into())),
            Box::new(Error::Io(std::io::Error::other("x"))),
        ];
        for v in &variants {
            let debug = format!("{v:?}");
            assert!(!debug.is_empty());
        }
    }
}

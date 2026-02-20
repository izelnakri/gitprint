use std::path::PathBuf;

/// Activity filter for the user report event feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ActivityFilter {
    /// Show all event types (pushes, PRs, issues, stars, etc.)
    All,
    /// Show only push events (commits to repos)
    Commits,
}

/// Configuration for a `gitprint user` run.
#[derive(Debug, Clone)]
pub struct UserReportConfig {
    pub username: String,
    pub output_path: PathBuf,
    pub paper_size: PaperSize,
    pub landscape: bool,
    /// Number of most-recently-pushed repos to include (0 = skip section).
    pub last_committed: usize,
    /// Number of recent commits with diffs to render (0 = skip diffs).
    pub commits: usize,
    /// Skip diff rendering entirely.
    pub no_diffs: bool,
    /// Font size used for diff/code blocks.
    pub font_size: f64,
    /// GitHub personal access token (`GITHUB_TOKEN` env var).
    pub github_token: Option<String>,
    /// Earliest date to include events from, in `YYYY-MM-DD` form (`None` = no lower bound).
    pub since: Option<String>,
    /// Latest date to include events from, in `YYYY-MM-DD` form (`None` = no upper bound).
    pub until: Option<String>,
    /// Which event types to include in the report.
    pub activity: ActivityFilter,
    /// Maximum number of events to show in the activity feed.
    pub events: usize,
}

/// Paper size for PDF output.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum PaperSize {
    A4,
    Letter,
    Legal,
}

/// Configuration for a gitprint run.
#[derive(Debug, Clone)]
pub struct Config {
    pub repo_path: PathBuf,
    pub output_path: PathBuf,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub theme: String,
    pub font_size: f64,
    pub no_line_numbers: bool,
    pub toc: bool,
    pub file_tree: bool,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub paper_size: PaperSize,
    pub landscape: bool,
    /// Original remote URL when input was a remote repository, used for GitHub links.
    pub remote_url: Option<String>,
}

impl Config {
    #[cfg(test)]
    pub(crate) fn test_default() -> Self {
        Self {
            repo_path: PathBuf::from("."),
            output_path: PathBuf::from("/tmp/gitprint-test.pdf"),
            include_patterns: vec![],
            exclude_patterns: vec![],
            theme: "InspiredGitHub".to_string(),
            font_size: 8.0,
            no_line_numbers: false,
            toc: true,
            file_tree: true,
            branch: None,
            commit: None,
            paper_size: PaperSize::A4,
            landscape: false,
            remote_url: None,
        }
    }
}

/// Metadata extracted from a git repository.
#[derive(Debug, Clone)]
pub struct RepoMetadata {
    pub name: String,
    pub branch: String,
    pub commit_hash: String,
    pub commit_hash_short: String,
    pub commit_date: String,
    pub commit_message: String,
    pub commit_author: String,
    /// Email address of the last committer.
    pub commit_author_email: String,
    pub file_count: usize,
    pub total_lines: usize,
    /// Filesystem owner of the input path (local paths only).
    pub fs_owner: Option<String>,
    /// Filesystem group of the input path (local paths only).
    pub fs_group: Option<String>,
    /// UTC timestamp when this PDF was generated.
    pub generated_at: String,
    /// Human-readable size of the git-tracked content (e.g. "4.2 MB").
    /// Computed from `git ls-tree -r -l`; empty for non-git paths.
    pub repo_size: String,
    /// Human-readable filesystem disk usage of the input path (e.g. "5.1 MB").
    /// Computed from `du -sh`; empty for remote repos.
    pub fs_size: String,
    /// Remote URL detected from git config (e.g. `git remote get-url origin`).
    /// Used to generate commit/author links even when `Config::remote_url` is None.
    pub detected_remote_url: Option<String>,
    /// Absolute filesystem path to the repo root (local repos only, `None` for remote clones).
    /// Used to generate `file://` links on the cover page.
    pub repo_absolute_path: Option<PathBuf>,
}

/// An RGB color value.
#[derive(Debug, Clone, Copy)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A single syntax-highlighted token with styling information.
#[derive(Debug, Clone)]
pub struct HighlightedToken {
    pub text: String,
    pub color: RgbColor,
    pub bold: bool,
    pub italic: bool,
}

/// A line of syntax-highlighted tokens.
#[derive(Debug, Clone)]
pub struct HighlightedLine {
    pub line_number: usize,
    pub tokens: Vec<HighlightedToken>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_test_default() {
        let config = Config::test_default();
        assert_eq!(config.repo_path, PathBuf::from("."));
        assert_eq!(config.theme, "InspiredGitHub");
        assert_eq!(config.font_size, 8.0);
        assert!(config.toc);
        assert!(config.file_tree);
        assert!(!config.no_line_numbers);
        assert!(!config.landscape);
        assert!(config.branch.is_none());
        assert!(config.commit.is_none());
    }

    #[test]
    fn test_repo_metadata_clone() {
        let meta = RepoMetadata {
            name: "test".to_string(),
            branch: "main".to_string(),
            commit_hash: "abc123".to_string(),
            commit_hash_short: "abc1234".to_string(),
            commit_date: "2024-01-01".to_string(),
            commit_message: "init".to_string(),
            commit_author: "Alice".to_string(),
            commit_author_email: "alice@example.com".to_string(),
            file_count: 10,
            total_lines: 500,
            fs_owner: None,
            fs_group: None,
            generated_at: "2024-01-15 10:00:00 UTC".to_string(),
            repo_size: "1.2 MB".to_string(),
            fs_size: "1.5 MB".to_string(),
            detected_remote_url: None,
            repo_absolute_path: None,
        };
        let cloned = meta.clone();
        assert_eq!(cloned.name, "test");
        assert_eq!(cloned.file_count, 10);
    }

    #[test]
    fn test_rgb_color_copy() {
        let color = RgbColor {
            r: 255,
            g: 128,
            b: 0,
        };
        let copied = color;
        assert_eq!(copied.r, 255);
        assert_eq!(copied.g, 128);
        assert_eq!(copied.b, 0);
        // Original still usable (Copy trait)
        assert_eq!(color.r, 255);
    }

    #[test]
    fn test_highlighted_line_structure() {
        let line = HighlightedLine {
            line_number: 42,
            tokens: vec![
                HighlightedToken {
                    text: "fn".to_string(),
                    color: RgbColor { r: 0, g: 0, b: 255 },
                    bold: true,
                    italic: false,
                },
                HighlightedToken {
                    text: " main".to_string(),
                    color: RgbColor { r: 0, g: 0, b: 0 },
                    bold: false,
                    italic: false,
                },
            ],
        };
        assert_eq!(line.line_number, 42);
        assert_eq!(line.tokens.len(), 2);
        assert!(line.tokens[0].bold);
        assert!(!line.tokens[1].bold);
    }
}

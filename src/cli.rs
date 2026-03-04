use std::path::PathBuf;

use clap::Parser;

use crate::types::{ActivityFilter, PaperSize};

#[derive(Parser, Debug)]
#[command(
    name = "gitprint",
    about = "Convert git repositories into beautifully formatted PDFs",
    long_about = "Convert git repositories into beautifully formatted PDFs.\n\
                  \n\
                  MODES\n\
                  \n  \
                  gitprint <PATH> [OPTIONS]\n    \
                    Local path, file, or remote URL (https://, git@, ssh://) → PDF\n\
                  \n  \
                  gitprint --user <USERNAME> [OPTIONS]\n    \
                    GitHub user activity report → PDF\n\
                  \n  \
                  gitprint <PATH|--user USERNAME> --preview\n    \
                    Preview output in the terminal — no PDF generated",
    version,
    arg_required_else_help = true,
    after_help = after_help_text(),
)]
pub struct Args {
    /// Local path, file, or remote URL (https://, git@, ssh://)
    pub path: Option<String>,

    /// Preview output in the terminal instead of generating a PDF
    #[arg(long)]
    pub preview: bool,

    /// Output PDF file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    // ── Repository Mode ────────────────────────────────────────────────────────
    /// Glob patterns for files to include (repeatable)
    #[arg(long, action = clap::ArgAction::Append, help_heading = "Repository Mode (Default)")]
    pub include: Vec<String>,

    /// Glob patterns for files to exclude (repeatable)
    #[arg(long, action = clap::ArgAction::Append, help_heading = "Repository Mode (Default)")]
    pub exclude: Vec<String>,

    /// Syntax highlighting theme
    #[arg(
        long,
        default_value = "InspiredGitHub",
        help_heading = "Repository Mode (Default)"
    )]
    pub theme: String,

    /// Code font size in points
    #[arg(
        long,
        default_value_t = 8.0,
        help_heading = "Repository Mode (Default)"
    )]
    pub font_size: f64,

    /// Disable line numbers
    #[arg(long, help_heading = "Repository Mode (Default)")]
    pub no_line_numbers: bool,

    /// Disable table of contents
    #[arg(long, help_heading = "Repository Mode (Default)")]
    pub no_toc: bool,

    /// Disable directory tree visualization
    #[arg(long, help_heading = "Repository Mode (Default)")]
    pub no_file_tree: bool,

    /// Use a specific branch
    #[arg(long, help_heading = "Repository Mode (Default)")]
    pub branch: Option<String>,

    /// Use a specific commit
    #[arg(long, help_heading = "Repository Mode (Default)")]
    pub commit: Option<String>,

    /// Paper size
    #[arg(long, value_enum, default_value_t = PaperSize::A4, help_heading = "Repository Mode (Default)")]
    pub paper_size: PaperSize,

    /// Use landscape orientation
    #[arg(long, help_heading = "Repository Mode (Default)")]
    pub landscape: bool,

    /// List available syntax themes and exit
    #[arg(long, help_heading = "Repository Mode (Default)")]
    pub list_themes: bool,

    /// List version tags of the repository and exit
    #[arg(long, help_heading = "Repository Mode (Default)")]
    pub list_tags: bool,

    /// Open the repository in Neovim instead of generating a PDF
    #[arg(long, help_heading = "Repository Mode (Default)")]
    pub nvim: bool,

    // ── User Report Mode ───────────────────────────────────────────────────────
    /// GitHub username — generate a user activity report instead of printing a repo
    #[arg(short = 'u', long = "user", help_heading = "User Report Mode")]
    pub user: Option<String>,

    /// Number of most-recently-pushed repos to include [default: 5]
    #[arg(long, default_value_t = 5, help_heading = "User Report Mode")]
    pub last_repos: usize,

    /// Number of recent commits with diffs to render [default: 5]
    #[arg(long, default_value_t = 5, help_heading = "User Report Mode")]
    pub last_commits: usize,

    /// Skip commit diff rendering (faster)
    #[arg(long, help_heading = "User Report Mode")]
    pub no_diffs: bool,

    /// Show events from this date forward [default: no lower bound; GitHub keeps ≤ 90 days]
    ///
    /// Accepted formats:
    ///   ISO date    2024-01-15  or  2024-01-15T00:00:00Z
    ///   Keywords    today · yesterday
    ///   Named       last week · last month · last year
    ///   Relative    30 days ago · 2 weeks ago · 1 month ago · 1 year ago
    #[arg(long, value_name = "DATE", help_heading = "User Report Mode")]
    pub since: Option<String>,

    /// Show events up to and including this date [default: no upper bound]
    ///
    /// Same formats as --since.
    #[arg(long, value_name = "DATE", help_heading = "User Report Mode")]
    pub until: Option<String>,

    /// Event types to include in the activity feed [default: all]
    ///
    /// all     — every public event (pushes, PRs, issues, stars, forks, …)
    /// commits — push events only
    #[arg(long, value_enum, default_value_t = ActivityFilter::All, help_heading = "User Report Mode")]
    pub activity: ActivityFilter,

    /// Maximum events shown in the activity feed [default: 30]
    ///
    /// Fetches up to 100 events from GitHub and applies --since/--until/--activity
    /// filters before counting toward this limit.
    #[arg(long, default_value_t = 30, help_heading = "User Report Mode")]
    pub events: usize,
}

fn after_help_text() -> &'static str {
    static TEXT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TEXT.get_or_init(|| {
        let version = match option_env!("GITPRINT_GIT_BRANCH") {
            Some(branch) => branch.to_string(),
            None => format!("v{}", env!("CARGO_PKG_VERSION")),
        };
        let size_str = std::env::current_exe()
            .ok()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| {
                let bytes = m.len();
                let (size, unit) = if bytes >= 1_048_576 {
                    (bytes as f64 / 1_048_576.0, "MB")
                } else {
                    (bytes as f64 / 1_024.0, "KB")
                };
                format!("{size:.1} {unit}")
            })
            .unwrap_or_else(|| "unknown".to_string());
        format!("Version: {version} | Binary size: {size_str}\nSponsor: https://github.com/sponsors/izelnakri")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn requires_path_or_user() {
        assert!(Args::try_parse_from(["gitprint"]).is_err());
    }

    #[test]
    fn accepts_path() {
        let args = Args::parse_from(["gitprint", "."]);
        assert_eq!(args.path, Some(".".to_string()));
    }

    #[test]
    fn custom_path() {
        let args = Args::parse_from(["gitprint", "/tmp/repo"]);
        assert_eq!(args.path, Some("/tmp/repo".to_string()));
    }

    #[test]
    fn accepts_https_url() {
        let args = Args::parse_from(["gitprint", "https://github.com/user/repo"]);
        assert_eq!(args.path, Some("https://github.com/user/repo".to_string()));
    }

    #[test]
    fn accepts_ssh_url() {
        let args = Args::parse_from(["gitprint", "git@github.com:user/repo.git"]);
        assert_eq!(args.path, Some("git@github.com:user/repo.git".to_string()));
    }

    #[test]
    fn user_flag_short() {
        let args = Args::parse_from(["gitprint", "-u", "izelnakri"]);
        assert_eq!(args.user, Some("izelnakri".to_string()));
        assert_eq!(args.path, None);
    }

    #[test]
    fn user_flag_long() {
        let args = Args::parse_from(["gitprint", "--user", "torvalds"]);
        assert_eq!(args.user, Some("torvalds".to_string()));
    }

    #[test]
    fn user_flag_with_output() {
        let args = Args::parse_from(["gitprint", "-u", "alice", "-o", "alice.pdf"]);
        assert_eq!(args.user, Some("alice".to_string()));
        assert_eq!(args.output, Some(PathBuf::from("alice.pdf")));
    }

    #[test]
    fn user_report_flags_defaults() {
        let args = Args::parse_from(["gitprint", "-u", "alice"]);
        assert_eq!(args.last_repos, 5);
        assert_eq!(args.last_commits, 5);
        assert!(!args.no_diffs);
        assert_eq!(args.events, 30);
        assert!(matches!(args.activity, ActivityFilter::All));
        assert!(args.since.is_none());
        assert!(args.until.is_none());
    }

    #[test]
    fn since_until_flags() {
        let args = Args::parse_from(["gitprint", "-u", "alice", "--since", "2024-01-01"]);
        assert_eq!(args.since.as_deref(), Some("2024-01-01"));
        let args = Args::parse_from([
            "gitprint",
            "-u",
            "alice",
            "--since",
            "30 days ago",
            "--until",
            "yesterday",
        ]);
        assert_eq!(args.since.as_deref(), Some("30 days ago"));
        assert_eq!(args.until.as_deref(), Some("yesterday"));
    }

    #[test]
    fn activity_flag() {
        let args = Args::parse_from(["gitprint", "-u", "alice", "--activity", "commits"]);
        assert!(matches!(args.activity, ActivityFilter::Commits));
        let args = Args::parse_from(["gitprint", "-u", "alice", "--activity", "all"]);
        assert!(matches!(args.activity, ActivityFilter::All));
    }

    #[test]
    fn events_flag() {
        let args = Args::parse_from(["gitprint", "-u", "alice", "--events", "50"]);
        assert_eq!(args.events, 50);
    }

    #[test]
    fn user_report_flags_custom() {
        let args = Args::parse_from([
            "gitprint",
            "-u",
            "alice",
            "--last-commits",
            "3",
            "--no-diffs",
        ]);
        assert_eq!(args.last_commits, 3);
        assert!(args.no_diffs);
    }

    #[test]
    fn output_short_flag() {
        let args = Args::parse_from(["gitprint", ".", "-o", "out.pdf"]);
        assert_eq!(args.output, Some(PathBuf::from("out.pdf")));
    }

    #[test]
    fn output_long_flag() {
        let args = Args::parse_from(["gitprint", ".", "--output", "out.pdf"]);
        assert_eq!(args.output, Some(PathBuf::from("out.pdf")));
    }

    #[test]
    fn all_flags() {
        let args = Args::parse_from([
            "gitprint",
            "https://github.com/user/repo",
            "-o",
            "out.pdf",
            "--theme",
            "Solarized (dark)",
            "--font-size",
            "10",
            "--no-line-numbers",
            "--no-toc",
            "--no-file-tree",
            "--branch",
            "dev",
            "--paper-size",
            "letter",
            "--landscape",
            "--list-themes",
        ]);
        assert_eq!(args.path, Some("https://github.com/user/repo".to_string()));
        assert_eq!(args.output, Some(PathBuf::from("out.pdf")));
        assert_eq!(args.theme, "Solarized (dark)");
        assert_eq!(args.font_size, 10.0);
        assert!(args.no_line_numbers);
        assert!(args.no_toc);
        assert!(args.no_file_tree);
        assert_eq!(args.branch, Some("dev".to_string()));
        assert!(matches!(args.paper_size, PaperSize::Letter));
        assert!(args.landscape);
        assert!(args.list_themes);
    }

    #[test]
    fn list_tags_flag() {
        let args = Args::parse_from(["gitprint", ".", "--list-tags"]);
        assert!(args.list_tags);
        let args = Args::parse_from(["gitprint", "."]);
        assert!(!args.list_tags);
    }

    #[test]
    fn commit_flag() {
        let args = Args::parse_from(["gitprint", ".", "--commit", "abc1234"]);
        assert_eq!(args.commit, Some("abc1234".to_string()));
    }

    #[test]
    fn paper_size_legal() {
        let args = Args::parse_from(["gitprint", ".", "--paper-size", "legal"]);
        assert!(matches!(args.paper_size, PaperSize::Legal));
    }

    #[test]
    fn multiple_include_exclude() {
        let args = Args::parse_from([
            "gitprint",
            ".",
            "--include",
            "*.rs",
            "--include",
            "*.toml",
            "--exclude",
            "*.lock",
            "--exclude",
            "*.md",
        ]);
        assert_eq!(args.include, vec!["*.rs", "*.toml"]);
        assert_eq!(args.exclude, vec!["*.lock", "*.md"]);
    }

    #[test]
    fn font_size_custom() {
        let args = Args::parse_from(["gitprint", ".", "--font-size", "12.5"]);
        assert_eq!(args.font_size, 12.5);
    }

    #[test]
    fn preview_flag() {
        let args = Args::parse_from(["gitprint", ".", "--preview"]);
        assert!(args.preview);
        let args = Args::parse_from(["gitprint", "."]);
        assert!(!args.preview);
    }

    #[test]
    fn preview_with_user() {
        let args = Args::parse_from(["gitprint", "-u", "alice", "--preview"]);
        assert!(args.preview);
        assert_eq!(args.user, Some("alice".to_string()));
    }
}

use std::path::PathBuf;

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
    pub file_count: usize,
    pub total_lines: usize,
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
            file_count: 10,
            total_lines: 500,
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

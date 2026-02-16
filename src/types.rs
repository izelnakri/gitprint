use std::path::PathBuf;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum PaperSize {
    A4,
    Letter,
    Legal,
}

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

#[derive(Debug, Clone, Copy)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone)]
pub struct HighlightedToken {
    pub text: String,
    pub color: RgbColor,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone)]
pub struct HighlightedLine {
    pub line_number: usize,
    pub tokens: Vec<HighlightedToken>,
}


use clap::Parser;
use std::path::PathBuf;

use crate::types::PaperSize;

#[derive(Parser, Debug)]
#[command(
    name = "gitprint",
    about = "Convert git repositories into beautifully formatted PDFs",
    version
)]
pub struct Args {
    /// Path to git repository
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output PDF file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Glob patterns for files to include (repeatable)
    #[arg(long, action = clap::ArgAction::Append)]
    pub include: Vec<String>,

    /// Glob patterns for files to exclude (repeatable)
    #[arg(long, action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,

    /// Syntax highlighting theme
    #[arg(long, default_value = "InspiredGitHub")]
    pub theme: String,

    /// Code font size in points
    #[arg(long, default_value_t = 8.0)]
    pub font_size: f64,

    /// Disable line numbers
    #[arg(long)]
    pub no_line_numbers: bool,

    /// Disable table of contents
    #[arg(long)]
    pub no_toc: bool,

    /// Disable directory tree visualization
    #[arg(long)]
    pub no_file_tree: bool,

    /// Use a specific branch
    #[arg(long)]
    pub branch: Option<String>,

    /// Use a specific commit
    #[arg(long)]
    pub commit: Option<String>,

    /// Paper size
    #[arg(long, value_enum, default_value_t = PaperSize::A4)]
    pub paper_size: PaperSize,

    /// Use landscape orientation
    #[arg(long)]
    pub landscape: bool,

    /// List available syntax themes and exit
    #[arg(long)]
    pub list_themes: bool,
}

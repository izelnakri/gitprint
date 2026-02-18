//! # gitprint
//!
//! Convert git repositories into syntax-highlighted, printer-friendly PDFs.
//!
//! The main entry point is [`run()`], which executes the full pipeline:
//! git repository inspection, file filtering, syntax highlighting, and PDF generation.

pub mod cli;
pub mod defaults;
pub mod error;
pub mod filter;
pub mod git;
pub mod highlight;
pub mod pdf;
pub mod types;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::Error;
use crate::types::{Config, HighlightedLine};

/// A processed file ready for PDF rendering.
struct ProcessedFile {
    path: PathBuf,
    lines: Vec<HighlightedLine>,
    line_count: usize,
    file_size: u64,
    last_modified: String,
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Run the full pipeline: git → filter → highlight → PDF.
///
/// Files are read and highlighted concurrently via tokio tasks,
/// then rendered into the PDF sequentially.
pub async fn run(config: &Config) -> Result<(), Error> {
    let repo_path = git::verify_repo(&config.repo_path).await?;
    let mut metadata = git::get_metadata(&repo_path, config).await?;

    let all_paths = git::list_tracked_files(&repo_path, config).await?;
    let file_filter = filter::FileFilter::new(&config.include_patterns, &config.exclude_patterns)?;
    let mut paths: Vec<_> = file_filter.filter_paths(all_paths).collect();
    paths.sort();

    let highlighter = Arc::new(highlight::Highlighter::new(&config.theme)?);
    let date_map = Arc::new(git::file_last_modified_dates(&repo_path, config).await?);

    // Parallel: read + filter + highlight all files concurrently
    let mut set = tokio::task::JoinSet::new();
    paths.into_iter().for_each(|path| {
        let repo = repo_path.clone();
        let cfg = config.clone();
        let hl = Arc::clone(&highlighter);
        let dates = Arc::clone(&date_map);

        set.spawn(async move {
            let content = read_text_file(&repo, &path, &cfg).await?;
            let file_size = content.len() as u64;
            let line_count = content.lines().count();
            let last_modified = dates.get(&path).cloned().unwrap_or_default();
            let lines: Vec<HighlightedLine> = hl.highlight_lines(&content, &path).collect();
            Some(ProcessedFile {
                path,
                lines,
                line_count,
                file_size,
                last_modified,
            })
        });
    });

    let mut files: Vec<ProcessedFile> = set.join_all().await.into_iter().flatten().collect();
    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));

    metadata.file_count = files.len();
    metadata.total_lines = files.iter().map(|f| f.line_count).sum();

    // Build PDF: create document, add fonts, then render pages sequentially
    let mut doc = printpdf::PdfDocument::new("gitprint");
    let fonts = pdf::fonts::load_fonts(&mut doc)?;
    let mut builder = pdf::create_builder(config, fonts);

    pdf::cover::render(&mut builder, &metadata);

    if config.toc {
        let toc_entries: Vec<(&Path, usize)> = files
            .iter()
            .map(|f| (f.path.as_path(), f.line_count))
            .collect();
        pdf::toc::render(&mut builder, &toc_entries);
    }

    if config.file_tree {
        let paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
        pdf::tree::render(&mut builder, &paths);
    }

    files.into_iter().for_each(|file| {
        let size_str = format_size(file.file_size);
        let info = format!(
            "{} lines \u{00B7} {} \u{00B7} {}",
            file.line_count, size_str, file.last_modified
        );
        pdf::code::render_file(
            &mut builder,
            &file.path.display().to_string(),
            file.lines.into_iter(),
            file.line_count,
            !config.no_line_numbers,
            config.font_size as u8,
            &info,
        );
    });

    doc.with_pages(builder.finish());
    pdf::save_pdf(&doc, &config.output_path)?;

    eprintln!(
        "wrote {} files ({} lines) to {}",
        metadata.file_count,
        metadata.total_lines,
        config.output_path.display()
    );

    Ok(())
}

async fn read_text_file(repo_path: &Path, path: &Path, config: &Config) -> Option<String> {
    git::read_file_content(repo_path, path, config)
        .await
        .ok()
        .filter(|c| !filter::is_binary(c.as_bytes()))
        .filter(|c| !filter::is_minified(c))
}

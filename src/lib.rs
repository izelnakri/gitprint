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

use std::path::Path;

use crate::error::Error;
use crate::types::Config;

/// Run the full pipeline: git → filter → highlight → PDF.
/// Files are processed one at a time to keep memory usage low.
pub fn run(config: &Config) -> Result<(), Error> {
    let repo_path = git::verify_repo(&config.repo_path)?;
    let mut metadata = git::get_metadata(&repo_path, config)?;

    let all_paths = git::list_tracked_files(&repo_path, config)?;
    let file_filter = filter::FileFilter::new(&config.include_patterns, &config.exclude_patterns)?;
    let mut paths: Vec<_> = file_filter.filter_paths(all_paths).collect();
    paths.sort();

    let highlighter = highlight::Highlighter::new(&config.theme)?;
    let mut doc = pdf::create_document(config)?;

    // First pass: collect line counts per file for TOC and cover page.
    // Reads each file once but only keeps the line count, not the content.
    let line_counts: Vec<usize> = paths
        .iter()
        .map(|p| count_lines(&repo_path, p, config))
        .collect();

    let toc_entries: Vec<(&Path, usize)> = paths
        .iter()
        .zip(&line_counts)
        .map(|(p, &n)| (p.as_path(), n))
        .collect();

    metadata.file_count = toc_entries.len();
    metadata.total_lines = line_counts.iter().sum();

    // Render front matter
    pdf::cover::render(&mut doc, &metadata);

    if config.toc {
        pdf::toc::render(&mut doc, &toc_entries);
    }
    if config.file_tree {
        pdf::tree::render(&mut doc, &paths);
    }

    // Second pass: read, highlight, and render each file one at a time.
    // Only one file's content + highlighted tokens live in memory at once.
    paths.iter().zip(&line_counts).for_each(|(path, &lines)| {
        if let Some(content) = read_text_file(&repo_path, path, config) {
            let highlighted = highlighter.highlight_lines(&content, path);
            let display_path = path.display().to_string();
            pdf::code::render_file(
                &mut doc,
                &display_path,
                highlighted,
                lines,
                !config.no_line_numbers,
                config.font_size as u8,
            );
            // `content` and highlighted iterator are dropped here
        }
    });

    pdf::write_pdf(doc, &config.output_path)?;

    eprintln!(
        "wrote {} files ({} lines) to {}",
        metadata.file_count,
        metadata.total_lines,
        config.output_path.display()
    );

    Ok(())
}

/// Read a file and return its content only if it's valid text (not binary/minified).
fn read_text_file(repo_path: &Path, path: &Path, config: &Config) -> Option<String> {
    git::read_file_content(repo_path, path, config)
        .ok()
        .filter(|c| !filter::is_binary(c.as_bytes()))
        .filter(|c| !filter::is_minified(c))
}

/// Count lines in a file without keeping its content in memory.
fn count_lines(repo_path: &Path, path: &Path, config: &Config) -> usize {
    read_text_file(repo_path, path, config)
        .map(|c| c.lines().count())
        .unwrap_or(0)
}

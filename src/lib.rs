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

fn format_elapsed(elapsed: std::time::Duration) -> String {
    if elapsed.as_millis() < 1000 {
        format!("{}ms", elapsed.as_millis())
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}

/// Run the full pipeline: git → filter → highlight → PDF.
///
/// Files are read and highlighted concurrently via tokio tasks,
/// then rendered into the PDF sequentially.
pub async fn run(config: &Config) -> Result<(), Error> {
    let start = std::time::Instant::now();

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

    // Build PDF document and load fonts once
    let mut doc = printpdf::PdfDocument::new("gitprint");
    let fonts = pdf::fonts::load_fonts(&mut doc)?;

    // -- Cover page (always page 1) --
    let cover_pages = {
        let mut b = pdf::create_builder(config, fonts.clone());
        pdf::cover::render(&mut b, &metadata);
        b.finish()
    };
    let cover_count = cover_pages.len();

    // Pre-collect tree paths (needed for both dummy + actual tree renders)
    let tree_paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();

    // Build dummy TocEntry list (start_page=0 placeholders) to count TOC pages
    let dummy_toc_entries: Vec<pdf::toc::TocEntry> = files
        .iter()
        .map(|f| pdf::toc::TocEntry {
            path: f.path.clone(),
            line_count: f.line_count,
            size_str: format_size(f.file_size),
            last_modified: f.last_modified.clone(),
            start_page: 0,
        })
        .collect();

    // -- Count TOC pages (dummy render, page numbers don't affect line count) --
    let toc_count = if config.toc {
        let mut b = pdf::create_builder(config, fonts.clone());
        pdf::toc::render(&mut b, &dummy_toc_entries);
        b.finish().len()
    } else {
        0
    };

    // -- Count tree pages (dummy render) --
    let tree_count = if config.file_tree {
        let mut b = pdf::create_builder(config, fonts.clone());
        pdf::tree::render(&mut b, &tree_paths);
        b.finish().len()
    } else {
        0
    };

    // -- Render file content, tracking each file's starting page --
    let file_base_page = cover_count + toc_count + tree_count + 1;
    let mut content_builder = pdf::create_builder_at_page(config, fonts.clone(), file_base_page);
    let mut toc_entries: Vec<pdf::toc::TocEntry> = Vec::with_capacity(files.len());

    files.into_iter().for_each(|file| {
        let start_page = content_builder.current_page();
        let size_str = format_size(file.file_size);
        let info = format!(
            "{} LOC \u{00B7} {} \u{00B7} {}",
            file.line_count, size_str, file.last_modified
        );
        toc_entries.push(pdf::toc::TocEntry {
            path: file.path.clone(),
            line_count: file.line_count,
            size_str,
            last_modified: file.last_modified.clone(),
            start_page,
        });
        pdf::code::render_file(
            &mut content_builder,
            &file.path.display().to_string(),
            file.lines.into_iter(),
            file.line_count,
            !config.no_line_numbers,
            config.font_size as u8,
            &info,
        );
    });
    let content_pages = content_builder.finish();

    // -- Render actual TOC with real page numbers --
    let toc_pages = if config.toc {
        let mut b = pdf::create_builder_at_page(config, fonts.clone(), cover_count + 1);
        pdf::toc::render(&mut b, &toc_entries);
        b.finish()
    } else {
        vec![]
    };

    // -- Render actual tree with correct page offset --
    let tree_pages = if config.file_tree {
        let mut b =
            pdf::create_builder_at_page(config, fonts.clone(), cover_count + toc_count + 1);
        pdf::tree::render(&mut b, &tree_paths);
        b.finish()
    } else {
        vec![]
    };

    // -- Assemble final document in order: cover → TOC → tree → files --
    let all_pages: Vec<_> = cover_pages
        .into_iter()
        .chain(toc_pages)
        .chain(tree_pages)
        .chain(content_pages)
        .collect();
    let total_pages = all_pages.len();

    doc.with_pages(all_pages);
    pdf::save_pdf(&doc, &config.output_path)?;

    let elapsed = start.elapsed();
    let pdf_size = std::fs::metadata(&config.output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    eprintln!(
        "{} — {} files, {} pages, {}, {}",
        config.output_path.display(),
        metadata.file_count,
        total_pages,
        format_size(pdf_size),
        format_elapsed(elapsed),
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

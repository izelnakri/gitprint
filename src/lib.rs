//! # gitprint
//!
//! Convert git repositories into syntax-highlighted, printer-friendly PDFs.
//!
//! The main entry point is [`run()`], which executes the full pipeline:
//! git repository inspection, file filtering, syntax highlighting, and PDF generation.

pub mod cli;
pub mod defaults;
pub mod filter;
pub mod git;
pub mod highlight;
pub mod pdf;
pub mod types;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::bail;

use crate::types::{Config, HighlightedLine};

/// A processed file ready for PDF rendering.
struct ProcessedFile {
    path: PathBuf,
    lines: Vec<HighlightedLine>,
    line_count: usize,
    /// Pre-formatted size string, computed once to avoid calling format_size twice.
    size_str: String,
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

/// Formats the current UTC time as `YYYY-MM-DD HH:MM:SS UTC`.
///
/// Uses Howard Hinnant's Euclidean Gregorian algorithm — no external crate needed.
fn format_utc_now() -> String {
    let total_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let (h, m, s) = (
        (total_secs / 3600) % 24,
        (total_secs / 60) % 60,
        total_secs % 60,
    );

    let z = (total_secs / 86400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };

    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02} UTC")
}

fn format_elapsed(elapsed: std::time::Duration) -> String {
    if elapsed.as_millis() < 1000 {
        format!("{}ms", elapsed.as_millis())
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}

/// Runs the full gitprint pipeline and writes a PDF to `config.output_path`.
///
/// Accepts a single file, a git repository (optionally scoped to a subdirectory),
/// or a plain directory. The output always goes to `config.output_path`.
///
/// # Errors
///
/// Returns an error if the path does not exist, git operations fail, the theme is
/// invalid, or writing the PDF fails.
///
/// # Examples
///
/// ```ignore
/// use gitprint::types::{Config, PaperSize};
/// use std::path::PathBuf;
///
/// let config = Config {
///     repo_path: PathBuf::from("."),
///     output_path: PathBuf::from("out.pdf"),
///     // ... other fields
/// #   include_patterns: vec![],
/// #   exclude_patterns: vec![],
/// #   theme: "InspiredGitHub".to_string(),
/// #   font_size: 8.0,
/// #   no_line_numbers: false,
/// #   toc: true,
/// #   file_tree: true,
/// #   branch: None,
/// #   commit: None,
/// #   paper_size: PaperSize::A4,
/// #   landscape: false,
/// };
/// gitprint::run(&config).await.unwrap();
/// ```
///
/// **Concurrency model**:
/// - Single-file mode: highlighter init (CPU, `spawn_blocking`) runs concurrently with
///   file content read and last-modified date fetch (both I/O).
/// - Multi-file mode: git metadata, tracked-file list, date map, and highlighter init
///   all run concurrently via `tokio::join!`; highlighter uses `spawn_blocking` to keep
///   tokio worker threads free for I/O.
/// - File reads use a tokio `JoinSet` (I/O-bound parallelism).
/// - Syntax highlighting uses a tokio `JoinSet` of `spawn_blocking` tasks — one per file
///   — so all files are highlighted concurrently across the blocking thread pool (CPU-bound).
/// - Cover, TOC, and tree PDF renders are sequential (each < 5 ms; not worth the overhead).
pub async fn run(config: &Config) -> anyhow::Result<()> {
    let start = std::time::Instant::now();

    let info = git::verify_repo(&config.repo_path).await?;

    // Single-file mode: no cover page, TOC, or file tree — just render the file.
    if let Some(ref single_file) = info.single_file {
        // Highlighter init (CPU, spawn_blocking) overlaps with two I/O calls.
        let theme = config.theme.clone();
        let (highlighter_res, content_res, last_modified) = tokio::join!(
            tokio::task::spawn_blocking(move || highlight::Highlighter::new(&theme)),
            git::read_file_content(&info.root, single_file, config),
            git::file_last_modified(&info.root, single_file, config, info.is_git),
        );
        let highlighter =
            highlighter_res.map_err(|e| anyhow::anyhow!("highlighter panicked: {e}"))??;
        let content = content_res?;

        if filter::is_binary(content.as_bytes()) || filter::is_minified(&content) {
            bail!("{}: binary or minified file", single_file.display());
        }
        let line_count = content.lines().count();
        let size_str = format_size(content.len() as u64);
        let lines: Vec<HighlightedLine> =
            highlighter.highlight_lines(&content, single_file).collect();

        let doc_title = config
            .remote_url
            .as_deref()
            .map(git::repo_name_from_url)
            .unwrap_or_else(|| {
                config
                    .repo_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "gitprint".to_string())
            });
        let mut doc = printpdf::PdfDocument::new(&doc_title);
        let fonts = pdf::fonts::load_fonts(&mut doc)?;
        let mut builder = pdf::create_builder(config, fonts);
        let file_info = format!("{line_count} LOC \u{00B7} {size_str} \u{00B7} {last_modified}");
        let header_url = config.remote_url.as_ref().map(|url| {
            let base = url.trim_end_matches(".git");
            format!("{base}/blob/HEAD/{}", single_file.display())
        });
        pdf::code::render_file(
            &mut builder,
            &single_file.display().to_string(),
            lines.into_iter(),
            line_count,
            !config.no_line_numbers,
            config.font_size as u8,
            &file_info,
            header_url.as_deref(),
        );
        let pages = builder.finish();
        let total_pages = pages.len();
        doc.with_pages(pages);
        pdf::save_pdf(&doc, &config.output_path).await?;

        let elapsed = start.elapsed();
        let pdf_size = tokio::fs::metadata(&config.output_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        eprintln!(
            "{} — 1 file, {} pages, {}, {}",
            config.output_path.display(),
            total_pages,
            format_size(pdf_size),
            format_elapsed(elapsed),
        );
        return Ok(());
    }

    let repo_path = info.root;
    let is_git = info.is_git;
    let scope = info.scope;

    // Parallel: git metadata + tracked file list + date map + highlighter init
    // + fs owner/group + repo disk size (for local paths).
    // Highlighter::new is CPU-bound (syntect deserialization); spawn_blocking keeps
    // tokio worker threads free for the concurrent I/O-bound git calls.
    let theme = config.theme.clone();
    let fs_path = config.repo_path.clone();
    let fs_path2 = config.repo_path.clone();
    let is_remote = config.remote_url.is_some();
    let generated_at = format_utc_now();
    let (metadata_res, all_paths_res, date_map_res, highlighter_res, fs_owner_group, size) = tokio::join!(
        git::get_metadata(&repo_path, config, is_git, scope.as_deref()),
        git::list_tracked_files(&repo_path, config, is_git, scope.as_deref()),
        git::file_last_modified_dates(&repo_path, config, is_git, scope.as_deref()),
        tokio::task::spawn_blocking(move || highlight::Highlighter::new(&theme)),
        async move {
            if is_remote {
                (None, None)
            } else {
                git::fs_owner_group(&fs_path).await
            }
        },
        git::repo_size(&fs_path2),
    );

    let mut metadata = metadata_res?;
    if let Some(ref url) = config.remote_url {
        metadata.name = git::repo_name_from_url(url);
    }
    metadata.fs_owner = fs_owner_group.0;
    metadata.fs_group = fs_owner_group.1;
    metadata.generated_at = generated_at;
    metadata.repo_size = size;
    if !is_remote {
        metadata.repo_absolute_path = Some(repo_path.clone());
    }
    let highlighter =
        Arc::new(highlighter_res.map_err(|e| anyhow::anyhow!("highlighter panicked: {e}"))??);
    let date_map = Arc::new(date_map_res?);

    let file_filter = filter::FileFilter::new(&config.include_patterns, &config.exclude_patterns)?;
    let mut paths: Vec<_> = file_filter.filter_paths(all_paths_res?).collect();
    paths.sort_unstable();

    // Phase 1 — I/O: read all file contents concurrently with tokio.
    let mut read_set: tokio::task::JoinSet<Option<(PathBuf, String, String)>> =
        tokio::task::JoinSet::new();
    paths.into_iter().for_each(|path| {
        let repo = repo_path.clone();
        let cfg = config.clone();
        let dates = Arc::clone(&date_map);
        read_set.spawn(async move {
            let content = read_text_file(&repo, &path, &cfg).await?;
            let last_modified = dates.get(&path).cloned().unwrap_or_default();
            Some((path, content, last_modified))
        });
    });
    let raw_files: Vec<(PathBuf, String, String)> =
        read_set.join_all().await.into_iter().flatten().collect();

    // Phase 2 — CPU: highlight each file in a dedicated blocking task so all files
    // are processed concurrently across tokio's blocking thread pool.
    let mut highlight_set: tokio::task::JoinSet<ProcessedFile> = tokio::task::JoinSet::new();
    raw_files
        .into_iter()
        .for_each(|(path, content, last_modified)| {
            let hl = Arc::clone(&highlighter);
            highlight_set.spawn_blocking(move || {
                let line_count = content.lines().count();
                let size_str = format_size(content.len() as u64);
                let lines: Vec<HighlightedLine> = hl.highlight_lines(&content, &path).collect();
                ProcessedFile {
                    path,
                    lines,
                    line_count,
                    size_str,
                    last_modified,
                }
            });
        });
    let mut files: Vec<ProcessedFile> = highlight_set.join_all().await;

    files.sort_unstable_by(|a, b| a.path.cmp(&b.path));

    metadata.file_count = files.len();
    metadata.total_lines = files.iter().map(|f| f.line_count).sum();

    // Build PDF document and load fonts once.
    let mut doc = printpdf::PdfDocument::new(&metadata.name);
    let fonts = pdf::fonts::load_fonts(&mut doc)?;

    // Collect paths and build dummy TOC entries before the parallel render phase.
    let tree_paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();

    // Dummy TOC entries (start_page=0) used purely to count how many pages the TOC occupies.
    // Each entry is one line regardless of content, so page count is stable.
    let dummy_toc_entries: Vec<pdf::toc::TocEntry> = files
        .iter()
        .map(|f| pdf::toc::TocEntry {
            path: f.path.clone(),
            line_count: f.line_count,
            size_str: f.size_str.clone(),
            last_modified: f.last_modified.clone(),
            start_page: 0,
        })
        .collect();

    // For cover links: use explicit remote_url from CLI, or fall back to remote detected
    // from git config so links work even when printing a local repo without --remote.
    let effective_remote_url = config
        .remote_url
        .as_deref()
        .or(metadata.detected_remote_url.as_deref());

    let cover_pages = {
        let mut b = pdf::create_builder(config, fonts.clone());
        pdf::cover::render(&mut b, &metadata, effective_remote_url);
        b.finish()
    };
    let toc_count = if config.toc {
        let mut b = pdf::create_builder(config, fonts.clone());
        pdf::toc::render(&mut b, &dummy_toc_entries);
        b.finish().len()
    } else {
        0
    };
    let tree_count = if config.file_tree {
        let mut b = pdf::create_builder(config, fonts.clone());
        pdf::tree::render(&mut b, &tree_paths);
        b.finish().len()
    } else {
        0
    };
    let cover_count = cover_pages.len();

    // Render file content sequentially, tracking each file's starting page.
    let file_base_page = cover_count + toc_count + tree_count + 1;
    let mut content_builder = pdf::create_builder_at_page(config, fonts.clone(), file_base_page);
    let mut toc_entries: Vec<pdf::toc::TocEntry> = Vec::with_capacity(files.len());

    let remote_base = config.remote_url.as_ref().map(|url| {
        let base = url.trim_end_matches(".git");
        let commit = if metadata.commit_hash.is_empty() {
            "HEAD"
        } else {
            &metadata.commit_hash
        };
        format!("{base}/blob/{commit}")
    });

    files.into_iter().for_each(|file| {
        let start_page = content_builder.current_page();
        let info = format!(
            "{} LOC \u{00B7} {} \u{00B7} {}",
            file.line_count, file.size_str, file.last_modified
        );
        toc_entries.push(pdf::toc::TocEntry {
            path: file.path.clone(),
            line_count: file.line_count,
            size_str: file.size_str,
            last_modified: file.last_modified.clone(),
            start_page,
        });
        let header_url = remote_base
            .as_ref()
            .map(|base| format!("{base}/{}", file.path.display()));
        pdf::code::render_file(
            &mut content_builder,
            &file.path.display().to_string(),
            file.lines.into_iter(),
            file.line_count,
            !config.no_line_numbers,
            config.font_size as u8,
            &info,
            header_url.as_deref(),
        );
    });
    let content_pages = content_builder.finish();

    let toc_pages = if config.toc {
        let mut b = pdf::create_builder_at_page(config, fonts.clone(), cover_count + 1);
        pdf::toc::render(&mut b, &toc_entries);
        b.finish()
    } else {
        vec![]
    };
    let tree_pages = if config.file_tree {
        let mut b = pdf::create_builder_at_page(config, fonts.clone(), cover_count + toc_count + 1);
        pdf::tree::render(&mut b, &tree_paths);
        b.finish()
    } else {
        vec![]
    };

    // Assemble final document: cover → TOC → tree → file content.
    let all_pages: Vec<_> = cover_pages
        .into_iter()
        .chain(toc_pages)
        .chain(tree_pages)
        .chain(content_pages)
        .collect();
    let total_pages = all_pages.len();

    doc.with_pages(all_pages);
    pdf::save_pdf(&doc, &config.output_path).await?;

    let elapsed = start.elapsed();
    let pdf_size = tokio::fs::metadata(&config.output_path)
        .await
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

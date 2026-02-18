use std::path::PathBuf;

use printpdf::{Actions, Color, Destination, Pt, Rgb};

use super::layout::{PageBuilder, Span};

/// A single entry in the Table of Contents.
pub struct TocEntry {
    pub path: PathBuf,
    pub line_count: usize,
    pub size_str: String,
    pub last_modified: String,
    pub start_page: usize,
}

/// Split `text` into chunks of at most `max_chars` characters each.
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 || text.is_empty() {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        let split_at = remaining
            .char_indices()
            .nth(max_chars)
            .map(|(i, _)| i)
            .unwrap_or(remaining.len());
        chunks.push(remaining[..split_at].to_string());
        remaining = &remaining[split_at..];
    }
    chunks
}

pub fn render(builder: &mut PageBuilder, entries: &[TocEntry]) {
    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let gray = Color::Rgb(Rgb::new(0.47, 0.47, 0.47, None));
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));

    builder.write_centered("Table of Contents", &bold, Pt(16.0), black);
    builder.vertical_space(10.0);

    // Approximate character width factors (monospace font approximation).
    const PATH_SIZE: f32 = 8.0;
    const META_SIZE: f32 = 7.0;
    const CHAR_WIDTH: f32 = 0.6;
    const GAP_PT: f32 = 8.0;

    entries.iter().for_each(|entry| {
        let meta = format!(
            "p.{}  {} LOC \u{00B7} {} \u{00B7} {}",
            entry.start_page, entry.line_count, entry.size_str, entry.last_modified
        );
        let meta_width = meta.len() as f32 * META_SIZE * CHAR_WIDTH;
        let available_left = builder.usable_width_pt() - meta_width - GAP_PT;
        let max_chars = (available_left / (PATH_SIZE * CHAR_WIDTH)).max(1.0) as usize;

        let path_str = entry.path.display().to_string();
        let chunks = wrap_text(&path_str, max_chars);
        let row_count = chunks.len();

        // First chunk shares the line with meta; remaining chunks are on their own lines.
        builder.write_line_justified(
            &[Span {
                text: chunks[0].clone(),
                font_id: regular.clone(),
                size: Pt(PATH_SIZE),
                color: gray.clone(),
            }],
            &[Span {
                text: meta,
                font_id: regular.clone(),
                size: Pt(META_SIZE),
                color: gray.clone(),
            }],
        );
        chunks[1..].iter().for_each(|chunk| {
            builder.write_line(&[Span {
                text: chunk.clone(),
                font_id: regular.clone(),
                size: Pt(PATH_SIZE),
                color: gray.clone(),
            }]);
        });

        builder.add_link(
            builder.line_height() * row_count as f32,
            Actions::Goto(Destination::Xyz {
                page: entry.start_page,
                left: None,
                top: None,
                zoom: None,
            }),
        );
    });

    builder.page_break();
}

#[cfg(test)]
mod tests {
    use crate::pdf;
    use crate::types::Config;
    use std::path::PathBuf;

    fn make_entry(path: &str, lines: usize, page: usize) -> super::TocEntry {
        super::TocEntry {
            path: PathBuf::from(path),
            line_count: lines,
            size_str: "1.2 KB".to_string(),
            last_modified: "2024-01-15".to_string(),
            start_page: page,
        }
    }

    #[test]
    fn wrap_text_short() {
        let chunks = super::wrap_text("short", 20);
        assert_eq!(chunks, vec!["short"]);
    }

    #[test]
    fn wrap_text_exact() {
        let chunks = super::wrap_text("1234567890", 10);
        assert_eq!(chunks, vec!["1234567890"]);
    }

    #[test]
    fn wrap_text_overflow() {
        let chunks = super::wrap_text("1234567890ab", 10);
        assert_eq!(chunks, vec!["1234567890", "ab"]);
    }

    #[test]
    fn wrap_text_empty() {
        let chunks = super::wrap_text("", 10);
        assert_eq!(chunks, vec![""]);
    }

    #[test]
    fn render_toc_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let entries = vec![
            make_entry("src/main.rs", 20, 5),
            make_entry("src/lib.rs", 50, 7),
        ];
        super::render(&mut builder, &entries);
    }

    #[test]
    fn render_toc_empty_files() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render(&mut builder, &[]);
    }

    #[test]
    fn render_toc_many_files() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let entries: Vec<_> = (0..100)
            .map(|i| make_entry("src/file.rs", i * 10, i + 5))
            .collect();
        super::render(&mut builder, &entries);
    }

    #[test]
    fn render_toc_long_path_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let entries = vec![make_entry(
            "src/very/deeply/nested/path/that/is/quite/long/file.rs",
            100,
            3,
        )];
        super::render(&mut builder, &entries);
    }
}

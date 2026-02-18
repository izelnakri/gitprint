use std::path::PathBuf;

use printpdf::{Color, Pt, Rgb};

use super::layout::{PageBuilder, Span};

/// A single entry in the Table of Contents.
pub struct TocEntry {
    pub path: PathBuf,
    pub line_count: usize,
    pub size_str: String,
    pub last_modified: String,
    pub start_page: usize,
}

pub fn render(builder: &mut PageBuilder, entries: &[TocEntry]) {
    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let gray = Color::Rgb(Rgb::new(0.47, 0.47, 0.47, None));
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));

    builder.write_centered("Table of Contents", &bold, Pt(16.0), black);
    builder.vertical_space(10.0);

    entries.iter().for_each(|entry| {
        let meta = format!(
            "p.{}  {} LOC \u{00B7} {} \u{00B7} {}",
            entry.start_page, entry.line_count, entry.size_str, entry.last_modified
        );
        builder.write_line_justified(
            &[Span {
                text: entry.path.display().to_string(),
                font_id: regular.clone(),
                size: Pt(8.0),
                color: gray.clone(),
            }],
            &[Span {
                text: meta,
                font_id: regular.clone(),
                size: Pt(7.0),
                color: gray.clone(),
            }],
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
        let entries: Vec<_> = (0..100).map(|i| make_entry("src/file.rs", i * 10, i + 5)).collect();
        super::render(&mut builder, &entries);
    }
}

use std::path::Path;

use genpdfi::elements::{self, Paragraph};
use genpdfi::{Alignment, Element, style};

fn make_entry(path: &Path, line_count: usize) -> Paragraph {
    Paragraph::default()
        .styled_string(
            path.display().to_string(),
            style::Style::new().with_font_size(8),
        )
        .styled_string(
            format!("  ({line_count} lines)"),
            style::Style::new()
                .with_font_size(7)
                .with_color(style::Color::Rgb(120, 120, 120)),
        )
}

/// Renders a table of contents. Each entry is a `(path, line_count)` pair.
pub fn render(doc: &mut genpdfi::Document, files: &[(&Path, usize)]) {
    doc.push(
        Paragraph::new("Table of Contents")
            .aligned(Alignment::Center)
            .styled(style::Style::new().bold().with_font_size(16)),
    );
    doc.push(elements::Break::new(1.0));

    files
        .iter()
        .map(|(path, lines)| make_entry(path, *lines))
        .for_each(|entry| doc.push(entry));

    doc.push(elements::PageBreak::new());
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::types::Config;

    #[test]
    fn render_toc_does_not_panic() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        let entries: Vec<(&Path, usize)> = vec![
            (Path::new("src/main.rs"), 20),
            (Path::new("src/lib.rs"), 50),
        ];
        super::render(&mut doc, &entries);
    }

    #[test]
    fn render_toc_empty_files() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        super::render(&mut doc, &[]);
    }

    #[test]
    fn render_toc_many_files() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        let entries: Vec<(&Path, usize)> = (0..100)
            .map(|i| (Path::new("src/file.rs"), i * 10))
            .collect();
        super::render(&mut doc, &entries);
    }

    #[test]
    fn render_toc_zero_line_count() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        let entries: Vec<(&Path, usize)> = vec![(Path::new("empty.rs"), 0)];
        super::render(&mut doc, &entries);
    }
}

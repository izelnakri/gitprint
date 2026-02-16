use std::path::Path;

use genpdfi::elements::{self, Paragraph};
use genpdfi::{style, Alignment, Element};

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

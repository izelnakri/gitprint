use genpdfi::elements::{self, Paragraph};
use genpdfi::{style, Element};

use crate::types::HighlightedLine;

/// Render a single highlighted line into a Paragraph, consuming it.
fn render_line(
    line: HighlightedLine,
    line_number_width: usize,
    show_line_numbers: bool,
    font_size: u8,
) -> Paragraph {
    let mut para = Paragraph::default();

    if show_line_numbers {
        let num_str = format!("{:>width$}  ", line.line_number, width = line_number_width);
        para.push_styled(
            num_str,
            style::Style::new()
                .with_font_size(font_size)
                .with_color(style::Color::Rgb(150, 150, 150)),
        );
    }

    for token in line.tokens {
        let mut s = style::Style::new()
            .with_font_size(font_size)
            .with_color(style::Color::Rgb(
                token.color.r,
                token.color.g,
                token.color.b,
            ));
        if token.bold {
            s = s.bold();
        }
        if token.italic {
            s = s.italic();
        }
        para.push_styled(token.text, s);
    }

    para
}

/// Push one file's highlighted content into the document, consuming the line
/// iterator so each line is dropped after it's rendered.
pub fn render_file(
    doc: &mut genpdfi::Document,
    file_path: &str,
    lines: impl Iterator<Item = HighlightedLine>,
    total_lines: usize,
    show_line_numbers: bool,
    font_size: u8,
) {
    // File header
    let header = Paragraph::new(file_path)
        .styled(style::Style::new().bold().with_font_size(font_size + 2));
    doc.push(elements::FramedElement::new(header));
    doc.push(elements::Break::new(0.5));

    let line_number_width = total_lines.max(1).ilog10() as usize + 1;

    for line in lines {
        doc.push(render_line(line, line_number_width, show_line_numbers, font_size));
    }

    doc.push(elements::PageBreak::new());
}

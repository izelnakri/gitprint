use genpdfi::elements::{self, Paragraph};
use genpdfi::{Element, style};

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
    let header =
        Paragraph::new(file_path).styled(style::Style::new().bold().with_font_size(font_size + 2));
    doc.push(elements::FramedElement::new(header));
    doc.push(elements::Break::new(0.5));

    let line_number_width = total_lines.max(1).ilog10() as usize + 1;

    for line in lines {
        doc.push(render_line(
            line,
            line_number_width,
            show_line_numbers,
            font_size,
        ));
    }

    doc.push(elements::PageBreak::new());
}

#[cfg(test)]
mod tests {
    use crate::types::{Config, HighlightedLine, HighlightedToken, RgbColor};

    fn sample_lines() -> Vec<HighlightedLine> {
        vec![
            HighlightedLine {
                line_number: 1,
                tokens: vec![HighlightedToken {
                    text: "fn main() {}".to_string(),
                    color: RgbColor { r: 0, g: 0, b: 0 },
                    bold: false,
                    italic: false,
                }],
            },
            HighlightedLine {
                line_number: 2,
                tokens: vec![HighlightedToken {
                    text: "// comment".to_string(),
                    color: RgbColor {
                        r: 100,
                        g: 100,
                        b: 100,
                    },
                    bold: false,
                    italic: true,
                }],
            },
        ]
    }

    fn bold_italic_line() -> HighlightedLine {
        HighlightedLine {
            line_number: 1,
            tokens: vec![
                HighlightedToken {
                    text: "bold".to_string(),
                    color: RgbColor { r: 255, g: 0, b: 0 },
                    bold: true,
                    italic: false,
                },
                HighlightedToken {
                    text: " italic".to_string(),
                    color: RgbColor { r: 0, g: 255, b: 0 },
                    bold: false,
                    italic: true,
                },
                HighlightedToken {
                    text: " bold-italic".to_string(),
                    color: RgbColor { r: 0, g: 0, b: 255 },
                    bold: true,
                    italic: true,
                },
            ],
        }
    }

    #[test]
    fn render_file_does_not_panic() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        super::render_file(&mut doc, "test.rs", sample_lines().into_iter(), 2, true, 8);
    }

    #[test]
    fn render_file_empty_iterator() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        super::render_file(&mut doc, "empty.rs", std::iter::empty(), 0, true, 8);
    }

    #[test]
    fn render_file_without_line_numbers() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        super::render_file(&mut doc, "test.rs", sample_lines().into_iter(), 2, false, 8);
    }

    #[test]
    fn render_file_large_font_size() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        super::render_file(&mut doc, "test.rs", sample_lines().into_iter(), 2, true, 16);
    }

    #[test]
    fn render_file_with_bold_italic_tokens() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        super::render_file(
            &mut doc,
            "styled.rs",
            vec![bold_italic_line()].into_iter(),
            1,
            true,
            8,
        );
    }

    #[test]
    fn render_file_many_lines() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        let lines: Vec<_> = (1..=100)
            .map(|i| HighlightedLine {
                line_number: i,
                tokens: vec![HighlightedToken {
                    text: format!("line {i}"),
                    color: RgbColor { r: 0, g: 0, b: 0 },
                    bold: false,
                    italic: false,
                }],
            })
            .collect();
        super::render_file(&mut doc, "big.rs", lines.into_iter(), 100, true, 8);
    }

    #[test]
    fn render_file_single_line() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        let lines = vec![HighlightedLine {
            line_number: 1,
            tokens: vec![HighlightedToken {
                text: "single".to_string(),
                color: RgbColor { r: 0, g: 0, b: 0 },
                bold: false,
                italic: false,
            }],
        }];
        super::render_file(&mut doc, "one.rs", lines.into_iter(), 1, true, 8);
    }
}

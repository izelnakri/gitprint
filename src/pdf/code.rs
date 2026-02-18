use printpdf::{Actions, Color, Pt, Rgb};

use super::layout::{PageBuilder, Span};
use crate::types::HighlightedLine;

#[allow(clippy::too_many_arguments)]
pub fn render_file(
    builder: &mut PageBuilder,
    file_path: &str,
    lines: impl Iterator<Item = HighlightedLine>,
    total_lines: usize,
    show_line_numbers: bool,
    font_size: u8,
    file_info: &str,
    // If `Some`, the file header becomes a clickable link to this URL (e.g. GitHub blob view).
    header_url: Option<&str>,
) {
    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
    let size = Pt(font_size as f32);
    let gray = Color::Rgb(Rgb::new(0.59, 0.59, 0.59, None));
    let line_number_width = total_lines.max(1).ilog10() as usize + 1;

    // File header: path left-aligned, metadata right-aligned
    builder.write_line_justified(
        &[Span {
            text: file_path.to_string(),
            font_id: bold,
            size: Pt(font_size as f32 + 2.0),
            color: black,
        }],
        &[Span {
            text: file_info.to_string(),
            font_id: regular,
            size: Pt(7.0),
            color: gray.clone(),
        }],
    );
    if let Some(url) = header_url {
        builder.add_link(builder.line_height(), Actions::Uri(url.to_string()));
    }
    builder.vertical_space(4.0);

    lines.for_each(|line| {
        let mut spans: Vec<Span> = Vec::with_capacity(line.tokens.len() + 1);

        if show_line_numbers {
            spans.push(Span {
                text: format!("{:>width$}  ", line.line_number, width = line_number_width),
                font_id: builder.font(false, false).clone(),
                size,
                color: gray.clone(),
            });
        }

        spans.extend(line.tokens.into_iter().map(|token| Span {
            text: token.text,
            font_id: builder.font(token.bold, token.italic).clone(),
            size,
            color: Color::Rgb(Rgb::new(
                token.color.r as f32 / 255.0,
                token.color.g as f32 / 255.0,
                token.color.b as f32 / 255.0,
                None,
            )),
        }));

        builder.write_line(&spans);
    });

    builder.page_break();
}

#[cfg(test)]
mod tests {
    use crate::pdf;
    use crate::types::{Config, HighlightedLine, HighlightedToken, RgbColor};

    fn sample_lines() -> Vec<HighlightedLine> {
        vec![
            HighlightedLine {
                line_number: 1,
                tokens: vec![HighlightedToken {
                    text: "fn main() {}".into(),
                    color: RgbColor { r: 0, g: 0, b: 0 },
                    bold: false,
                    italic: false,
                }],
            },
            HighlightedLine {
                line_number: 2,
                tokens: vec![HighlightedToken {
                    text: "// comment".into(),
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

    #[test]
    fn render_file_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render_file(
            &mut builder,
            "test.rs",
            sample_lines().into_iter(),
            2,
            true,
            8,
            "2 lines \u{00B7} 24 B \u{00B7} 2025-01-15",
            None,
        );
    }

    #[test]
    fn render_file_empty_iterator() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render_file(
            &mut builder,
            "empty.rs",
            std::iter::empty(),
            0,
            true,
            8,
            "0 lines \u{00B7} 0 B",
            None,
        );
    }

    #[test]
    fn render_file_without_line_numbers() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render_file(
            &mut builder,
            "test.rs",
            sample_lines().into_iter(),
            2,
            false,
            8,
            "2 lines \u{00B7} 24 B \u{00B7} 2025-01-15",
            None,
        );
    }

    #[test]
    fn render_file_with_header_url() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render_file(
            &mut builder,
            "src/main.rs",
            sample_lines().into_iter(),
            2,
            true,
            8,
            "2 lines \u{00B7} 24 B \u{00B7} 2025-01-15",
            Some("https://github.com/user/repo/blob/abc123/src/main.rs"),
        );
    }

    #[test]
    fn render_file_many_lines() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
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
        super::render_file(
            &mut builder,
            "big.rs",
            lines.into_iter(),
            100,
            true,
            8,
            "100 lines \u{00B7} 1.2 KB \u{00B7} 2025-01-15",
            None,
        );
    }
}

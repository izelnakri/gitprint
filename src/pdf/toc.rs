use std::path::Path;

use printpdf::{Color, Pt, Rgb};

use super::layout::{PageBuilder, Span};

pub fn render(builder: &mut PageBuilder, files: &[(&Path, usize)]) {
    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let gray = Color::Rgb(Rgb::new(0.47, 0.47, 0.47, None));
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));

    builder.write_centered("Table of Contents", &bold, Pt(16.0), black);
    builder.vertical_space(10.0);

    files.iter().for_each(|(path, lines)| {
        builder.write_line(&[
            Span {
                text: path.display().to_string(),
                font_id: regular.clone(),
                size: Pt(8.0),
                color: gray.clone(),
            },
            Span {
                text: format!("  ({lines} lines)"),
                font_id: regular.clone(),
                size: Pt(7.0),
                color: gray.clone(),
            },
        ]);
    });

    builder.page_break();
}

#[cfg(test)]
mod tests {
    use crate::pdf;
    use crate::types::Config;
    use std::path::Path;

    #[test]
    fn render_toc_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let entries: Vec<(&Path, usize)> = vec![
            (Path::new("src/main.rs"), 20),
            (Path::new("src/lib.rs"), 50),
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
        let entries: Vec<(&Path, usize)> = (0..100)
            .map(|i| (Path::new("src/file.rs"), i * 10))
            .collect();
        super::render(&mut builder, &entries);
    }
}

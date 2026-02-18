pub mod code;
pub mod cover;
pub mod fonts;
pub mod layout;
pub mod toc;
pub mod tree;

use std::path::Path;

use printpdf::{Mm, PdfDocument, PdfSaveOptions};

use crate::types::{Config, PaperSize};
use layout::{FontSet, PageBuilder};

fn paper_dimensions(config: &Config) -> (Mm, Mm) {
    let (w, h) = match config.paper_size {
        PaperSize::A4 => (Mm(210.0), Mm(297.0)),
        PaperSize::Letter => (Mm(215.9), Mm(279.4)),
        PaperSize::Legal => (Mm(215.9), Mm(355.6)),
    };
    if config.landscape { (h, w) } else { (w, h) }
}

pub fn create_builder(config: &Config, fonts: FontSet) -> PageBuilder {
    create_builder_at_page(config, fonts, 1)
}

pub fn create_builder_at_page(
    config: &Config,
    fonts: FontSet,
    starting_page: usize,
) -> PageBuilder {
    let (w, h) = paper_dimensions(config);
    let line_height = config.font_size as f32 + 2.0;
    PageBuilder::new(w, h, Mm(10.0), line_height, fonts, starting_page)
}

pub fn save_pdf(doc: &PdfDocument, path: &Path) -> anyhow::Result<()> {
    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    std::fs::write(path, bytes).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Config;

    #[test]
    fn paper_dimensions_a4() {
        let config = Config::test_default();
        let (w, h) = paper_dimensions(&config);
        assert_eq!(w.0, 210.0);
        assert_eq!(h.0, 297.0);
    }

    #[test]
    fn paper_dimensions_letter() {
        let mut config = Config::test_default();
        config.paper_size = PaperSize::Letter;
        let (w, h) = paper_dimensions(&config);
        assert_eq!(w.0, 215.9);
        assert_eq!(h.0, 279.4);
    }

    #[test]
    fn paper_dimensions_landscape() {
        let mut config = Config::test_default();
        config.landscape = true;
        let (w, h) = paper_dimensions(&config);
        assert_eq!(w.0, 297.0);
        assert_eq!(h.0, 210.0);
    }

    #[test]
    fn save_pdf_to_tempfile() {
        let mut doc = PdfDocument::new("test");
        let fonts = fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let builder = create_builder(&config, fonts);
        doc.with_pages(builder.finish());

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.pdf");
        assert!(save_pdf(&doc, &path).is_ok());
        assert!(path.exists());
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
    }

    #[test]
    fn save_pdf_invalid_path() {
        let mut doc = PdfDocument::new("test");
        let _ = fonts::load_fonts(&mut doc).unwrap();
        let result = save_pdf(&doc, Path::new("/nonexistent/dir/test.pdf"));
        assert!(result.is_err());
    }
}

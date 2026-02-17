pub mod code;
pub mod cover;
pub mod fonts;
pub mod toc;
pub mod tree;

use std::path::Path;

use genpdfi::{Element, SimplePageDecorator};

use crate::error::Error;
use crate::types::{Config, PaperSize};

pub fn create_document(config: &Config) -> Result<genpdfi::Document, Error> {
    let font_family = fonts::load_font_family()?;
    let mut doc = genpdfi::Document::new(font_family);

    let paper = match config.paper_size {
        PaperSize::A4 => genpdfi::PaperSize::A4,
        PaperSize::Letter => genpdfi::PaperSize::Letter,
        PaperSize::Legal => genpdfi::PaperSize::Legal,
    };
    doc.set_paper_size(paper);

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(10);
    decorator.set_header(|page| {
        genpdfi::elements::Paragraph::new(format!("- {page} -"))
            .aligned(genpdfi::Alignment::Center)
            .styled(genpdfi::style::Style::new().with_font_size(7))
    });
    doc.set_page_decorator(decorator);

    Ok(doc)
}

pub fn write_pdf(doc: genpdfi::Document, path: &Path) -> Result<(), Error> {
    doc.render_to_file(path)
        .map_err(|e| Error::Pdf(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Config;

    #[test]
    fn create_document_a4() {
        let config = Config::test_default();
        assert!(create_document(&config).is_ok());
    }

    #[test]
    fn create_document_letter() {
        let mut config = Config::test_default();
        config.paper_size = PaperSize::Letter;
        assert!(create_document(&config).is_ok());
    }

    #[test]
    fn create_document_legal() {
        let mut config = Config::test_default();
        config.paper_size = PaperSize::Legal;
        assert!(create_document(&config).is_ok());
    }

    #[test]
    fn write_pdf_to_tempfile() {
        let config = Config::test_default();
        let doc = create_document(&config).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.pdf");
        assert!(write_pdf(doc, &path).is_ok());
        assert!(path.exists());
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
    }

    #[test]
    fn write_pdf_invalid_path() {
        let config = Config::test_default();
        let doc = create_document(&config).unwrap();
        let result = write_pdf(doc, Path::new("/nonexistent/dir/test.pdf"));
        assert!(result.is_err());
    }
}

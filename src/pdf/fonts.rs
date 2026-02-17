use printpdf::{ParsedFont, PdfDocument, PdfWarnMsg};

use super::layout::FontSet;
use crate::error::Error;

const REGULAR: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Bold.ttf");
const ITALIC: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Italic.ttf");
const BOLD_ITALIC: &[u8] = include_bytes!("../../fonts/JetBrainsMono-BoldItalic.ttf");

fn parse_font(bytes: &[u8], label: &str) -> Result<ParsedFont, Error> {
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();
    ParsedFont::from_bytes(bytes, 0, &mut warnings)
        .ok_or_else(|| Error::Font(format!("{label}: failed to parse font")))
}

pub fn load_fonts(doc: &mut PdfDocument) -> Result<FontSet, Error> {
    let regular = parse_font(REGULAR, "regular")?;
    let bold = parse_font(BOLD, "bold")?;
    let italic = parse_font(ITALIC, "italic")?;
    let bold_italic = parse_font(BOLD_ITALIC, "bold-italic")?;

    Ok(FontSet {
        regular: doc.add_font(&regular),
        bold: doc.add_font(&bold),
        italic: doc.add_font(&italic),
        bold_italic: doc.add_font(&bold_italic),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_fonts_succeeds() {
        let mut doc = PdfDocument::new("test");
        assert!(load_fonts(&mut doc).is_ok());
    }

    #[test]
    fn embedded_font_bytes_are_substantial() {
        assert!(REGULAR.len() > 100_000);
        assert!(BOLD.len() > 100_000);
        assert!(ITALIC.len() > 100_000);
        assert!(BOLD_ITALIC.len() > 100_000);
    }
}

use genpdfi::fonts::{FontData, FontFamily};

use crate::error::Error;

const REGULAR: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Bold.ttf");
const ITALIC: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Italic.ttf");
const BOLD_ITALIC: &[u8] = include_bytes!("../../fonts/JetBrainsMono-BoldItalic.ttf");

pub fn load_font_family() -> Result<FontFamily<FontData>, Error> {
    let load = |data: &[u8], label: &str| {
        FontData::new(data.to_vec(), None).map_err(|e| Error::Font(format!("{label}: {e}")))
    };

    Ok(FontFamily {
        regular: load(REGULAR, "regular")?,
        bold: load(BOLD, "bold")?,
        italic: load(ITALIC, "italic")?,
        bold_italic: load(BOLD_ITALIC, "bold-italic")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_font_family_succeeds() {
        assert!(load_font_family().is_ok());
    }

    #[test]
    fn embedded_font_bytes_are_substantial() {
        // Font files should be at least 100KB
        assert!(REGULAR.len() > 100_000);
        assert!(BOLD.len() > 100_000);
        assert!(ITALIC.len() > 100_000);
        assert!(BOLD_ITALIC.len() > 100_000);
    }
}

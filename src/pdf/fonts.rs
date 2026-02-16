use genpdfi::fonts::{FontData, FontFamily};

use crate::error::Error;

const REGULAR: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Bold.ttf");
const ITALIC: &[u8] = include_bytes!("../../fonts/JetBrainsMono-Italic.ttf");
const BOLD_ITALIC: &[u8] = include_bytes!("../../fonts/JetBrainsMono-BoldItalic.ttf");

pub fn load_font_family() -> Result<FontFamily<FontData>, Error> {
    let load = |data: &[u8], label: &str| {
        FontData::new(data.to_vec(), None)
            .map_err(|e| Error::Font(format!("{label}: {e}")))
    };

    Ok(FontFamily {
        regular: load(REGULAR, "regular")?,
        bold: load(BOLD, "bold")?,
        italic: load(ITALIC, "italic")?,
        bold_italic: load(BOLD_ITALIC, "bold-italic")?,
    })
}

use std::path::Path;

use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::error::Error;
use crate::types::{HighlightedLine, HighlightedToken, RgbColor};

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: syntect::highlighting::Theme,
}

impl Highlighter {
    pub fn new(theme_name: &str) -> Result<Self, Error> {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        let theme = theme_set
            .themes
            .get(theme_name)
            .cloned()
            .ok_or_else(|| Error::ThemeNotFound(theme_name.to_string()))?;

        Ok(Self { syntax_set, theme })
    }

    /// Returns a lazy iterator that yields one highlighted line at a time.
    /// Only one line's worth of tokens exists in memory at any point.
    pub fn highlight_lines<'a>(
        &'a self,
        content: &'a str,
        path: &Path,
    ) -> impl Iterator<Item = HighlightedLine> + 'a {
        let syntax = self
            .syntax_set
            .find_syntax_for_file(path)
            .ok()
            .flatten()
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, &self.theme);
        let mut lines = content.lines().enumerate();

        std::iter::from_fn(move || {
            let (i, line_text) = lines.next()?;

            let tokens = h
                .highlight_line(line_text, &self.syntax_set)
                .unwrap_or_default()
                .into_iter()
                .map(|(style, text)| HighlightedToken {
                    text: text.to_string(),
                    color: RgbColor {
                        r: style.foreground.r,
                        g: style.foreground.g,
                        b: style.foreground.b,
                    },
                    bold: style.font_style.contains(FontStyle::BOLD),
                    italic: style.font_style.contains(FontStyle::ITALIC),
                })
                .collect();

            Some(HighlightedLine {
                line_number: i + 1,
                tokens,
            })
        })
    }
}

pub fn list_themes() -> Vec<String> {
    let mut themes: Vec<_> = ThemeSet::load_defaults().themes.into_keys().collect();
    themes.sort();
    themes
}

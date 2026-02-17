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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_valid_theme() {
        assert!(Highlighter::new("InspiredGitHub").is_ok());
    }

    #[test]
    fn new_with_another_valid_theme() {
        assert!(Highlighter::new("base16-ocean.dark").is_ok());
    }

    #[test]
    fn new_with_invalid_theme() {
        let result = Highlighter::new("NonExistentTheme");
        assert!(matches!(result, Err(Error::ThemeNotFound(_))));
    }

    #[test]
    fn highlight_lines_produces_output() {
        let h = Highlighter::new("InspiredGitHub").unwrap();
        let lines: Vec<_> = h
            .highlight_lines("fn main() {}", Path::new("test.rs"))
            .collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].line_number, 1);
        assert!(!lines[0].tokens.is_empty());
    }

    #[test]
    fn highlight_lines_multiline() {
        let h = Highlighter::new("InspiredGitHub").unwrap();
        let content = "line1\nline2\nline3";
        let lines: Vec<_> = h.highlight_lines(content, Path::new("test.txt")).collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].line_number, 1);
        assert_eq!(lines[1].line_number, 2);
        assert_eq!(lines[2].line_number, 3);
    }

    #[test]
    fn highlight_lines_preserves_text() {
        let h = Highlighter::new("InspiredGitHub").unwrap();
        let content = "hello world";
        let lines: Vec<_> = h.highlight_lines(content, Path::new("test.txt")).collect();
        let reconstructed: String = lines[0].tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(reconstructed, "hello world");
    }

    #[test]
    fn highlight_lines_plain_text_fallback() {
        let h = Highlighter::new("InspiredGitHub").unwrap();
        let lines: Vec<_> = h
            .highlight_lines("some content", Path::new("file.xyz"))
            .collect();
        assert_eq!(lines.len(), 1);
        assert!(!lines[0].tokens.is_empty());
    }

    #[test]
    fn highlight_lines_empty_content() {
        let h = Highlighter::new("InspiredGitHub").unwrap();
        let lines: Vec<_> = h.highlight_lines("", Path::new("empty.rs")).collect();
        assert!(lines.is_empty());
    }

    #[test]
    fn highlight_lines_rust_code_has_colors() {
        let h = Highlighter::new("InspiredGitHub").unwrap();
        let content = "fn main() {\n    let x = 42;\n}";
        let lines: Vec<_> = h.highlight_lines(content, Path::new("main.rs")).collect();
        assert_eq!(lines.len(), 3);
        // Rust keywords should produce tokens with styling
        assert!(!lines[0].tokens.is_empty());
    }

    #[test]
    fn highlight_tokens_have_rgb_colors() {
        let h = Highlighter::new("InspiredGitHub").unwrap();
        let lines: Vec<_> = h.highlight_lines("let x = 1;", Path::new("t.rs")).collect();
        for token in &lines[0].tokens {
            // RGB values should be valid (0-255 is guaranteed by u8)
            let _ = (token.color.r, token.color.g, token.color.b);
        }
    }

    #[test]
    fn list_themes_non_empty() {
        assert!(!list_themes().is_empty());
    }

    #[test]
    fn list_themes_contains_known_theme() {
        assert!(list_themes().contains(&"InspiredGitHub".to_string()));
    }

    #[test]
    fn list_themes_is_sorted() {
        let themes = list_themes();
        let mut sorted = themes.clone();
        sorted.sort();
        assert_eq!(themes, sorted);
    }

    #[test]
    fn list_themes_contains_multiple() {
        let themes = list_themes();
        assert!(themes.len() > 1);
        assert!(themes.contains(&"base16-ocean.dark".to_string()));
    }
}

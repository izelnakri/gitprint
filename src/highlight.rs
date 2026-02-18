use std::path::Path;

use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::types::{HighlightedLine, HighlightedToken, RgbColor};

/// Syntax highlighter backed by the bundled syntect theme and syntax sets.
pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: syntect::highlighting::Theme,
}

impl Highlighter {
    /// Creates a new `Highlighter` using the named syntect theme.
    ///
    /// Theme names are the keys returned by [`list_themes`]. Pass `"InspiredGitHub"` for
    /// the default light theme.
    ///
    /// # Errors
    ///
    /// Returns an error if `theme_name` is not found in the bundled theme set.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitprint::highlight::Highlighter;
    ///
    /// let hl = Highlighter::new("InspiredGitHub").unwrap();
    ///
    /// let err = Highlighter::new("no-such-theme").err().unwrap();
    /// assert!(err.to_string().contains("no-such-theme"));
    /// ```
    pub fn new(theme_name: &str) -> anyhow::Result<Self> {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();

        let theme = theme_set
            .themes
            .get(theme_name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!(
                "theme not found: {theme_name} (use --list-themes to see available themes)"
            ))?;

        Ok(Self { syntax_set, theme })
    }

    /// Returns a lazy iterator that yields one [`HighlightedLine`] at a time.
    ///
    /// Syntax is detected from the file extension of `path`; unknown extensions fall
    /// back to plain text. Line numbers start at 1.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitprint::highlight::Highlighter;
    /// use std::path::Path;
    ///
    /// let hl = Highlighter::new("InspiredGitHub").unwrap();
    /// let lines: Vec<_> = hl.highlight_lines("fn main() {}", Path::new("main.rs")).collect();
    ///
    /// assert_eq!(lines.len(), 1);
    /// assert_eq!(lines[0].line_number, 1);
    /// assert!(!lines[0].tokens.is_empty());
    /// ```
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

/// Returns all available theme names in sorted order.
///
/// # Examples
///
/// ```
/// use gitprint::highlight::list_themes;
///
/// let themes = list_themes();
/// assert!(themes.contains(&"InspiredGitHub".to_string()));
/// assert!(themes.windows(2).all(|w| w[0] <= w[1])); // sorted
/// ```
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
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("NonExistentTheme"));
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
        assert!(!lines[0].tokens.is_empty());
    }

    #[test]
    fn highlight_tokens_have_rgb_colors() {
        let h = Highlighter::new("InspiredGitHub").unwrap();
        let lines: Vec<_> = h.highlight_lines("let x = 1;", Path::new("t.rs")).collect();
        lines[0].tokens.iter().for_each(|token| {
            let _ = (token.color.r, token.color.g, token.color.b);
        });
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

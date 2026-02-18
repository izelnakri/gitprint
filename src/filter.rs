use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::defaults::DEFAULT_EXCLUDES;

/// Filters file paths based on glob include/exclude patterns.
///
/// Exclude patterns always take precedence over include patterns.
/// Default excludes (lock files, binaries, build artifacts) are always applied.
pub struct FileFilter {
    include_set: Option<GlobSet>,
    exclude_set: GlobSet,
}

impl FileFilter {
    /// Creates a new `FileFilter` from glob include and exclude patterns.
    ///
    /// An empty `include_patterns` slice allows all files (subject to excludes).
    /// Default excludes (lock files, build artifacts, binaries, etc.) are always applied.
    ///
    /// # Errors
    ///
    /// Returns an error if any glob pattern is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitprint::filter::FileFilter;
    /// use std::path::Path;
    ///
    /// // Include only Rust files, exclude test helpers
    /// let filter = FileFilter::new(
    ///     &["*.rs".to_string()],
    ///     &["test_*.rs".to_string()],
    /// ).unwrap();
    ///
    /// assert!(filter.should_include(Path::new("main.rs")));
    /// assert!(!filter.should_include(Path::new("test_helper.rs")));
    /// assert!(!filter.should_include(Path::new("README.md")));
    /// ```
    pub fn new(include_patterns: &[String], exclude_patterns: &[String]) -> anyhow::Result<Self> {
        let include_set = if include_patterns.is_empty() {
            None
        } else {
            let set = include_patterns
                .iter()
                .try_fold(GlobSetBuilder::new(), |mut b, p| {
                    b.add(
                        Glob::new(p)
                            .map_err(|e| anyhow::anyhow!("invalid glob pattern '{p}': {e}"))?,
                    );
                    Ok::<_, anyhow::Error>(b)
                })?
                .build()
                .map_err(|e| anyhow::anyhow!("failed to build glob set: {e}"))?;
            Some(set)
        };

        let exclude_set = DEFAULT_EXCLUDES
            .iter()
            .map(|p| Glob::new(p).unwrap())
            .chain(
                exclude_patterns
                    .iter()
                    .map(|p| {
                        Glob::new(p).map_err(|e| anyhow::anyhow!("invalid glob pattern '{p}': {e}"))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?
                    .into_iter(),
            )
            .fold(GlobSetBuilder::new(), |mut b, g| {
                b.add(g);
                b
            })
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build glob set: {e}"))?;

        Ok(Self {
            include_set,
            exclude_set,
        })
    }

    /// Returns `true` if `path` should be included given the configured patterns.
    ///
    /// Exclude patterns always win over include patterns.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitprint::filter::FileFilter;
    /// use std::path::Path;
    ///
    /// let filter = FileFilter::new(&["*.rs".to_string()], &[]).unwrap();
    /// assert!(filter.should_include(Path::new("src/lib.rs")));
    /// assert!(!filter.should_include(Path::new("Cargo.toml")));
    /// assert!(!filter.should_include(Path::new("Cargo.lock"))); // default exclude
    /// ```
    pub fn should_include(&self, path: &Path) -> bool {
        if self.exclude_set.is_match(path) {
            return false;
        }
        self.include_set
            .as_ref()
            .is_none_or(|set| set.is_match(path))
    }

    /// Filters a list of paths, retaining only those that pass `should_include`.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitprint::filter::FileFilter;
    /// use std::path::PathBuf;
    ///
    /// let filter = FileFilter::new(&["*.rs".to_string()], &[]).unwrap();
    /// let paths = vec![
    ///     PathBuf::from("main.rs"),
    ///     PathBuf::from("README.md"),
    ///     PathBuf::from("lib.rs"),
    /// ];
    /// let kept: Vec<_> = filter.filter_paths(paths).collect();
    /// assert_eq!(kept, vec![PathBuf::from("main.rs"), PathBuf::from("lib.rs")]);
    /// ```
    pub fn filter_paths(&self, paths: Vec<PathBuf>) -> impl Iterator<Item = PathBuf> + '_ {
        paths.into_iter().filter(|p| self.should_include(p))
    }
}

/// Returns `true` if the content appears to be a binary file.
///
/// Detection is based on the presence of non-text byte sequences (e.g. null bytes).
///
/// # Examples
///
/// ```
/// use gitprint::filter::is_binary;
///
/// assert!(is_binary(b"hello\x00world")); // null byte → binary
/// assert!(!is_binary(b"fn main() {}")); // valid UTF-8 → not binary
/// assert!(!is_binary(b""));
/// ```
pub fn is_binary(content: &[u8]) -> bool {
    content_inspector::inspect(content).is_binary()
}

/// Returns `true` if the content appears to be minified.
///
/// A file is considered minified when any of its first 5 lines exceeds 500 characters,
/// which is characteristic of bundled or minified JavaScript/CSS.
///
/// # Examples
///
/// ```
/// use gitprint::filter::is_minified;
///
/// assert!(is_minified(&"x".repeat(501)));   // single very long line
/// assert!(!is_minified("fn main() {\n    println!(\"hello\");\n}\n"));
/// assert!(!is_minified(""));
/// ```
pub fn is_minified(content: &str) -> bool {
    content.lines().take(5).any(|line| line.len() > 500)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_excludes_applied() {
        let filter = FileFilter::new(&[], &[]).unwrap();
        assert!(!filter.should_include(Path::new("Cargo.lock")));
        assert!(!filter.should_include(Path::new("node_modules/foo.js")));
        assert!(!filter.should_include(Path::new("image.png")));
        assert!(!filter.should_include(Path::new("target/debug/binary")));
        assert!(!filter.should_include(Path::new(".git/HEAD")));
        assert!(!filter.should_include(Path::new("bundle.min.js")));
    }

    #[test]
    fn custom_exclude() {
        let filter = FileFilter::new(&[], &["*.md".to_string()]).unwrap();
        assert!(!filter.should_include(Path::new("README.md")));
        assert!(!filter.should_include(Path::new("docs/GUIDE.md")));
        assert!(filter.should_include(Path::new("main.rs")));
    }

    #[test]
    fn include_only() {
        let filter = FileFilter::new(&["*.rs".to_string()], &[]).unwrap();
        assert!(filter.should_include(Path::new("main.rs")));
        assert!(filter.should_include(Path::new("src/lib.rs")));
        assert!(!filter.should_include(Path::new("README.md")));
        assert!(!filter.should_include(Path::new("Cargo.toml")));
    }

    #[test]
    fn include_and_exclude_interaction() {
        let filter = FileFilter::new(&["*.rs".to_string()], &["test_*.rs".to_string()]).unwrap();
        assert!(filter.should_include(Path::new("main.rs")));
        assert!(!filter.should_include(Path::new("test_helper.rs")));
    }

    #[test]
    fn empty_filter_includes_normal_files() {
        let filter = FileFilter::new(&[], &[]).unwrap();
        assert!(filter.should_include(Path::new("src/main.rs")));
        assert!(filter.should_include(Path::new("Cargo.toml")));
        assert!(filter.should_include(Path::new("README.md")));
    }

    #[test]
    fn multiple_include_patterns() {
        let filter = FileFilter::new(&["*.rs".to_string(), "*.toml".to_string()], &[]).unwrap();
        assert!(filter.should_include(Path::new("main.rs")));
        assert!(filter.should_include(Path::new("Cargo.toml")));
        assert!(!filter.should_include(Path::new("README.md")));
    }

    #[test]
    fn multiple_exclude_patterns() {
        let filter = FileFilter::new(&[], &["*.md".to_string(), "*.txt".to_string()]).unwrap();
        assert!(!filter.should_include(Path::new("README.md")));
        assert!(!filter.should_include(Path::new("notes.txt")));
        assert!(filter.should_include(Path::new("main.rs")));
    }

    #[test]
    fn exclude_takes_precedence_over_include() {
        let filter = FileFilter::new(&["*.rs".to_string()], &["main.rs".to_string()]).unwrap();
        assert!(!filter.should_include(Path::new("main.rs")));
        assert!(filter.should_include(Path::new("lib.rs")));
    }

    #[test]
    fn filter_paths_works() {
        let filter = FileFilter::new(&["*.rs".to_string()], &[]).unwrap();
        let paths = vec![
            PathBuf::from("main.rs"),
            PathBuf::from("README.md"),
            PathBuf::from("lib.rs"),
        ];
        let filtered: Vec<_> = filter.filter_paths(paths).collect();
        assert_eq!(
            filtered,
            vec![PathBuf::from("main.rs"), PathBuf::from("lib.rs")]
        );
    }

    #[test]
    fn filter_paths_empty_input() {
        let filter = FileFilter::new(&[], &[]).unwrap();
        let filtered: Vec<_> = filter.filter_paths(vec![]).collect();
        assert!(filtered.is_empty());
    }

    #[test]
    fn is_binary_with_null_bytes() {
        let content = b"hello\x00world";
        assert!(is_binary(content));
    }

    #[test]
    fn is_binary_with_text() {
        let content = b"fn main() { println!(\"hello\"); }";
        assert!(!is_binary(content));
    }

    #[test]
    fn is_binary_with_empty() {
        assert!(!is_binary(b""));
    }

    #[test]
    fn is_binary_with_utf8() {
        assert!(!is_binary("こんにちは世界".as_bytes()));
    }

    #[test]
    fn is_minified_with_long_line() {
        let long_line = "a".repeat(501);
        assert!(is_minified(&long_line));
    }

    #[test]
    fn is_minified_with_normal_content() {
        assert!(!is_minified("fn main() {\n    println!(\"hi\");\n}\n"));
    }

    #[test]
    fn is_minified_long_line_after_fifth() {
        let mut content = "short\n".repeat(5);
        content.push_str(&"a".repeat(501));
        assert!(!is_minified(&content));
    }

    #[test]
    fn is_minified_exactly_500_chars() {
        let line = "a".repeat(500);
        assert!(!is_minified(&line));
    }

    #[test]
    fn is_minified_empty() {
        assert!(!is_minified(""));
    }

    #[test]
    fn is_minified_long_line_on_line_3() {
        let content = format!("short\nshort\n{}\nshort\nshort\n", "a".repeat(501));
        assert!(is_minified(&content));
    }

    #[test]
    fn invalid_include_glob_returns_error() {
        let result = FileFilter::new(&["[invalid".to_string()], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_exclude_glob_returns_error() {
        let result = FileFilter::new(&[], &["[invalid".to_string()]);
        assert!(result.is_err());
    }
}

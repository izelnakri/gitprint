pub const DEFAULT_EXCLUDES: &[&str] = &[
    // Lock files
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "Cargo.lock",
    "Gemfile.lock",
    "composer.lock",
    "poetry.lock",
    "Pipfile.lock",
    "flake.lock",
    // Build output
    "node_modules/**",
    "target/**",
    "dist/**",
    "build/**",
    ".next/**",
    "__pycache__/**",
    "*.pyc",
    // VCS / IDE
    ".git/**",
    ".svn/**",
    ".idea/**",
    ".vscode/**",
    "*.swp",
    "*.swo",
    ".DS_Store",
    // Images
    "*.png",
    "*.jpg",
    "*.jpeg",
    "*.gif",
    "*.ico",
    "*.svg",
    "*.webp",
    "*.bmp",
    // Fonts
    "*.woff",
    "*.woff2",
    "*.ttf",
    "*.otf",
    "*.eot",
    // Archives / binaries
    "*.zip",
    "*.tar",
    "*.gz",
    "*.bz2",
    "*.xz",
    "*.7z",
    "*.rar",
    "*.exe",
    "*.dll",
    "*.so",
    "*.dylib",
    "*.o",
    "*.a",
    "*.class",
    "*.jar",
    "*.war",
    "*.wasm",
    // Generated / minified
    "*.min.js",
    "*.min.css",
    "*.map",
    "*.bundle.js",
    // Data
    "*.sqlite",
    "*.db",
    "*.pdf",
];

#[cfg(test)]
mod tests {
    use super::*;
    use globset::Glob;

    #[test]
    fn default_excludes_has_entries() {
        assert!(DEFAULT_EXCLUDES.len() > 10);
    }

    #[test]
    fn all_patterns_are_valid_globs() {
        for pattern in DEFAULT_EXCLUDES {
            Glob::new(pattern).unwrap_or_else(|e| panic!("invalid glob '{pattern}': {e}"));
        }
    }

    #[test]
    fn known_lock_files_present() {
        assert!(DEFAULT_EXCLUDES.contains(&"Cargo.lock"));
        assert!(DEFAULT_EXCLUDES.contains(&"package-lock.json"));
        assert!(DEFAULT_EXCLUDES.contains(&"yarn.lock"));
        assert!(DEFAULT_EXCLUDES.contains(&"poetry.lock"));
        assert!(DEFAULT_EXCLUDES.contains(&"flake.lock"));
    }

    #[test]
    fn known_build_dirs_present() {
        assert!(DEFAULT_EXCLUDES.contains(&"node_modules/**"));
        assert!(DEFAULT_EXCLUDES.contains(&"target/**"));
        assert!(DEFAULT_EXCLUDES.contains(&"dist/**"));
        assert!(DEFAULT_EXCLUDES.contains(&"__pycache__/**"));
    }

    #[test]
    fn known_image_extensions_present() {
        assert!(DEFAULT_EXCLUDES.contains(&"*.png"));
        assert!(DEFAULT_EXCLUDES.contains(&"*.jpg"));
        assert!(DEFAULT_EXCLUDES.contains(&"*.svg"));
    }

    #[test]
    fn known_binary_extensions_present() {
        assert!(DEFAULT_EXCLUDES.contains(&"*.exe"));
        assert!(DEFAULT_EXCLUDES.contains(&"*.wasm"));
        assert!(DEFAULT_EXCLUDES.contains(&"*.zip"));
    }

    #[test]
    fn known_generated_extensions_present() {
        assert!(DEFAULT_EXCLUDES.contains(&"*.min.js"));
        assert!(DEFAULT_EXCLUDES.contains(&"*.min.css"));
        assert!(DEFAULT_EXCLUDES.contains(&"*.map"));
    }

    #[test]
    fn vcs_and_ide_dirs_present() {
        assert!(DEFAULT_EXCLUDES.contains(&".git/**"));
        assert!(DEFAULT_EXCLUDES.contains(&".idea/**"));
        assert!(DEFAULT_EXCLUDES.contains(&".vscode/**"));
        assert!(DEFAULT_EXCLUDES.contains(&".DS_Store"));
    }
}

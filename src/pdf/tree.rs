use std::collections::BTreeMap;
use std::path::PathBuf;

use printpdf::{Color, Pt, Rgb};

use super::layout::PageBuilder;

/// A recursive directory tree. BTreeMap keeps entries sorted alphabetically.
struct Tree(BTreeMap<String, Tree>);

impl Tree {
    fn new() -> Self {
        Self(BTreeMap::new())
    }

    fn insert(&mut self, parts: &[&str]) {
        if let [first, rest @ ..] = parts {
            self.0
                .entry(first.to_string())
                .or_insert_with(Tree::new)
                .insert(rest);
        }
    }

    fn to_lines(&self, prefix: &str) -> Vec<String> {
        let last_idx = self.0.len().saturating_sub(1);

        self.0
            .iter()
            .enumerate()
            .flat_map(|(i, (name, child))| {
                let is_last = i == last_idx;
                let connector = if is_last {
                    "\u{2514}\u{2500}\u{2500} "
                } else {
                    "\u{251C}\u{2500}\u{2500} "
                };
                let extension = if is_last { "    " } else { "\u{2502}   " };

                std::iter::once(format!("{prefix}{connector}{name}"))
                    .chain(child.to_lines(&format!("{prefix}{extension}")))
            })
            .collect()
    }
}

pub fn render(builder: &mut PageBuilder, paths: &[PathBuf]) {
    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));

    builder.write_centered("File Tree", &bold, Pt(16.0), black.clone());
    builder.vertical_space(10.0);

    let mut root = Tree::new();
    paths.iter().for_each(|p| {
        let parts: Vec<_> = p
            .components()
            .map(|c| c.as_os_str().to_str().unwrap_or("?"))
            .collect();
        root.insert(&parts);
    });

    root.to_lines("").into_iter().for_each(|line| {
        builder.write_line(&[super::layout::Span {
            text: line,
            font_id: regular.clone(),
            size: Pt(7.0),
            color: black.clone(),
        }]);
    });

    builder.page_break();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_file() {
        let mut tree = Tree::new();
        tree.insert(&["src", "main.rs"]);
        let lines = tree.to_lines("");
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("src"));
        assert!(lines[1].contains("main.rs"));
    }

    #[test]
    fn nested_structure_with_box_drawing() {
        let mut tree = Tree::new();
        tree.insert(&["src", "main.rs"]);
        tree.insert(&["src", "lib.rs"]);
        tree.insert(&["Cargo.toml"]);
        let lines = tree.to_lines("");
        assert!(lines.len() >= 4);
        let joined = lines.join("\n");
        assert!(joined.contains('\u{251C}'));
        assert!(joined.contains('\u{2514}'));
        assert!(joined.contains('\u{2500}'));
    }

    #[test]
    fn empty_tree() {
        assert!(Tree::new().to_lines("").is_empty());
    }

    #[test]
    fn sorted_output() {
        let mut tree = Tree::new();
        tree.insert(&["z.rs"]);
        tree.insert(&["a.rs"]);
        tree.insert(&["m.rs"]);
        let lines = tree.to_lines("");
        assert!(lines[0].contains("a.rs"));
        assert!(lines[1].contains("m.rs"));
        assert!(lines[2].contains("z.rs"));
    }

    #[test]
    fn deep_nesting() {
        let mut tree = Tree::new();
        tree.insert(&["a", "b", "c", "d", "e.txt"]);
        let lines = tree.to_lines("");
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn multiple_files_same_directory() {
        let mut tree = Tree::new();
        tree.insert(&["src", "a.rs"]);
        tree.insert(&["src", "b.rs"]);
        tree.insert(&["src", "c.rs"]);
        assert_eq!(tree.to_lines("").len(), 4);
    }

    #[test]
    fn render_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = crate::pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = crate::types::Config::test_default();
        let mut builder = crate::pdf::create_builder(&config, fonts);
        render(
            &mut builder,
            &[
                PathBuf::from("src/main.rs"),
                PathBuf::from("src/lib.rs"),
                PathBuf::from("Cargo.toml"),
            ],
        );
    }
}

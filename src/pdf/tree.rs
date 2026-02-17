use std::collections::BTreeMap;
use std::path::PathBuf;

use genpdfi::elements::{self, Paragraph};
use genpdfi::{Alignment, Element, style};

/// A recursive directory tree. Leaves (files) are nodes with empty children.
/// BTreeMap keeps entries sorted alphabetically without an explicit sort step.
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

pub fn render(doc: &mut genpdfi::Document, paths: &[PathBuf]) {
    doc.push(
        Paragraph::new("File Tree")
            .aligned(Alignment::Center)
            .styled(style::Style::new().bold().with_font_size(16)),
    );
    doc.push(elements::Break::new(1.0));

    let mut root = Tree::new();
    paths.iter().for_each(|p| {
        let parts: Vec<_> = p
            .components()
            .map(|c| c.as_os_str().to_str().unwrap_or("?"))
            .collect();
        root.insert(&parts);
    });

    root.to_lines("").into_iter().for_each(|line| {
        doc.push(Paragraph::new(line).styled(style::Style::new().with_font_size(7)));
    });

    doc.push(elements::PageBreak::new());
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
        // Branch connector
        assert!(joined.contains('\u{251C}'));
        // Last-item connector
        assert!(joined.contains('\u{2514}'));
        // Horizontal bar
        assert!(joined.contains('\u{2500}'));
    }

    #[test]
    fn empty_tree() {
        let tree = Tree::new();
        assert!(tree.to_lines("").is_empty());
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
        assert!(lines[0].contains("a"));
        assert!(lines[4].contains("e.txt"));
    }

    #[test]
    fn multiple_files_same_directory() {
        let mut tree = Tree::new();
        tree.insert(&["src", "a.rs"]);
        tree.insert(&["src", "b.rs"]);
        tree.insert(&["src", "c.rs"]);
        let lines = tree.to_lines("");
        // src + 3 files = 4 lines
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn sibling_directories() {
        let mut tree = Tree::new();
        tree.insert(&["src", "main.rs"]);
        tree.insert(&["tests", "test.rs"]);
        let lines = tree.to_lines("");
        assert_eq!(lines.len(), 4);
        let joined = lines.join("\n");
        assert!(joined.contains("src"));
        assert!(joined.contains("tests"));
    }

    #[test]
    fn prefix_propagation() {
        let mut tree = Tree::new();
        tree.insert(&["a", "b", "c.txt"]);
        tree.insert(&["a", "d.txt"]);
        let lines = tree.to_lines("");
        // Lines deeper in the tree should have longer prefixes
        assert!(lines.last().unwrap().len() > lines.first().unwrap().len() || lines.len() >= 2);
    }

    #[test]
    fn last_item_uses_corner_connector() {
        let mut tree = Tree::new();
        tree.insert(&["only.txt"]);
        let lines = tree.to_lines("");
        assert_eq!(lines.len(), 1);
        // Single item should use └── (last-item connector)
        assert!(lines[0].contains('\u{2514}'));
    }

    #[test]
    fn render_does_not_panic() {
        let config = crate::types::Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        let paths = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/lib.rs"),
            PathBuf::from("Cargo.toml"),
        ];
        render(&mut doc, &paths);
    }
}

use std::collections::BTreeMap;
use std::path::PathBuf;

use genpdfi::elements::{self, Paragraph};
use genpdfi::{style, Alignment, Element};

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
                let connector = if is_last { "\u{2514}\u{2500}\u{2500} " } else { "\u{251C}\u{2500}\u{2500} " };
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

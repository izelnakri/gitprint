use genpdfi::elements::{self, Paragraph};
use genpdfi::{Alignment, Element, style};

use crate::types::RepoMetadata;

pub fn render(doc: &mut genpdfi::Document, metadata: &RepoMetadata) {
    let file_count = metadata.file_count.to_string();
    let total_lines = metadata.total_lines.to_string();

    doc.push(elements::Break::new(6.0));
    doc.push(
        Paragraph::new(&metadata.name)
            .aligned(Alignment::Center)
            .styled(style::Style::new().bold().with_font_size(28)),
    );
    doc.push(elements::Break::new(4.0));

    [
        ("Branch:", metadata.branch.as_str()),
        ("Commit:", metadata.commit_hash_short.as_str()),
        ("Date:", metadata.commit_date.as_str()),
        ("Message:", metadata.commit_message.as_str()),
        ("Files:", file_count.as_str()),
        ("Lines:", total_lines.as_str()),
    ]
    .iter()
    .for_each(|(label, value)| {
        doc.push(
            Paragraph::default()
                .styled_string(*label, style::Style::new().bold().with_font_size(10))
                .styled_string(format!("  {value}"), style::Style::new().with_font_size(10))
                .aligned(Alignment::Center),
        );
        doc.push(elements::Break::new(0.3));
    });

    doc.push(elements::PageBreak::new());
}

#[cfg(test)]
mod tests {
    use crate::types::{Config, RepoMetadata};

    fn test_metadata() -> RepoMetadata {
        RepoMetadata {
            name: "test-repo".to_string(),
            branch: "main".to_string(),
            commit_hash: "abc1234567890abcdef1234567890abcdef123456".to_string(),
            commit_hash_short: "abc1234".to_string(),
            commit_date: "2024-01-01 12:00:00 +0000".to_string(),
            commit_message: "initial commit".to_string(),
            file_count: 5,
            total_lines: 100,
        }
    }

    #[test]
    fn render_cover_does_not_panic() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        super::render(&mut doc, &test_metadata());
    }

    #[test]
    fn render_cover_with_empty_metadata() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        let meta = RepoMetadata {
            name: String::new(),
            branch: String::new(),
            commit_hash: String::new(),
            commit_hash_short: String::new(),
            commit_date: String::new(),
            commit_message: String::new(),
            file_count: 0,
            total_lines: 0,
        };
        super::render(&mut doc, &meta);
    }

    #[test]
    fn render_cover_with_large_counts() {
        let config = Config::test_default();
        let mut doc = crate::pdf::create_document(&config).unwrap();
        let mut meta = test_metadata();
        meta.file_count = 999_999;
        meta.total_lines = 10_000_000;
        super::render(&mut doc, &meta);
    }
}

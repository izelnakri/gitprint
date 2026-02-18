use printpdf::{Color, Pt, Rgb};

use super::layout::{PageBuilder, Span};
use crate::types::RepoMetadata;

pub fn render(builder: &mut PageBuilder, metadata: &RepoMetadata) {
    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));

    builder.vertical_space(120.0);
    builder.write_centered(&metadata.name, &bold, Pt(28.0), black.clone());
    builder.vertical_space(40.0);

    [
        ("Branch:  ", metadata.branch.as_str()),
        ("Commit:  ", metadata.commit_hash_short.as_str()),
        ("Date:    ", metadata.commit_date.as_str()),
        ("Message: ", metadata.commit_message.as_str()),
        ("Files:   ", &metadata.file_count.to_string()),
        ("Lines:   ", &metadata.total_lines.to_string()),
    ]
    .into_iter()
    .filter(|(_, value)| !value.is_empty())
    .for_each(|(label, value)| {
        builder.write_line_centered(&[
            Span {
                text: label.into(),
                font_id: bold.clone(),
                size: Pt(10.0),
                color: black.clone(),
            },
            Span {
                text: value.into(),
                font_id: regular.clone(),
                size: Pt(10.0),
                color: black.clone(),
            },
        ]);
        builder.vertical_space(3.0);
    });

    builder.page_break();
}

#[cfg(test)]
mod tests {
    use crate::pdf;
    use crate::types::{Config, RepoMetadata};

    fn test_metadata() -> RepoMetadata {
        RepoMetadata {
            name: "test-repo".into(),
            branch: "main".into(),
            commit_hash: "abc1234567890abcdef1234567890abcdef123456".into(),
            commit_hash_short: "abc1234".into(),
            commit_date: "2024-01-01 12:00:00 +0000".into(),
            commit_message: "initial commit".into(),
            file_count: 5,
            total_lines: 100,
        }
    }

    #[test]
    fn render_cover_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render(&mut builder, &test_metadata());
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_cover_with_empty_metadata() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render(
            &mut builder,
            &RepoMetadata {
                name: String::new(),
                branch: String::new(),
                commit_hash: String::new(),
                commit_hash_short: String::new(),
                commit_date: String::new(),
                commit_message: String::new(),
                file_count: 0,
                total_lines: 0,
            },
        );
    }
}

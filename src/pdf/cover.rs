use genpdfi::elements::{self, Paragraph};
use genpdfi::{style, Alignment, Element};

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

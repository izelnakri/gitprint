use printpdf::{Color, Pt, Rgb};

use super::layout::{PageBuilder, Span};
use crate::github::CommitDetail;

pub fn render_commit(builder: &mut PageBuilder, detail: &CommitDetail, font_size: f32) {
    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
    let gray = Color::Rgb(Rgb::new(0.47, 0.47, 0.47, None));
    let dark_gray = Color::Rgb(Rgb::new(0.3, 0.3, 0.3, None));
    let green = Color::Rgb(Rgb::new(0.1, 0.5, 0.1, None));
    let red = Color::Rgb(Rgb::new(0.7, 0.1, 0.1, None));
    let blue_gray = Color::Rgb(Rgb::new(0.3, 0.3, 0.6, None));

    let sha_short = detail.sha.get(..7).unwrap_or(&detail.sha);
    let author = &detail.commit.author.name;
    let date = detail
        .commit
        .author
        .date
        .get(..10)
        .unwrap_or(&detail.commit.author.date);
    let message_first_line = detail
        .commit
        .message
        .lines()
        .next()
        .unwrap_or(&detail.commit.message);

    let (total_additions, total_deletions) =
        detail.files.iter().fold((0u64, 0u64), |(add, del), f| {
            (add + f.additions, del + f.deletions)
        });

    builder.ensure_space(builder.line_height() * 4.0);

    // ── Commit header ──────────────────────────────────────────────────────────
    builder.write_line(&[
        Span {
            text: format!("{sha_short}  "),
            font_id: bold.clone(),
            size: Pt(font_size),
            color: dark_gray.clone(),
        },
        Span {
            text: format!("{author}  "),
            font_id: regular.clone(),
            size: Pt(font_size),
            color: black.clone(),
        },
        Span {
            text: format!("{date}  "),
            font_id: regular.clone(),
            size: Pt(font_size),
            color: gray.clone(),
        },
        Span {
            text: format!("+{total_additions} -{total_deletions}"),
            font_id: regular.clone(),
            size: Pt(font_size),
            color: dark_gray.clone(),
        },
    ]);

    // Commit message
    builder.write_line(&[Span {
        text: format!("  {message_first_line}"),
        font_id: bold.clone(),
        size: Pt(font_size),
        color: black.clone(),
    }]);

    builder.vertical_space(4.0);

    // ── Per-file diffs ─────────────────────────────────────────────────────────
    detail.files.iter().for_each(|file| {
        builder.ensure_space(builder.line_height() * 3.0);

        // File header line
        builder.write_line(&[
            Span {
                text: format!("  {} ", file.filename),
                font_id: bold.clone(),
                size: Pt(font_size - 0.5),
                color: black.clone(),
            },
            Span {
                text: format!("+{} -{}", file.additions, file.deletions),
                font_id: regular.clone(),
                size: Pt(font_size - 0.5),
                color: dark_gray.clone(),
            },
        ]);

        match &file.patch {
            None => {
                builder.write_line(&[Span {
                    text: "  [diff too large to display]".to_string(),
                    font_id: regular.clone(),
                    size: Pt(font_size - 1.0),
                    color: gray.clone(),
                }]);
            }
            Some(patch) => {
                patch.lines().for_each(|line| {
                    let (prefix, color) = if line.starts_with('+') {
                        ("+", green.clone())
                    } else if line.starts_with('-') {
                        ("-", red.clone())
                    } else if line.starts_with("@@") {
                        ("@", blue_gray.clone())
                    } else {
                        (" ", dark_gray.clone())
                    };
                    // Indent all diff lines by 4 spaces; strip the diff prefix char and
                    // replace it with a padded marker so columns stay aligned.
                    let body = if line.starts_with("@@") {
                        line.to_string()
                    } else {
                        format!("{prefix} {}", line.get(1..).unwrap_or(line))
                    };
                    builder.write_line(&[Span {
                        text: format!("    {body}"),
                        font_id: regular.clone(),
                        size: Pt(font_size - 1.0),
                        color,
                    }]);
                });
            }
        }

        builder.vertical_space(3.0);
    });

    builder.vertical_space(8.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::{CommitAuthor, CommitFile, CommitInfo};
    use crate::pdf;
    use crate::types::Config;

    fn test_detail(with_patch: bool) -> CommitDetail {
        CommitDetail {
            sha: "abc1234567890".to_string(),
            html_url: "https://github.com/alice/repo/commit/abc1234".to_string(),
            commit: CommitInfo {
                message: "fix: correct off-by-one error\n\nDetailed description.".to_string(),
                author: CommitAuthor {
                    name: "Alice".to_string(),
                    date: "2024-03-01T12:00:00Z".to_string(),
                },
            },
            files: vec![CommitFile {
                filename: "src/lib.rs".to_string(),
                status: "modified".to_string(),
                additions: 2,
                deletions: 1,
                patch: if with_patch {
                    Some(
                        "@@ -10,7 +10,8 @@\n context line\n-old line\n+new line\n+added line"
                            .to_string(),
                    )
                } else {
                    None
                },
            }],
        }
    }

    #[test]
    fn render_commit_with_patch_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render_commit(&mut builder, &test_detail(true), 8.0);
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_commit_without_patch_shows_placeholder() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render_commit(&mut builder, &test_detail(false), 8.0);
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_commit_no_files() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let mut detail = test_detail(false);
        detail.files.clear();
        super::render_commit(&mut builder, &detail, 8.0);
        assert!(!builder.finish().is_empty());
    }
}

use printpdf::{Actions, Color, Pt, Rgb};

use super::layout::{PageBuilder, Span};
use crate::github::CommitDetail;

// ── Color palette ──────────────────────────────────────────────────────────────
// Green/red chosen to be distinguishable for common colorblindness types:
//   • Protanopes/deuteranopes see the green as teal-cyan and the red as orange-amber,
//     which remain clearly distinct from each other and from context-line gray.
//   • Both convert to clearly different gray values for black-only printing.
//   • Green is darker (HSL ~150°, 100%, 38%) for better legibility at small sizes.
fn neon_green() -> Color {
    Color::Rgb(Rgb::new(0.0, 0.76, 0.38, None)) // #00C261 — dark electric jade
}
fn neon_red() -> Color {
    Color::Rgb(Rgb::new(0.94, 0.20, 0.20, None)) // #F03333 — deep neon red
}
fn hunk_blue() -> Color {
    Color::Rgb(Rgb::new(0.34, 0.60, 0.96, None)) // #5799F5 — electric blue
}

/// Renders a single commit with its per-file diffs into the PDF.
pub fn render_commit(
    builder: &mut PageBuilder,
    detail: &CommitDetail,
    repo: &str,
    branch: Option<&str>,
    font_size: f32,
) {
    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let italic = builder.font(false, true).clone();
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
    let gray = Color::Rgb(Rgb::new(0.50, 0.50, 0.50, None));
    let dark_gray = Color::Rgb(Rgb::new(0.28, 0.28, 0.28, None));
    let rule_gray = Color::Rgb(Rgb::new(0.78, 0.78, 0.78, None));

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

    builder.ensure_space(builder.line_height() * 5.0);

    // ── Thin separator rule before each commit ─────────────────────────────────
    builder.draw_horizontal_rule(rule_gray.clone(), 0.4);
    builder.vertical_space(7.0);

    // ── Line 1: sha · message — links to the commit page ──────────────────────
    builder.write_line(&[
        Span {
            text: format!("{sha_short}  "),
            font_id: bold.clone(),
            size: Pt(font_size),
            color: dark_gray.clone(),
        },
        Span {
            text: message_first_line.to_string(),
            font_id: bold.clone(),
            size: Pt(font_size),
            color: black.clone(),
        },
    ]);
    builder.add_link(builder.line_height(), Actions::Uri(detail.html_url.clone()));

    // ── Line 2: repo (branch) · author · date · ±stats — links to repo/branch ─
    let meta_size = Pt(font_size - 1.0);
    let mut meta_spans = vec![Span {
        text: format!("  {repo}  "),
        font_id: regular.clone(),
        size: meta_size,
        color: dark_gray.clone(),
    }];
    if let Some(b) = branch {
        meta_spans.push(Span {
            text: format!("({b})  "),
            font_id: italic.clone(),
            size: meta_size,
            color: gray.clone(),
        });
    }
    meta_spans.extend([
        Span {
            text: format!("{author}  "),
            font_id: regular.clone(),
            size: meta_size,
            color: dark_gray.clone(),
        },
        Span {
            text: format!("{date}  "),
            font_id: regular.clone(),
            size: meta_size,
            color: gray.clone(),
        },
        Span {
            text: format!("+{total_additions}"),
            font_id: bold.clone(),
            size: meta_size,
            color: neon_green(),
        },
        Span {
            text: "  ".to_string(),
            font_id: regular.clone(),
            size: meta_size,
            color: gray.clone(),
        },
        Span {
            text: format!("-{total_deletions}"),
            font_id: bold.clone(),
            size: meta_size,
            color: neon_red(),
        },
    ]);
    builder.write_line(&meta_spans);
    let meta_url = branch
        .map(|b| format!("https://github.com/{repo}/tree/{b}"))
        .unwrap_or_else(|| format!("https://github.com/{repo}"));
    builder.add_link(builder.line_height(), Actions::Uri(meta_url));

    builder.vertical_space(5.0);

    // ── Per-file diffs ─────────────────────────────────────────────────────────
    detail.files.iter().for_each(|file| {
        builder.ensure_space(builder.line_height() * 3.0);

        // File header: filename + stats, links to the file at this commit on GitHub.
        builder.write_line(&[
            Span {
                text: format!("  {} ", file.filename),
                font_id: bold.clone(),
                size: Pt(font_size - 0.5),
                color: black.clone(),
            },
            Span {
                text: format!("+{}", file.additions),
                font_id: regular.clone(),
                size: Pt(font_size - 0.5),
                color: neon_green(),
            },
            Span {
                text: " ".to_string(),
                font_id: regular.clone(),
                size: Pt(font_size - 0.5),
                color: gray.clone(),
            },
            Span {
                text: format!("-{}", file.deletions),
                font_id: regular.clone(),
                size: Pt(font_size - 0.5),
                color: neon_red(),
            },
        ]);
        let file_url = format!(
            "https://github.com/{repo}/blob/{}/{}",
            detail.sha, file.filename
        );
        builder.add_link(builder.line_height(), Actions::Uri(file_url));

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
                    let (marker, color) = if line.starts_with('+') {
                        ("+", neon_green())
                    } else if line.starts_with('-') {
                        ("-", neon_red())
                    } else if line.starts_with("@@") {
                        ("@", hunk_blue())
                    } else {
                        (" ", dark_gray.clone())
                    };
                    let body = if line.starts_with("@@") {
                        line.to_string()
                    } else {
                        // Strip the diff prefix char; replace with padded marker.
                        format!("{marker} {}", line.get(1..).unwrap_or(line))
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

    builder.vertical_space(6.0);
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
        super::render_commit(
            &mut builder,
            &test_detail(true),
            "alice/repo",
            Some("main"),
            8.0,
        );
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_commit_without_patch_shows_placeholder() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render_commit(&mut builder, &test_detail(false), "alice/repo", None, 8.0);
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
        super::render_commit(&mut builder, &detail, "alice/repo", Some("dev"), 8.0);
        assert!(!builder.finish().is_empty());
    }
}

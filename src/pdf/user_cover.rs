use printpdf::{Actions, Color, Pt, Rgb};

use super::layout::{PageBuilder, Span};
use crate::github::GitHubUser;

const CRATES_URL: &str = "https://crates.io/crates/gitprint";
const LABEL_COL: usize = 14;
const CHAR_WIDTH: f32 = 0.6;

fn separator_line(width_pt: f32, font_size: f32) -> String {
    let chars = (width_pt / (font_size * CHAR_WIDTH)).max(1.0) as usize;
    "─".repeat(chars)
}

/// Word-wrap `text` into lines of at most `max_chars` characters, breaking at word boundaries.
fn word_wrap(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return vec![text.to_string()];
    }
    let (mut lines, last) = text.split_whitespace().fold(
        (Vec::<String>::new(), String::new()),
        |(mut lines, mut cur), word| {
            if !cur.is_empty() && cur.len() + 1 + word.len() > max_chars {
                lines.push(std::mem::take(&mut cur));
            } else if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
            (lines, cur)
        },
    );
    if !last.is_empty() {
        lines.push(last);
    }
    lines
}

pub fn render(builder: &mut PageBuilder, user: &GitHubUser, total_stars: u64) {
    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
    let gray = Color::Rgb(Rgb::new(0.47, 0.47, 0.47, None));
    let lh = builder.line_height();

    const TABLE_SIZE: f32 = 9.0;
    const SEP_SIZE: f32 = 7.5;

    let display_name = user.name.as_deref().unwrap_or(&user.login);

    // ── Title ──────────────────────────────────────────────────────────────────
    builder.vertical_space(120.0);
    builder.write_centered(display_name, &bold, Pt(28.0), black.clone());
    builder.add_link(28.0 + 4.0, Actions::Uri(user.html_url.clone()));

    if display_name != user.login {
        builder.vertical_space(6.0);
        builder.write_centered(
            &format!("@{}", user.login),
            &regular,
            Pt(12.0),
            gray.clone(),
        );
        builder.add_link(12.0 + 4.0, Actions::Uri(user.html_url.clone()));
    }

    builder.vertical_space(32.0);

    // ── Metadata table ─────────────────────────────────────────────────────────
    let sep = separator_line(builder.usable_width_pt(), SEP_SIZE);
    let sep_span = || Span {
        text: sep.clone(),
        font_id: regular.clone(),
        size: Pt(SEP_SIZE),
        color: gray.clone(),
    };

    builder.write_line(&[sep_span()]);
    builder.vertical_space(4.0);

    // Build rows dynamically — only show non-empty fields.
    let repos_str = user.public_repos.to_string();
    let stars_str = total_stars.to_string();
    let followers_str = user.followers.to_string();
    let following_str = user.following.to_string();
    let member_since = user
        .created_at
        .get(..10)
        .unwrap_or(&user.created_at)
        .to_string();

    let value_col_max_chars = ((builder.usable_width_pt()
        - LABEL_COL as f32 * TABLE_SIZE * CHAR_WIDTH)
        / (TABLE_SIZE * CHAR_WIDTH))
        .max(1.0) as usize;

    [
        ("Bio", user.bio.as_deref().unwrap_or(""), None::<String>),
        ("Location", user.location.as_deref().unwrap_or(""), None),
        ("Company", user.company.as_deref().unwrap_or(""), None),
        (
            "Blog",
            user.blog.as_deref().unwrap_or(""),
            user.blog.as_ref().map(|b| {
                if b.starts_with("http") {
                    b.clone()
                } else {
                    format!("https://{b}")
                }
            }),
        ),
        ("Email", user.email.as_deref().unwrap_or(""), None),
        ("Public Repos", &repos_str, None),
        ("Total Stars", &stars_str, None),
        ("Followers", &followers_str, None),
        ("Following", &following_str, None),
        ("Member Since", &member_since, None),
        ("Profile", &user.html_url, Some(user.html_url.clone())),
    ]
    .into_iter()
    .filter(|(_, value, _)| !value.is_empty())
    .for_each(|(label, value, url)| {
        word_wrap(value, value_col_max_chars)
            .into_iter()
            .enumerate()
            .for_each(|(i, line)| {
                let label_text = if i == 0 {
                    format!("{label:<LABEL_COL$}")
                } else {
                    " ".repeat(LABEL_COL)
                };
                builder.write_line(&[
                    Span {
                        text: label_text,
                        font_id: bold.clone(),
                        size: Pt(TABLE_SIZE),
                        color: black.clone(),
                    },
                    Span {
                        text: line,
                        font_id: regular.clone(),
                        size: Pt(TABLE_SIZE),
                        color: black.clone(),
                    },
                ]);
            });
        if let Some(u) = url {
            builder.add_link(lh, Actions::Uri(u));
        }
    });

    builder.vertical_space(4.0);
    builder.write_line(&[sep_span()]);

    // ── Footer ─────────────────────────────────────────────────────────────────
    let version = env!("CARGO_PKG_VERSION");
    let footer_text =
        format!("Generated with gitprint v{version} ({CRATES_URL}), a Izel Nakri production");
    let footer_size = Pt(7.0);
    let footer_area = lh + 4.0 + footer_size.0 + 4.0;
    builder.vertical_space((builder.remaining_pt() - footer_area).max(0.0));

    builder.write_line(&[Span {
        text: separator_line(builder.usable_width_pt(), footer_size.0),
        font_id: regular.clone(),
        size: footer_size,
        color: gray.clone(),
    }]);
    builder.vertical_space(4.0);
    builder.write_centered(&footer_text, &regular, footer_size, gray);
    builder.add_link(footer_size.0 + 4.0, Actions::Uri(CRATES_URL.to_string()));

    builder.page_break();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf;
    use crate::types::{Config, UserReportConfig};

    fn test_user() -> GitHubUser {
        GitHubUser {
            login: "alice".to_string(),
            name: Some("Alice Dev".to_string()),
            bio: Some("Rust enthusiast".to_string()),
            location: Some("Berlin".to_string()),
            company: Some("Acme Corp".to_string()),
            blog: Some("https://alice.dev".to_string()),
            email: Some("alice@example.com".to_string()),
            public_repos: 42,
            followers: 100,
            following: 50,
            created_at: "2018-03-15T10:00:00Z".to_string(),
            html_url: "https://github.com/alice".to_string(),
        }
    }

    fn test_user_config() -> UserReportConfig {
        UserReportConfig {
            username: "alice".to_string(),
            output_path: "/tmp/alice.pdf".into(),
            paper_size: crate::types::PaperSize::A4,
            landscape: false,
            last_committed: 5,
            commits: 5,
            no_diffs: false,
            font_size: 8.0,
            github_token: None,
            since: None,
            until: None,
            activity: crate::types::ActivityFilter::All,
            events: 30,
        }
    }

    #[test]
    fn render_user_cover_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render(&mut builder, &test_user(), 1337);
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_user_cover_no_name() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let mut user = test_user();
        user.name = None;
        super::render(&mut builder, &user, 0);
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_user_cover_minimal_fields() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let uc = test_user_config();
        let mut builder = pdf::create_user_builder(&uc, fonts);
        let user = GitHubUser {
            login: "bob".to_string(),
            name: None,
            bio: None,
            location: None,
            company: None,
            blog: None,
            email: None,
            public_repos: 1,
            followers: 0,
            following: 0,
            created_at: "2020-01-01T00:00:00Z".to_string(),
            html_url: "https://github.com/bob".to_string(),
        };
        super::render(&mut builder, &user, 0);
        assert!(!builder.finish().is_empty());
    }
}

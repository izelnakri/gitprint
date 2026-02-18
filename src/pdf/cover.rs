use std::path::Path;

use printpdf::{Actions, Color, Pt, Rgb};

use super::layout::{PageBuilder, Span};
use crate::types::RepoMetadata;

const CRATES_URL: &str = "https://crates.io/crates/gitprint";
/// Label column width in characters (monospace font — spaces give exact alignment).
const LABEL_COL: usize = 12;
/// Approximate character-width-to-font-size ratio for JetBrains Mono.
const CHAR_WIDTH: f32 = 0.6;

// ── Pure URL-building helpers (also tested independently below) ────────────────

/// Extracts a GitHub username from a noreply email address.
///
/// Handles both `123456+username@users.noreply.github.com` and
/// `username@users.noreply.github.com` formats.
fn github_username_from_email(email: &str) -> Option<&str> {
    let local = email.strip_suffix("@users.noreply.github.com")?;
    Some(local.split('+').next_back().unwrap_or(local))
}

/// Returns the URL for a specific commit on the remote.
fn commit_link(remote_base: &str, commit_hash: &str) -> String {
    format!("{remote_base}/commit/{commit_hash}")
}

/// Returns a link to the repo tree at the given commit, or the repo root if no commit.
fn repo_tree_link(remote_base: &str, commit_hash: &str) -> String {
    if commit_hash.is_empty() {
        remote_base.to_string()
    } else {
        format!("{remote_base}/tree/{commit_hash}")
    }
}

/// Returns an author profile/search link for the given email on the remote.
///
/// When the email is a GitHub noreply address the username is extracted and a
/// profile URL (`https://github.com/{user}`) is returned. Otherwise a
/// commit-search-by-email URL is used.
fn author_link(remote_base: &str, email: &str) -> String {
    if let Some(username) = github_username_from_email(email) {
        let host = remote_base
            .splitn(4, '/')
            .take(3)
            .collect::<Vec<_>>()
            .join("/");
        format!("{host}/{username}")
    } else {
        format!("{remote_base}/commits?author={email}")
    }
}

/// Returns a `file://` URL for a local filesystem path.
fn file_url(path: &Path) -> String {
    format!("file://{}", path.display())
}

/// Returns a horizontal rule string that fills `width_pt` at the given `font_size`.
fn separator_line(width_pt: f32, font_size: f32) -> String {
    let chars = (width_pt / (font_size * CHAR_WIDTH)).max(1.0) as usize;
    "─".repeat(chars)
}

// ── Renderer ──────────────────────────────────────────────────────────────────

pub fn render(builder: &mut PageBuilder, metadata: &RepoMetadata, remote_url: Option<&str>) {
    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
    let gray = Color::Rgb(Rgb::new(0.47, 0.47, 0.47, None));
    let lh = builder.line_height();

    const TABLE_SIZE: f32 = 9.0;
    const SEP_SIZE: f32 = 7.5;

    // Use explicit remote_url if provided; otherwise fall back to the one detected
    // from git config so links work for local git repos without --remote.
    let effective_remote = remote_url.or(metadata.detected_remote_url.as_deref());
    let remote_base = effective_remote.map(|u| u.trim_end_matches(".git"));

    // Title links to repo tree at current commit (remote) or to the local path (local).
    let title_url: Option<String> = remote_base
        .map(|base| repo_tree_link(base, &metadata.commit_hash))
        .or_else(|| metadata.repo_absolute_path.as_deref().map(file_url));

    let commit_url = remote_base
        .filter(|_| !metadata.commit_hash.is_empty())
        .map(|base| commit_link(base, &metadata.commit_hash));

    let author_url = remote_base
        .filter(|_| !metadata.commit_author_email.is_empty())
        .map(|base| author_link(base, &metadata.commit_author_email));

    let author_display = if metadata.commit_author_email.is_empty() {
        metadata.commit_author.clone()
    } else {
        format!(
            "{} <{}>",
            metadata.commit_author, metadata.commit_author_email
        )
    };

    // ── Title ─────────────────────────────────────────────────────────────────
    builder.vertical_space(120.0);
    builder.write_centered(&metadata.name, &bold, Pt(28.0), black.clone());
    if let Some(url) = title_url {
        builder.add_link(28.0 + 4.0, Actions::Uri(url));
    }
    builder.vertical_space(32.0);

    // ── Metadata table ────────────────────────────────────────────────────────
    let sep = separator_line(builder.usable_width_pt(), SEP_SIZE);

    let sep_span = || Span {
        text: sep.clone(),
        font_id: regular.clone(),
        size: Pt(SEP_SIZE),
        color: gray.clone(),
    };

    builder.write_line(&[sep_span()]);
    builder.vertical_space(4.0);

    // Rows: (label, value, optional URL). Message links to the same commit as Commit.
    [
        ("Branch", metadata.branch.as_str(), None::<String>),
        (
            "Commit",
            metadata.commit_hash_short.as_str(),
            commit_url.clone(),
        ),
        ("Author", author_display.as_str(), author_url),
        ("Date", metadata.commit_date.as_str(), None),
        (
            "Message",
            metadata.commit_message.as_str(),
            commit_url.clone(),
        ),
        ("Files", &metadata.file_count.to_string(), None),
        ("Lines", &metadata.total_lines.to_string(), None),
        ("Size", metadata.repo_size.as_str(), None),
        ("FS Owner", metadata.fs_owner.as_deref().unwrap_or(""), None),
        ("FS Group", metadata.fs_group.as_deref().unwrap_or(""), None),
        ("Generated", metadata.generated_at.as_str(), None),
    ]
    .into_iter()
    .filter(|(_, value, _)| !value.is_empty())
    .for_each(|(label, value, url)| {
        builder.write_line(&[
            Span {
                text: format!("{label:<LABEL_COL$}"),
                font_id: bold.clone(),
                size: Pt(TABLE_SIZE),
                color: black.clone(),
            },
            Span {
                text: value.into(),
                font_id: regular.clone(),
                size: Pt(TABLE_SIZE),
                color: black.clone(),
            },
        ]);
        if let Some(u) = url {
            builder.add_link(lh, Actions::Uri(u));
        }
    });

    builder.vertical_space(4.0);
    builder.write_line(&[sep_span()]);

    // ── Footer (pushed to the bottom of the page) ─────────────────────────────
    let version = env!("CARGO_PKG_VERSION");
    let footer_text =
        format!("Generated with gitprint v{version} ({CRATES_URL}), a Izel Nakri production");
    let footer_size = Pt(7.0);
    // footer area = separator line (lh) + 4pt gap + footer text (size + 4)
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
    use std::path::PathBuf;

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
            commit_author: "Alice Dev".into(),
            commit_author_email: "alice@example.com".into(),
            file_count: 5,
            total_lines: 100,
            fs_owner: Some("alice".into()),
            fs_group: Some("staff".into()),
            generated_at: "2024-01-15 10:00:00 UTC".into(),
            repo_size: "1.2 MB".into(),
            detected_remote_url: None,
            repo_absolute_path: None,
        }
    }

    // ── URL-building helpers ───────────────────────────────────────────────────

    #[test]
    fn commit_link_https() {
        assert_eq!(
            super::commit_link("https://github.com/user/repo", "abc123"),
            "https://github.com/user/repo/commit/abc123"
        );
    }

    #[test]
    fn repo_tree_link_with_commit() {
        assert_eq!(
            super::repo_tree_link("https://github.com/user/repo", "abc123"),
            "https://github.com/user/repo/tree/abc123"
        );
    }

    #[test]
    fn repo_tree_link_without_commit_returns_base() {
        assert_eq!(
            super::repo_tree_link("https://github.com/user/repo", ""),
            "https://github.com/user/repo"
        );
    }

    #[test]
    fn author_link_noreply_with_numeric_prefix() {
        assert_eq!(
            super::author_link(
                "https://github.com/user/repo",
                "123456+alice@users.noreply.github.com"
            ),
            "https://github.com/alice"
        );
    }

    #[test]
    fn author_link_noreply_without_numeric_prefix() {
        assert_eq!(
            super::author_link(
                "https://github.com/user/repo",
                "alice@users.noreply.github.com"
            ),
            "https://github.com/alice"
        );
    }

    #[test]
    fn author_link_regular_email_falls_back_to_search() {
        assert_eq!(
            super::author_link("https://github.com/user/repo", "alice@example.com"),
            "https://github.com/user/repo/commits?author=alice@example.com"
        );
    }

    #[test]
    fn file_url_absolute_path() {
        assert_eq!(
            super::file_url(std::path::Path::new("/home/user/project")),
            "file:///home/user/project"
        );
    }

    #[test]
    fn github_username_from_noreply_email() {
        assert_eq!(
            super::github_username_from_email("123456+alice@users.noreply.github.com"),
            Some("alice")
        );
        assert_eq!(
            super::github_username_from_email("alice@users.noreply.github.com"),
            Some("alice")
        );
        assert_eq!(super::github_username_from_email("alice@example.com"), None);
    }

    #[test]
    fn separator_line_fills_width() {
        // At 7.5pt with 0.6 ratio, each char ≈ 4.5pt wide.
        let chars = super::separator_line(45.0, 7.5).chars().count();
        assert_eq!(chars, 10);
    }

    // ── render() smoke tests ───────────────────────────────────────────────────

    #[test]
    fn render_cover_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render(&mut builder, &test_metadata(), None);
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_cover_with_remote_url() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render(
            &mut builder,
            &test_metadata(),
            Some("https://github.com/user/repo"),
        );
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_cover_with_detected_remote_url() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let mut meta = test_metadata();
        meta.detected_remote_url = Some("https://github.com/user/local-repo".into());
        super::render(&mut builder, &meta, None);
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_cover_with_local_path_file_url() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let mut meta = test_metadata();
        meta.repo_absolute_path = Some(PathBuf::from("/home/user/myproject"));
        super::render(&mut builder, &meta, None);
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_cover_remote_takes_precedence_over_local_path() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let mut meta = test_metadata();
        meta.repo_absolute_path = Some(PathBuf::from("/home/user/myproject"));
        super::render(&mut builder, &meta, Some("https://github.com/user/repo"));
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
                commit_author: String::new(),
                commit_author_email: String::new(),
                file_count: 0,
                total_lines: 0,
                fs_owner: None,
                fs_group: None,
                generated_at: String::new(),
                repo_size: String::new(),
                detected_remote_url: None,
                repo_absolute_path: None,
            },
            None,
        );
    }

    #[test]
    fn render_cover_with_commit_message_is_linked() {
        // Smoke test: cover with remote must not panic with commit_url on message row.
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render(
            &mut builder,
            &test_metadata(),
            Some("https://github.com/user/repo.git"),
        );
        assert!(!builder.finish().is_empty());
    }
}

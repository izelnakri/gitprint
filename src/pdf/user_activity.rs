use printpdf::{Actions, Color, Pt, Rgb};

use super::layout::{PageBuilder, Span};
use crate::github::GitHubEvent;

pub fn render(
    builder: &mut PageBuilder,
    events: &[GitHubEvent],
    commit_msgs: &std::collections::HashMap<String, String>,
) {
    if events.is_empty() {
        return;
    }

    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let italic = builder.font(false, true).clone();
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
    let gray = Color::Rgb(Rgb::new(0.50, 0.50, 0.50, None));
    let dark_gray = Color::Rgb(Rgb::new(0.25, 0.25, 0.25, None));
    let rule_gray = Color::Rgb(Rgb::new(0.82, 0.82, 0.82, None));

    // ── Section title ──────────────────────────────────────────────────────────
    builder.ensure_space(builder.line_height() * 3.0);
    builder.write_centered("Recent Activity", &bold, Pt(16.0), black.clone());
    builder.vertical_space(10.0);
    builder.draw_horizontal_rule(rule_gray.clone(), 0.5);
    builder.vertical_space(8.0);

    // ── Events grouped by date ─────────────────────────────────────────────────
    let mut last_date = String::new();
    events.iter().for_each(|event| {
        let date = event.created_at.get(..10).unwrap_or(&event.created_at);
        if date != last_date {
            if !last_date.is_empty() {
                // Thin rule between date groups for visual separation.
                builder.vertical_space(4.0);
                builder.draw_horizontal_rule(rule_gray.clone(), 0.3);
                builder.vertical_space(8.0);
            } else {
                builder.vertical_space(2.0);
            }
            builder.ensure_space(builder.line_height() * 2.0);
            builder.write_line(&[Span {
                text: date.to_string(),
                font_id: bold.clone(),
                size: Pt(9.5),
                color: dark_gray.clone(),
            }]);
            last_date = date.to_string();
            builder.vertical_space(2.0);
        }

        let time = event.created_at.get(11..16).unwrap_or("");
        let description = describe_event(event);
        let icon = event_icon(event);

        // Enrich push events that have no commit info in the payload (force push /
        // rebase). Look up this event's HEAD SHA to get its specific commit message.
        let (main, detail) = if event.kind == "PushEvent" && description.detail.is_empty() {
            let sha = event.payload["head"].as_str().unwrap_or("");
            if let Some(msg) = commit_msgs.get(sha) {
                let branch = event.payload["ref"]
                    .as_str()
                    .unwrap_or("")
                    .trim_start_matches("refs/heads/");
                let enriched_main = format!("Pushed to {} ({branch})", event.repo.name);
                (enriched_main, vec![format!("  {msg}")])
            } else {
                (description.main, description.detail)
            }
        } else {
            (description.main, description.detail)
        };

        let url = event_url(event);
        builder.write_line(&[
            Span {
                text: format!("{icon} "),
                font_id: bold.clone(),
                size: Pt(8.0),
                color: event_icon_color(event),
            },
            Span {
                text: format!("{time}  "),
                font_id: regular.clone(),
                size: Pt(7.5),
                color: gray.clone(),
            },
            Span {
                text: main,
                font_id: regular.clone(),
                size: Pt(8.5),
                color: black.clone(),
            },
        ]);
        if let Some(u) = &url {
            builder.add_link(builder.line_height(), Actions::Uri(u.clone()));
        }

        // Detail lines (commit messages, PR diff stats, etc.) — also link to the event.
        detail.iter().for_each(|detail_line| {
            builder.write_line(&[
                Span {
                    text: "    ".to_string(),
                    font_id: regular.clone(),
                    size: Pt(7.5),
                    color: gray.clone(),
                },
                Span {
                    text: detail_line.clone(),
                    font_id: italic.clone(),
                    size: Pt(7.5),
                    color: gray.clone(),
                },
            ]);
            if let Some(u) = &url {
                builder.add_link(builder.line_height(), Actions::Uri(u.clone()));
            }
        });
    });

    builder.vertical_space(12.0);
    builder.page_break();
}

// ── Event decorators ────────────────────────────────────────────────────────────

/// Single-character icon using Geometric Shapes (U+25A0–U+25FF) — all present
/// in JetBrains Mono and rendered reliably across PDF viewers including PDF.js.
fn event_icon(event: &GitHubEvent) -> &'static str {
    match event.kind.as_str() {
        "PushEvent" => "\u{25B6}",         // ▶ black right-pointing triangle
        "PullRequestEvent" => "\u{25B2}",  // ▲ black up-pointing triangle
        "IssuesEvent" => "\u{25CF}",       // ● black circle
        "IssueCommentEvent" => "\u{25E6}", // ◦ white bullet
        "PullRequestReviewEvent" | "PullRequestReviewCommentEvent" => "\u{25CB}", // ○ white circle
        "ReleaseEvent" => "\u{25C6}",      // ◆ black diamond
        "ForkEvent" => "\u{25B7}",         // ▷ white right-pointing triangle
        "WatchEvent" => "\u{25C6}",        // ◆ black diamond (star/highlight)
        "CreateEvent" => "\u{25AA}",       // ▪ black small square
        "DeleteEvent" => "\u{25AB}",       // ▫ white small square
        _ => "\u{00B7}",                   // · middle dot
    }
}

fn event_icon_color(event: &GitHubEvent) -> Color {
    match event.kind.as_str() {
        "PushEvent" => Color::Rgb(Rgb::new(0.27, 0.68, 0.96, None)), // blue
        "PullRequestEvent" => Color::Rgb(Rgb::new(0.55, 0.36, 0.90, None)), // purple
        "IssuesEvent" => Color::Rgb(Rgb::new(0.96, 0.55, 0.13, None)), // orange
        "IssueCommentEvent" | "PullRequestReviewEvent" | "PullRequestReviewCommentEvent" => {
            Color::Rgb(Rgb::new(0.50, 0.50, 0.50, None)) // gray
        }
        "ReleaseEvent" => Color::Rgb(Rgb::new(0.13, 0.78, 0.47, None)), // green
        "WatchEvent" => Color::Rgb(Rgb::new(0.96, 0.80, 0.10, None)),   // gold
        "ForkEvent" => Color::Rgb(Rgb::new(0.27, 0.68, 0.96, None)),    // blue
        "CreateEvent" => Color::Rgb(Rgb::new(0.13, 0.78, 0.47, None)),  // green
        "DeleteEvent" => Color::Rgb(Rgb::new(0.78, 0.25, 0.25, None)),  // red
        _ => Color::Rgb(Rgb::new(0.50, 0.50, 0.50, None)),
    }
}

struct EventDescription {
    main: String,
    detail: Vec<String>,
}

fn describe_event(event: &GitHubEvent) -> EventDescription {
    let repo = &event.repo.name;
    let p = &event.payload;

    let (main, detail) = match event.kind.as_str() {
        "PushEvent" => {
            let branch = p["ref"]
                .as_str()
                .unwrap_or("")
                .trim_start_matches("refs/heads/");
            let commits_arr = p["commits"].as_array();
            let count = p["size"]
                .as_u64()
                .map(|n| n as usize)
                .filter(|&n| n > 0)
                .unwrap_or_else(|| commits_arr.map(|c| c.len()).unwrap_or(0));
            let main = if count > 0 {
                let label = if count == 1 { "commit" } else { "commits" };
                format!("Pushed {count} {label} to {repo} ({branch})")
            } else {
                format!("Pushed to {repo} ({branch})")
            };
            let detail: Vec<String> = commits_arr
                .into_iter()
                .flatten()
                .take(5)
                .filter_map(|c| c["message"].as_str())
                .map(|m| format!("  {}", m.lines().next().unwrap_or(m)))
                .collect();
            (main, detail)
        }
        "PullRequestEvent" => {
            let action = p["action"].as_str().unwrap_or("updated");
            let merged =
                action == "closed" && p["pull_request"]["merged"].as_bool().unwrap_or(false);
            let label = if merged { "merged" } else { action };
            let title = p["pull_request"]["title"].as_str().unwrap_or("");
            let number = p["pull_request"]["number"].as_u64().unwrap_or(0);
            let detail = match (
                p["pull_request"]["additions"].as_u64(),
                p["pull_request"]["deletions"].as_u64(),
                p["pull_request"]["changed_files"].as_u64(),
            ) {
                (Some(a), Some(d), Some(f)) => {
                    let fword = if f == 1 { "file" } else { "files" };
                    vec![format!("    +{a} \u{2212}{d} across {f} {fword}")]
                }
                _ => vec![],
            };
            (
                format!("{} PR #{number}: {title} in {repo}", capitalise(label)),
                detail,
            )
        }
        "IssuesEvent" => {
            let action = p["action"].as_str().unwrap_or("updated");
            let title = p["issue"]["title"].as_str().unwrap_or("");
            let number = p["issue"]["number"].as_u64().unwrap_or(0);
            (
                format!("{} issue #{number}: {title} in {repo}", capitalise(action)),
                vec![],
            )
        }
        "IssueCommentEvent" => {
            let title = p["issue"]["title"].as_str().unwrap_or("");
            let number = p["issue"]["number"].as_u64().unwrap_or(0);
            (
                format!("Commented on issue #{number}: {title} in {repo}"),
                vec![],
            )
        }
        "PullRequestReviewEvent" => {
            let state = p["review"]["state"].as_str().unwrap_or("reviewed");
            let number = p["pull_request"]["number"].as_u64().unwrap_or(0);
            (
                format!("{} PR #{number} in {repo}", capitalise(state)),
                vec![],
            )
        }
        "PullRequestReviewCommentEvent" => {
            let number = p["pull_request"]["number"].as_u64().unwrap_or(0);
            (format!("Reviewed PR #{number} in {repo}"), vec![])
        }
        "CreateEvent" => {
            let ref_type = p["ref_type"].as_str().unwrap_or("ref");
            let ref_name = p["ref"].as_str().unwrap_or("");
            if ref_name.is_empty() {
                (format!("Created {ref_type} {repo}"), vec![])
            } else {
                (format!("Created {ref_type} '{ref_name}' in {repo}"), vec![])
            }
        }
        "DeleteEvent" => {
            let ref_type = p["ref_type"].as_str().unwrap_or("ref");
            let ref_name = p["ref"].as_str().unwrap_or("");
            (format!("Deleted {ref_type} '{ref_name}' in {repo}"), vec![])
        }
        "ForkEvent" => {
            let forkee = p["forkee"]["full_name"].as_str().unwrap_or(repo);
            (format!("Forked {repo} \u{2192} {forkee}"), vec![])
        }
        "WatchEvent" => (format!("Starred {repo}"), vec![]),
        "ReleaseEvent" => {
            let action = p["action"].as_str().unwrap_or("published");
            let tag = p["release"]["tag_name"].as_str().unwrap_or("");
            (
                format!("{} release {tag} in {repo}", capitalise(action)),
                vec![],
            )
        }
        "CommitCommentEvent" => (format!("Commented on a commit in {repo}"), vec![]),
        "GollumEvent" => (format!("Updated wiki in {repo}"), vec![]),
        "MemberEvent" => {
            let action = p["action"].as_str().unwrap_or("updated");
            let member = p["member"]["login"].as_str().unwrap_or("someone");
            (
                format!("{} {member} as collaborator in {repo}", capitalise(action)),
                vec![],
            )
        }
        "PublicEvent" => (format!("Made {repo} public"), vec![]),
        "SponsorshipEvent" => (format!("Sponsorship activity in {repo}"), vec![]),
        other => (format!("{other} in {repo}"), vec![]),
    };

    EventDescription { main, detail }
}

/// Returns the most relevant clickable URL for a GitHub event, if one exists.
fn event_url(event: &GitHubEvent) -> Option<String> {
    let repo = &event.repo.name;
    let p = &event.payload;
    match event.kind.as_str() {
        "PushEvent" => {
            if let Some(sha) = p["head"].as_str() {
                Some(format!("https://github.com/{repo}/commit/{sha}"))
            } else {
                p["ref"].as_str().map(|r| {
                    let branch = r.trim_start_matches("refs/heads/");
                    format!("https://github.com/{repo}/tree/{branch}")
                })
            }
        }
        "PullRequestEvent" => p["pull_request"]["html_url"].as_str().map(str::to_string),
        "IssuesEvent" => p["issue"]["html_url"].as_str().map(str::to_string),
        "IssueCommentEvent" => p["comment"]["html_url"].as_str().map(str::to_string),
        "PullRequestReviewEvent" | "PullRequestReviewCommentEvent" => {
            p["pull_request"]["html_url"].as_str().map(str::to_string)
        }
        "ForkEvent" => p["forkee"]["html_url"].as_str().map(str::to_string),
        "ReleaseEvent" => p["release"]["html_url"].as_str().map(str::to_string),
        _ => Some(format!("https://github.com/{repo}")),
    }
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf;
    use crate::types::Config;

    fn push_event() -> GitHubEvent {
        GitHubEvent {
            kind: "PushEvent".to_string(),
            repo: crate::github::EventRepo {
                name: "alice/myrepo".to_string(),
            },
            payload: serde_json::json!({
                "ref": "refs/heads/main",
                "commits": [
                    { "message": "fix: correct typo" },
                    { "message": "feat: add feature" }
                ]
            }),
            created_at: "2024-03-01T12:00:00Z".to_string(),
        }
    }

    fn pr_event() -> GitHubEvent {
        GitHubEvent {
            kind: "PullRequestEvent".to_string(),
            repo: crate::github::EventRepo {
                name: "alice/myrepo".to_string(),
            },
            payload: serde_json::json!({
                "action": "opened",
                "pull_request": { "number": 42, "title": "Add dark mode" }
            }),
            created_at: "2024-03-01T11:00:00Z".to_string(),
        }
    }

    #[test]
    fn render_activity_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render(
            &mut builder,
            &[push_event(), pr_event()],
            &std::collections::HashMap::new(),
        );
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_activity_empty_is_noop() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let page_before = builder.current_page();
        super::render(&mut builder, &[], &std::collections::HashMap::new());
        assert_eq!(builder.current_page(), page_before);
    }

    #[test]
    fn capitalise_works() {
        assert_eq!(super::capitalise("opened"), "Opened");
        assert_eq!(super::capitalise(""), "");
        assert_eq!(super::capitalise("x"), "X");
    }

    #[test]
    fn describe_push_event() {
        let desc = super::describe_event(&push_event());
        assert!(desc.main.contains("Pushed 2 commits"));
        assert!(desc.main.contains("alice/myrepo"));
        assert!(desc.main.contains("main"));
        assert_eq!(desc.detail.len(), 2);
    }

    #[test]
    fn describe_pr_event() {
        let desc = super::describe_event(&pr_event());
        assert!(desc.main.contains("Opened PR #42"));
        assert!(desc.main.contains("Add dark mode"));
    }

    #[test]
    fn event_icons_cover_all_known_types() {
        [
            "PushEvent",
            "PullRequestEvent",
            "IssuesEvent",
            "ReleaseEvent",
            "WatchEvent",
            "IssueCommentEvent",
            "PullRequestReviewEvent",
            "PullRequestReviewCommentEvent",
            "ForkEvent",
            "CreateEvent",
            "DeleteEvent",
            "UnknownEvent",
        ]
        .iter()
        .for_each(|kind| {
            let e = GitHubEvent {
                kind: kind.to_string(),
                repo: crate::github::EventRepo {
                    name: "a/b".to_string(),
                },
                payload: serde_json::json!({}),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            };
            assert!(!super::event_icon(&e).is_empty());
            // icon color must not panic
            let _ = super::event_icon_color(&e);
        });
    }

    fn make_event(kind: &str, payload: serde_json::Value) -> GitHubEvent {
        GitHubEvent {
            kind: kind.to_string(),
            repo: crate::github::EventRepo {
                name: "alice/repo".to_string(),
            },
            payload,
            created_at: "2024-03-01T09:30:00Z".to_string(),
        }
    }

    #[test]
    fn describe_issue_comment_event() {
        let e = make_event(
            "IssueCommentEvent",
            serde_json::json!({ "issue": { "number": 7, "title": "Bug" } }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("#7"));
        assert!(d.main.contains("Bug"));
    }

    #[test]
    fn describe_pr_review_event() {
        let e = make_event(
            "PullRequestReviewEvent",
            serde_json::json!({ "review": { "state": "approved" }, "pull_request": { "number": 3 } }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("Approved"));
        assert!(d.main.contains("#3"));
    }

    #[test]
    fn describe_pr_review_comment_event() {
        let e = make_event(
            "PullRequestReviewCommentEvent",
            serde_json::json!({ "pull_request": { "number": 5 } }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("#5"));
    }

    #[test]
    fn describe_create_event_with_ref() {
        let e = make_event(
            "CreateEvent",
            serde_json::json!({ "ref_type": "branch", "ref": "feature/x" }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("branch"));
        assert!(d.main.contains("feature/x"));
    }

    #[test]
    fn describe_create_event_no_ref() {
        let e = make_event(
            "CreateEvent",
            serde_json::json!({ "ref_type": "repository", "ref": "" }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("repository"));
        assert!(!d.main.contains("''"));
    }

    #[test]
    fn describe_delete_event() {
        let e = make_event(
            "DeleteEvent",
            serde_json::json!({ "ref_type": "branch", "ref": "old-feature" }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("old-feature"));
    }

    #[test]
    fn describe_fork_event() {
        let e = make_event(
            "ForkEvent",
            serde_json::json!({ "forkee": { "full_name": "bob/repo" } }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("bob/repo"));
    }

    #[test]
    fn describe_watch_event() {
        let d = super::describe_event(&make_event("WatchEvent", serde_json::json!({})));
        assert!(d.main.contains("Starred"));
    }

    #[test]
    fn describe_release_event() {
        let e = make_event(
            "ReleaseEvent",
            serde_json::json!({ "action": "published", "release": { "tag_name": "v1.2.3" } }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("v1.2.3"));
    }

    #[test]
    fn describe_commit_comment_event() {
        let d = super::describe_event(&make_event("CommitCommentEvent", serde_json::json!({})));
        assert!(d.main.contains("commit"));
    }

    #[test]
    fn describe_gollum_event() {
        let d = super::describe_event(&make_event("GollumEvent", serde_json::json!({})));
        assert!(d.main.contains("wiki"));
    }

    #[test]
    fn describe_member_event() {
        let e = make_event(
            "MemberEvent",
            serde_json::json!({ "action": "added", "member": { "login": "bob" } }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("bob"));
    }

    #[test]
    fn describe_public_event() {
        let d = super::describe_event(&make_event("PublicEvent", serde_json::json!({})));
        assert!(d.main.contains("public"));
    }

    #[test]
    fn describe_unknown_event() {
        let d = super::describe_event(&make_event("CoolNewEvent", serde_json::json!({})));
        assert!(!d.main.is_empty());
    }

    #[test]
    fn describe_pr_event_merged() {
        let e = make_event(
            "PullRequestEvent",
            serde_json::json!({
                "action": "closed",
                "pull_request": {
                    "number": 10, "title": "Big feature",
                    "merged": true,
                    "additions": 50, "deletions": 5, "changed_files": 3
                }
            }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("Merged"));
        assert_eq!(d.detail.len(), 1);
    }

    #[test]
    fn describe_push_event_no_size_field() {
        // When "size" is absent, falls back to commits array length.
        let e = make_event(
            "PushEvent",
            serde_json::json!({
                "ref": "refs/heads/main",
                "commits": [{ "message": "only commit" }]
            }),
        );
        let d = super::describe_event(&e);
        assert!(d.main.contains("1 commit"));
    }

    #[test]
    fn event_url_push_without_head_uses_branch() {
        let e = make_event("PushEvent", serde_json::json!({ "ref": "refs/heads/feat" }));
        let url = super::event_url(&e).unwrap();
        assert!(url.contains("feat"));
    }

    #[test]
    fn event_url_pull_request_uses_html_url() {
        let e = make_event(
            "PullRequestEvent",
            serde_json::json!({ "pull_request": { "html_url": "https://github.com/alice/repo/pull/1" } }),
        );
        assert_eq!(
            super::event_url(&e),
            Some("https://github.com/alice/repo/pull/1".to_string())
        );
    }

    #[test]
    fn event_url_issues_event() {
        let e = make_event(
            "IssuesEvent",
            serde_json::json!({ "issue": { "html_url": "https://github.com/alice/repo/issues/2" } }),
        );
        assert_eq!(
            super::event_url(&e),
            Some("https://github.com/alice/repo/issues/2".to_string())
        );
    }

    #[test]
    fn event_url_fork_event() {
        let e = make_event(
            "ForkEvent",
            serde_json::json!({ "forkee": { "html_url": "https://github.com/bob/repo" } }),
        );
        assert_eq!(
            super::event_url(&e),
            Some("https://github.com/bob/repo".to_string())
        );
    }

    #[test]
    fn event_url_catchall_returns_repo_url() {
        let url = super::event_url(&make_event("WatchEvent", serde_json::json!({}))).unwrap();
        assert!(url.contains("alice/repo"));
    }

    #[test]
    fn render_activity_many_event_types() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let events: Vec<GitHubEvent> = [
            "IssuesEvent",
            "IssueCommentEvent",
            "PullRequestReviewEvent",
            "CreateEvent",
            "DeleteEvent",
            "ForkEvent",
            "ReleaseEvent",
            "WatchEvent",
        ]
        .iter()
        .map(|kind| make_event(kind, serde_json::json!({})))
        .collect();
        super::render(&mut builder, &events, &std::collections::HashMap::new());
        assert!(!builder.finish().is_empty());
    }
}

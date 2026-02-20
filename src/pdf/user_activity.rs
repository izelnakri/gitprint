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
    let gray = Color::Rgb(Rgb::new(0.47, 0.47, 0.47, None));
    let dark_gray = Color::Rgb(Rgb::new(0.25, 0.25, 0.25, None));

    // ── Section title ──────────────────────────────────────────────────────────
    builder.ensure_space(builder.line_height() * 3.0);
    builder.write_centered("Recent Activity", &bold, Pt(16.0), black.clone());
    builder.vertical_space(12.0);

    // ── Events grouped by date ─────────────────────────────────────────────────
    let mut last_date = String::new();
    events.iter().for_each(|event| {
        let date = event.created_at.get(..10).unwrap_or(&event.created_at);
        if date != last_date {
            builder.vertical_space(6.0);
            builder.ensure_space(builder.line_height() * 2.0);
            builder.write_line(&[Span {
                text: date.to_string(),
                font_id: bold.clone(),
                size: Pt(9.0),
                color: dark_gray.clone(),
            }]);
            last_date = date.to_string();
        }

        let time = event.created_at.get(11..16).unwrap_or("");
        let description = describe_event(event);

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

        builder.write_line(&[
            Span {
                text: "  · ".to_string(),
                font_id: regular.clone(),
                size: Pt(8.5),
                color: gray.clone(),
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
        if let Some(url) = event_url(event) {
            builder.add_link(builder.line_height(), Actions::Uri(url));
        }

        // Detail lines (commit messages, PR diff stats, etc.)
        detail.iter().for_each(|detail| {
            builder.write_line(&[
                Span {
                    text: "    ".to_string(),
                    font_id: regular.clone(),
                    size: Pt(7.5),
                    color: gray.clone(),
                },
                Span {
                    text: detail.clone(),
                    font_id: italic.clone(),
                    size: Pt(7.5),
                    color: gray.clone(),
                },
            ]);
        });
    });

    builder.vertical_space(12.0);
    builder.page_break();
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
            // `size` is the total; `commits` only holds distinct ones and can be empty
            // even for real pushes (force-push / rebase). Fall back to commits.len().
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
            (format!("Forked {repo} → {forkee}"), vec![])
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
            // Prefer a direct link to the HEAD commit; fall back to branch tree.
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
        assert!(desc.main.contains("Pushed 2 commits")); // size absent → falls back to commits.len()
        assert!(desc.main.contains("alice/myrepo"));
        assert!(desc.main.contains("main"));
        assert_eq!(desc.detail.len(), 2); // distinct commits still listed
    }

    #[test]
    fn describe_pr_event() {
        let desc = super::describe_event(&pr_event());
        assert!(desc.main.contains("Opened PR #42"));
        assert!(desc.main.contains("Add dark mode"));
    }
}

use std::collections::HashMap;

use printpdf::{Actions, Color, Pt, Rgb};

use super::layout::{PageBuilder, Span};
use crate::github::{GitHubEvent, GitHubRepo};

pub fn render(
    builder: &mut PageBuilder,
    title: &str,
    repos: &[GitHubRepo],
    events: &[GitHubEvent],
    commit_msgs: &std::collections::HashMap<String, Vec<String>>,
) {
    if repos.is_empty() {
        return;
    }

    // Index events by repo name, keeping the newest (API returns newest-first).
    let push_ctx: HashMap<&str, &GitHubEvent> = events
        .iter()
        .filter(|e| e.kind == "PushEvent")
        .fold(HashMap::new(), |mut map, e| {
            map.entry(e.repo.name.as_str()).or_insert(e);
            map
        });
    let activity_ctx: HashMap<&str, &GitHubEvent> = events
        .iter()
        .filter(|e| e.kind != "PushEvent")
        .fold(HashMap::new(), |mut map, e| {
            map.entry(e.repo.name.as_str()).or_insert(e);
            map
        });

    let bold = builder.font(true, false).clone();
    let regular = builder.font(false, false).clone();
    let italic = builder.font(false, true).clone();
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));
    let gray = Color::Rgb(Rgb::new(0.47, 0.47, 0.47, None));
    let dark_gray = Color::Rgb(Rgb::new(0.25, 0.25, 0.25, None));

    builder.ensure_space(builder.line_height() * 3.0);
    builder.write_centered(title, &bold, Pt(14.0), black.clone());
    builder.vertical_space(10.0);

    repos.iter().for_each(|repo| {
        builder.ensure_space(builder.line_height() * 5.0);

        // ── Row 1: name (left) + stats (right) ─────────────────────────────
        let fork_tag = if repo.fork { " [fork]" } else { "" };
        let lang = repo.language.as_deref().unwrap_or("—");
        let stats = format!(
            "{} stars  {} forks  {} issues  {}",
            repo.stargazers_count, repo.forks_count, repo.open_issues_count, lang
        );
        builder.write_line_justified(
            &[Span {
                text: format!("{}{fork_tag}", repo.name),
                font_id: bold.clone(),
                size: Pt(9.0),
                color: black.clone(),
            }],
            &[Span {
                text: stats,
                font_id: regular.clone(),
                size: Pt(8.0),
                color: dark_gray.clone(),
            }],
        );
        builder.add_link(builder.line_height(), Actions::Uri(repo.html_url.clone()));

        // ── Row 2: description ──────────────────────────────────────────────
        if let Some(desc) = repo.description.as_deref().filter(|d| !d.is_empty()) {
            builder.write_line(&[Span {
                text: format!("  {desc}"),
                font_id: italic.clone(),
                size: Pt(8.0),
                color: gray.clone(),
            }]);
        }

        // ── Row 3: dates + size ─────────────────────────────────────────────
        let pushed = repo
            .pushed_at
            .as_deref()
            .or(repo.updated_at.as_deref())
            .and_then(|d| d.get(..10))
            .unwrap_or("—");
        let created = repo
            .created_at
            .as_deref()
            .and_then(|d| d.get(..10))
            .unwrap_or("—");
        let size_part = match repo.size {
            0 => String::new(),
            kb if kb < 1024 => format!("  ·  {kb} KB"),
            kb => format!("  ·  {:.1} MB", kb as f64 / 1024.0),
        };
        builder.write_line(&[Span {
            text: format!("  last push {pushed}  ·  created {created}{size_part}"),
            font_id: regular.clone(),
            size: Pt(7.5),
            color: gray.clone(),
        }]);

        // ── Row 4: your recent activity context ─────────────────────────────
        // Push event → show branch + commit messages (you pushed code here).
        // Non-push event → show what you did (opened issue, reviewed PR, etc.).
        if let Some(ev) = push_ctx.get(repo.full_name.as_str()) {
            let branch = ev.payload["ref"]
                .as_str()
                .unwrap_or("")
                .trim_start_matches("refs/heads/");
            // Commit messages from the event payload (present for normal pushes).
            let from_payload: Vec<String> = ev.payload["commits"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| c["message"].as_str())
                .map(|m| m.lines().next().unwrap_or(m).to_string())
                .take(2)
                .collect();
            // Fall back to API-fetched messages (force push / rebase gave empty payload).
            let commits = if from_payload.is_empty() {
                commit_msgs
                    .get(&repo.full_name)
                    .map(|msgs| msgs.iter().take(2).cloned().collect::<Vec<_>>())
                    .unwrap_or_default()
            } else {
                from_payload
            };
            if commits.is_empty() {
                let date = ev.created_at.get(..10).unwrap_or(&ev.created_at);
                builder.write_line(&[Span {
                    text: format!("  \u{2192} pushed to {branch} on {date}"),
                    font_id: italic.clone(),
                    size: Pt(7.5),
                    color: dark_gray.clone(),
                }]);
            } else {
                builder.write_line(&[Span {
                    text: format!("  \u{2192} pushed to {branch}:"),
                    font_id: italic.clone(),
                    size: Pt(7.5),
                    color: dark_gray.clone(),
                }]);
                commits.iter().for_each(|msg| {
                    builder.write_line(&[Span {
                        text: format!("      {msg}"),
                        font_id: italic.clone(),
                        size: Pt(7.5),
                        color: gray.clone(),
                    }]);
                });
            }
        } else if let Some(ev) = activity_ctx.get(repo.full_name.as_str()) {
            let date = ev.created_at.get(..10).unwrap_or(&ev.created_at);
            builder.write_line(&[Span {
                text: format!("  \u{2192} {} on {date}", brief_activity(ev)),
                font_id: italic.clone(),
                size: Pt(7.5),
                color: dark_gray.clone(),
            }]);
        }

        builder.vertical_space(4.0);
    });

    builder.vertical_space(12.0);
}

/// One-line description of a non-push GitHub event for display in the repo context row.
fn brief_activity(event: &GitHubEvent) -> String {
    let p = &event.payload;
    match event.kind.as_str() {
        "PullRequestEvent" => {
            let action = p["action"].as_str().unwrap_or("updated");
            let n = p["pull_request"]["number"].as_u64().unwrap_or(0);
            let merged =
                action == "closed" && p["pull_request"]["merged"].as_bool().unwrap_or(false);
            if merged {
                format!("merged PR #{n}")
            } else {
                format!("{action} PR #{n}")
            }
        }
        "IssuesEvent" => {
            let action = p["action"].as_str().unwrap_or("updated");
            let n = p["issue"]["number"].as_u64().unwrap_or(0);
            format!("{action} issue #{n}")
        }
        "IssueCommentEvent" => {
            let n = p["issue"]["number"].as_u64().unwrap_or(0);
            format!("commented on issue #{n}")
        }
        "PullRequestReviewEvent" | "PullRequestReviewCommentEvent" => {
            let n = p["pull_request"]["number"].as_u64().unwrap_or(0);
            format!("reviewed PR #{n}")
        }
        "WatchEvent" => "starred".to_string(),
        "ForkEvent" => "forked".to_string(),
        "ReleaseEvent" => {
            let tag = p["release"]["tag_name"].as_str().unwrap_or("");
            format!("released {tag}")
        }
        "CreateEvent" => {
            let ref_type = p["ref_type"].as_str().unwrap_or("ref");
            let ref_name = p["ref"].as_str().unwrap_or("");
            if ref_name.is_empty() {
                format!("created {ref_type}")
            } else {
                format!("created {ref_type} '{ref_name}'")
            }
        }
        other => other.replace("Event", "").to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf;
    use crate::types::Config;

    fn test_repo(name: &str, stars: u64) -> GitHubRepo {
        GitHubRepo {
            name: name.to_string(),
            full_name: format!("alice/{name}"),
            html_url: format!("https://github.com/alice/{name}"),
            description: Some(format!("{name} — a great project")),
            language: Some("Rust".to_string()),
            stargazers_count: stars,
            forks_count: 10,
            open_issues_count: 3,
            size: 2048,
            pushed_at: Some("2024-03-01T00:00:00Z".to_string()),
            updated_at: Some("2024-03-02T00:00:00Z".to_string()),
            created_at: Some("2020-06-15T00:00:00Z".to_string()),
            fork: false,
        }
    }

    fn test_push_event(repo: &str, branch: &str, msgs: &[&str]) -> GitHubEvent {
        use crate::github::EventRepo;
        let commits: Vec<serde_json::Value> = msgs
            .iter()
            .map(|m| serde_json::json!({ "message": m }))
            .collect();
        GitHubEvent {
            kind: "PushEvent".to_string(),
            repo: EventRepo {
                name: repo.to_string(),
            },
            payload: serde_json::json!({
                "ref": format!("refs/heads/{branch}"),
                "commits": commits,
                "size": msgs.len()
            }),
            created_at: "2024-03-01T09:00:00Z".to_string(),
        }
    }

    #[test]
    fn render_repos_does_not_panic() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        super::render(
            &mut builder,
            "Top Starred Repositories",
            &[test_repo("gitprint", 500), test_repo("another", 200)],
            &[],
            &std::collections::HashMap::new(),
        );
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_repos_empty_is_noop() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let page_before = builder.current_page();
        super::render(
            &mut builder,
            "Top Repos",
            &[],
            &[],
            &std::collections::HashMap::new(),
        );
        assert_eq!(builder.current_page(), page_before);
    }

    #[test]
    fn render_fork_repo_shows_tag() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let mut repo = test_repo("forked", 5);
        repo.fork = true;
        super::render(
            &mut builder,
            "Forks",
            &[repo],
            &[],
            &std::collections::HashMap::new(),
        );
        assert!(!builder.finish().is_empty());
    }

    fn test_issue_event(repo: &str, number: u64) -> GitHubEvent {
        use crate::github::EventRepo;
        GitHubEvent {
            kind: "IssuesEvent".to_string(),
            repo: EventRepo {
                name: repo.to_string(),
            },
            payload: serde_json::json!({ "action": "opened", "issue": { "number": number } }),
            created_at: "2024-03-02T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn render_repos_with_activity_event_context() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let events = [test_issue_event("alice/gitprint", 42)];
        super::render(
            &mut builder,
            "Repos You Were Active In",
            &[test_repo("gitprint", 100)],
            &events,
            &std::collections::HashMap::new(),
        );
        assert!(!builder.finish().is_empty());
    }

    #[test]
    fn render_repos_with_push_event_context() {
        let mut doc = printpdf::PdfDocument::new("test");
        let fonts = pdf::fonts::load_fonts(&mut doc).unwrap();
        let config = Config::test_default();
        let mut builder = pdf::create_builder(&config, fonts);
        let events = [test_push_event(
            "alice/gitprint",
            "main",
            &["fix: typo", "feat: add feature"],
        )];
        super::render(
            &mut builder,
            "Recently Pushed",
            &[test_repo("gitprint", 100)],
            &events,
            &std::collections::HashMap::new(),
        );
        assert!(!builder.finish().is_empty());
    }
}

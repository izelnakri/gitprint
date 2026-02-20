//! User report pipeline: fetch GitHub data in parallel, then render PDF.

use tokio::task::JoinSet;

use crate::github::{self, CommitDetail, GitHubEvent, GitHubRepo};
use crate::pdf;
use crate::types::{ActivityFilter, UserReportConfig};

/// Runs the full user report pipeline and writes a PDF to `config.output_path`.
pub async fn run(config: &UserReportConfig) -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    let token = config.github_token.as_deref();
    let username = &config.username;

    eprintln!("Fetching GitHub data for @{username}...");

    // ── Phase 1: parallel API fetches ─────────────────────────────────────────
    // search_user_commits returns the N most recent commits across ALL public repos,
    // regardless of push type (force push, rebase, etc.). It drives both the commit
    // diffs section and the per-SHA message enrichment in the activity/repos renderers.
    let (user_res, starred_res, active_res, pushed_res, events_res, search_commits_res) = tokio::join!(
        github::get_user(username, token),
        github::get_user_starred_repos(username, 5, token),
        github::get_user_repos(username, "updated", 5, token),
        github::get_user_repos(username, "pushed", config.last_committed, token),
        // Fetch up to 100 events (GitHub API max per page) so date/activity
        // filters have the most events to work with before the display limit is applied.
        github::get_user_events(username, 100, token),
        // Skip the search if diffs are disabled or not requested.
        async {
            if config.no_diffs || config.commits == 0 {
                Ok(vec![])
            } else {
                github::search_user_commits(username, config.commits, token).await
            }
        },
    );

    let user = user_res?;
    let starred_repos = starred_res.unwrap_or_default();

    // Coalesce duplicate push events first, then apply user-specified filters.
    let events = {
        let raw = coalesce_push_events(events_res.unwrap_or_default());

        // Date range filter — ISO 8601 strings sort lexicographically, so simple
        // string comparison is correct for YYYY-MM-DD prefixes.
        let date_filtered = raw.into_iter().filter(|e| {
            let date = e.created_at.get(..10).unwrap_or(&e.created_at);
            config.since.as_deref().is_none_or(|s| date >= s)
                && config.until.as_deref().is_none_or(|u| date <= u)
        });

        // Activity type filter.
        match config.activity {
            ActivityFilter::All => date_filtered.collect::<Vec<_>>(),
            ActivityFilter::Commits => date_filtered
                .filter(|e| e.kind == "PushEvent")
                .collect::<Vec<_>>(),
        }
    };

    // Build repo-name sets from the user's own event feed — these are definitively
    // the user's own actions, not collaborator or bot activity.
    let push_event_repos: std::collections::HashSet<String> = events
        .iter()
        .filter(|e| e.kind == "PushEvent")
        .map(|e| e.repo.name.clone())
        .collect();
    let other_event_repos: std::collections::HashSet<String> = events
        .iter()
        .filter(|e| e.kind != "PushEvent")
        .map(|e| e.repo.name.clone())
        .collect();

    // "Recently Pushed" = repos the user personally pushed code to.
    // Filter by push events so collaborator/bot pushes to owned repos are excluded.
    // Fall back to unfiltered if the event window is empty (new account / no events).
    let pushed_repos: Vec<_> = pushed_res
        .unwrap_or_default()
        .into_iter()
        .filter(|r| {
            !r.fork && (push_event_repos.is_empty() || push_event_repos.contains(&r.full_name))
        })
        .collect();

    // "Recently Active" = repos where the user did something *other* than pushing code
    // (opened issues, reviewed PRs, left comments, starred, etc.).
    // Excludes repos already shown in "Recently Pushed" to avoid duplicates.
    let pushed_names: std::collections::HashSet<&str> =
        pushed_repos.iter().map(|r| r.full_name.as_str()).collect();
    let active_repos: Vec<_> = active_res
        .unwrap_or_default()
        .into_iter()
        .filter(|r| {
            !r.fork
                && !pushed_names.contains(r.full_name.as_str())
                && (other_event_repos.is_empty() || other_event_repos.contains(&r.full_name))
        })
        .collect();

    // Total stars across all owned (non-fork) repos — use the starred list which
    // already gives us the top repos; for a rough total we sum what we have.
    let total_stars: u64 = starred_repos.iter().map(|r| r.stargazers_count).sum();

    // ── Phase 2: fetch commit details in parallel ──────────────────────────────
    // SHA → first-line-of-message lookup built from the search API results.
    // Keyed by commit SHA so each push event's HEAD can be enriched individually,
    // preventing the same messages from appearing across multiple push events.
    let search_commits = search_commits_res.unwrap_or_default();
    let commit_msgs: std::collections::HashMap<String, String> = search_commits
        .iter()
        .map(|(_, sha, msg)| (sha.clone(), msg.clone()))
        .collect();

    let commit_details: Vec<(String, CommitDetail)> = if !config.no_diffs && config.commits > 0 {
        let shas: Vec<(String, String)> = search_commits
            .into_iter()
            .map(|(repo, sha, _)| (repo, sha))
            .collect();
        eprintln!("Fetching {} commit diff(s)...", shas.len());
        let mut set: JoinSet<anyhow::Result<(String, CommitDetail)>> = JoinSet::new();
        shas.into_iter().for_each(|(repo, sha)| {
            let tok = token.map(str::to_string);
            set.spawn(async move {
                github::get_commit_detail(&repo, &sha, tok.as_deref())
                    .await
                    .map(|cd| (repo, cd))
            });
        });
        let mut details: Vec<(String, CommitDetail)> =
            set.join_all().await.into_iter().flatten().collect();
        // Sort newest-first so displayed messages are in chronological order.
        details.sort_unstable_by(|(_, a), (_, b)| b.commit.author.date.cmp(&a.commit.author.date));
        details
    } else {
        vec![]
    };

    // ── Phase 3: PDF assembly (sequential) ────────────────────────────────────
    eprintln!("Rendering PDF...");
    let mut doc = printpdf::PdfDocument::new(&format!("{username} — GitHub User Report"));
    let fonts = pdf::fonts::load_fonts(&mut doc)?;
    let mut builder = pdf::create_user_builder(config, fonts);

    // Cover page
    pdf::user_cover::render(&mut builder, &user, total_stars);

    // Activity feed — capped to the requested display limit.
    let display_events = &events[..config.events.min(events.len())];
    pdf::user_activity::render(&mut builder, display_events, &commit_msgs);

    // Repository sections — pass events + fetched commit msgs for rich context
    render_repos_section(
        &mut builder,
        "Top Starred Repositories",
        &starred_repos,
        5,
        &events,
        &commit_msgs,
    );
    render_repos_section(
        &mut builder,
        "Repos You Were Active In",
        &active_repos,
        5,
        &events,
        &commit_msgs,
    );
    render_repos_section(
        &mut builder,
        "Repos You Pushed To",
        &pushed_repos,
        config.last_committed,
        &events,
        &commit_msgs,
    );

    // Commit diffs
    if !commit_details.is_empty() {
        // Build SHA → branch from push events so each diff header can show the branch.
        let sha_to_branch: std::collections::HashMap<&str, &str> = events
            .iter()
            .filter(|e| e.kind == "PushEvent")
            .filter_map(|e| {
                let sha = e.payload["head"].as_str()?;
                let branch = e.payload["ref"].as_str()?.trim_start_matches("refs/heads/");
                Some((sha, branch))
            })
            .collect();

        let bold = builder.font(true, false).clone();
        let black = printpdf::Color::Rgb(printpdf::Rgb::new(0.0, 0.0, 0.0, None));
        builder.write_centered("Recent Commits", &bold, printpdf::Pt(16.0), black);
        builder.vertical_space(12.0);
        commit_details.iter().for_each(|(repo, detail)| {
            let branch = sha_to_branch.get(detail.sha.as_str()).copied();
            pdf::diff::render_commit(&mut builder, detail, repo, branch, config.font_size as f32);
        });
    }

    let pages = builder.finish();
    let total_pages = pages.len();
    doc.with_pages(pages);
    pdf::save_pdf(&doc, &config.output_path).await?;

    let elapsed = elapsed_str(start.elapsed());
    let pdf_size = tokio::fs::metadata(&config.output_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);
    eprintln!(
        "{} — {} pages, {}, {}",
        config.output_path.display(),
        total_pages,
        format_size(pdf_size),
        elapsed,
    );
    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Keep only the first PushEvent per (date, repo, branch) — GitHub emits one per push, so a busy
/// day can produce many identical-looking entries. Keeping the first (newest) is sufficient.
fn coalesce_push_events(events: Vec<GitHubEvent>) -> Vec<GitHubEvent> {
    let mut seen = std::collections::HashSet::new();
    events
        .into_iter()
        .filter(|event| {
            if event.kind != "PushEvent" {
                return true;
            }
            let date = event.created_at.get(..10).unwrap_or(&event.created_at);
            let branch = event.payload["ref"].as_str().unwrap_or("");
            seen.insert((
                date.to_string(),
                event.repo.name.clone(),
                branch.to_string(),
            ))
        })
        .collect()
}

fn render_repos_section(
    builder: &mut crate::pdf::layout::PageBuilder,
    title: &str,
    repos: &[GitHubRepo],
    limit: usize,
    events: &[GitHubEvent],
    commit_msgs: &std::collections::HashMap<String, String>,
) {
    if limit == 0 || repos.is_empty() {
        return;
    }
    let capped: Vec<_> = repos.iter().take(limit).cloned().collect();
    pdf::user_repos::render(builder, title, &capped, events, commit_msgs);
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn elapsed_str(d: std::time::Duration) -> String {
    if d.as_millis() < 1000 {
        format!("{}ms", d.as_millis())
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::EventRepo;

    fn make_push_event(repo: &str) -> GitHubEvent {
        GitHubEvent {
            kind: "PushEvent".to_string(),
            repo: EventRepo {
                name: repo.to_string(),
            },
            payload: serde_json::json!({ "ref": "refs/heads/main", "commits": [] }),
            created_at: "2024-03-01T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn coalesce_keeps_first_push_per_day_branch() {
        let events = vec![
            make_push_event("alice/a"),
            make_push_event("alice/a"),
            make_push_event("alice/a"),
        ];
        let out = coalesce_push_events(events);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn coalesce_keeps_different_branches_separate() {
        let mut ev2 = make_push_event("alice/a");
        ev2.payload["ref"] = serde_json::json!("refs/heads/dev");
        let events = vec![make_push_event("alice/a"), ev2];
        assert_eq!(coalesce_push_events(events).len(), 2);
    }

    #[test]
    fn coalesce_preserves_non_push_events() {
        let events = vec![
            GitHubEvent {
                kind: "WatchEvent".to_string(),
                repo: EventRepo {
                    name: "alice/a".to_string(),
                },
                payload: serde_json::json!({}),
                created_at: "2024-03-01T00:00:00Z".to_string(),
            },
            make_push_event("alice/a"),
            make_push_event("alice/a"),
        ];
        let out = coalesce_push_events(events);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].kind, "WatchEvent");
    }
}

//! User report pipeline: fetch GitHub data in parallel, then render PDF.

use tokio::task::JoinSet;

use crate::github::{self, CommitDetail, GitHubEvent, GitHubRepo};
use crate::pdf;
use crate::types::UserReportConfig;

/// Runs the full user report pipeline and writes a PDF to `config.output_path`.
pub async fn run(config: &UserReportConfig) -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    let token = config.github_token.as_deref();
    let username = &config.username;

    eprintln!("Fetching GitHub data for @{username}...");

    // ── Phase 1: parallel API fetches ─────────────────────────────────────────
    let (user_res, starred_res, active_res, pushed_res, events_res) = tokio::join!(
        github::get_user(username, token),
        github::get_user_starred_repos(username, config.top_starred, token),
        github::get_user_repos(username, "updated", config.last_repos, token),
        github::get_user_repos(username, "pushed", config.last_committed, token),
        github::get_user_events(username, 30, token),
    );

    let user = user_res?;
    let starred_repos = starred_res.unwrap_or_default();
    let events = coalesce_push_events(events_res.unwrap_or_default());

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
    // Store (repo_name, CommitDetail) so we can build a per-repo message lookup
    // for enriching the activity and repos sections.
    let commit_details: Vec<(String, CommitDetail)> = if !config.no_diffs && config.commits > 0 {
        let shas = extract_push_shas(&events, config.commits);
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

    // Per-repo commit message lookup: used by activity + repos renderers to show
    // commit messages even when the Events API omits them (force push / rebase).
    let commit_msgs: std::collections::HashMap<String, Vec<String>> =
        commit_details
            .iter()
            .fold(std::collections::HashMap::new(), |mut map, (repo, cd)| {
                map.entry(repo.clone()).or_default().push(
                    cd.commit
                        .message
                        .lines()
                        .next()
                        .unwrap_or(&cd.commit.message)
                        .to_string(),
                );
                map
            });

    // ── Phase 3: PDF assembly (sequential) ────────────────────────────────────
    eprintln!("Rendering PDF...");
    let mut doc = printpdf::PdfDocument::new(&format!("{username} — GitHub User Report"));
    let fonts = pdf::fonts::load_fonts(&mut doc)?;
    let mut builder = pdf::create_user_builder(config, fonts);

    // Cover page
    pdf::user_cover::render(&mut builder, &user, total_stars);

    // Activity feed
    pdf::user_activity::render(&mut builder, &events, &commit_msgs);

    // Repository sections — pass events + fetched commit msgs for rich context
    render_repos_section(
        &mut builder,
        "Top Starred Repositories",
        &starred_repos,
        config.top_starred,
        &events,
        &commit_msgs,
    );
    render_repos_section(
        &mut builder,
        "Repos You Were Active In",
        &active_repos,
        config.last_repos,
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
        let bold = builder.font(true, false).clone();
        let black = printpdf::Color::Rgb(printpdf::Rgb::new(0.0, 0.0, 0.0, None));
        builder.write_centered("Recent Commits", &bold, printpdf::Pt(16.0), black);
        builder.vertical_space(12.0);
        commit_details.iter().for_each(|(_, detail)| {
            pdf::diff::render_commit(&mut builder, detail, config.font_size as f32);
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

/// Extract up to `limit` (repo, sha) pairs from PushEvents, newest first.
///
/// GitHub omits individual commit entries for force-pushes and rebases but always
/// includes `payload.head` (the resulting HEAD SHA). We fall back to that so we can
/// still fetch the commit message for those events.
fn extract_push_shas(events: &[GitHubEvent], limit: usize) -> Vec<(String, String)> {
    events
        .iter()
        .filter(|e| e.kind == "PushEvent")
        .flat_map(|e| {
            let repo = e.repo.name.clone();
            let from_commits: Vec<_> = e.payload["commits"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| c["sha"].as_str().map(str::to_string))
                .map(|sha| (repo.clone(), sha))
                .collect();
            if from_commits.is_empty() {
                e.payload["head"]
                    .as_str()
                    .map(|sha| vec![(repo, sha.to_string())])
                    .unwrap_or_default()
            } else {
                from_commits
            }
        })
        .take(limit)
        .collect()
}

fn render_repos_section(
    builder: &mut crate::pdf::layout::PageBuilder,
    title: &str,
    repos: &[GitHubRepo],
    limit: usize,
    events: &[GitHubEvent],
    commit_msgs: &std::collections::HashMap<String, Vec<String>>,
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

    fn make_push_event(repo: &str, shas: &[&str]) -> GitHubEvent {
        let commits: Vec<serde_json::Value> = shas
            .iter()
            .map(|s| serde_json::json!({ "sha": s }))
            .collect();
        GitHubEvent {
            kind: "PushEvent".to_string(),
            repo: EventRepo {
                name: repo.to_string(),
            },
            payload: serde_json::json!({ "ref": "refs/heads/main", "commits": commits }),
            created_at: "2024-03-01T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn extract_push_shas_respects_limit() {
        let events = vec![
            make_push_event("alice/a", &["sha1", "sha2", "sha3"]),
            make_push_event("alice/b", &["sha4", "sha5"]),
        ];
        let shas = extract_push_shas(&events, 4);
        assert_eq!(shas.len(), 4);
        assert_eq!(shas[0], ("alice/a".to_string(), "sha1".to_string()));
        assert_eq!(shas[3], ("alice/b".to_string(), "sha4".to_string()));
    }

    #[test]
    fn extract_push_shas_skips_non_push() {
        let events = vec![
            GitHubEvent {
                kind: "WatchEvent".to_string(),
                repo: EventRepo {
                    name: "alice/a".to_string(),
                },
                payload: serde_json::json!({}),
                created_at: "2024-03-01T00:00:00Z".to_string(),
            },
            make_push_event("alice/b", &["sha1"]),
        ];
        let shas = extract_push_shas(&events, 10);
        assert_eq!(shas.len(), 1);
        assert_eq!(shas[0].0, "alice/b");
    }

    #[test]
    fn extract_push_shas_empty_events() {
        assert!(extract_push_shas(&[], 5).is_empty());
    }

    #[test]
    fn coalesce_keeps_first_push_per_day_branch() {
        let events = vec![
            make_push_event("alice/a", &["sha1"]),
            make_push_event("alice/a", &["sha2"]),
            make_push_event("alice/a", &["sha3"]),
        ];
        let out = coalesce_push_events(events);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].payload["commits"][0]["sha"], "sha1");
    }

    #[test]
    fn coalesce_keeps_different_branches_separate() {
        let mut ev2 = make_push_event("alice/a", &["sha2"]);
        ev2.payload["ref"] = serde_json::json!("refs/heads/dev");
        let events = vec![make_push_event("alice/a", &["sha1"]), ev2];
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
            make_push_event("alice/a", &["sha1"]),
            make_push_event("alice/a", &["sha2"]),
        ];
        let out = coalesce_push_events(events);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].kind, "WatchEvent");
    }
}

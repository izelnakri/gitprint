//! User report pipeline: fetch GitHub data in parallel, then render PDF.

use tokio::task::JoinSet;

use crate::github::{self, CommitDetail, GitHubEvent, GitHubRepo, GitHubUser};
use crate::pdf;
use crate::types::{ActivityFilter, UserReportConfig};

/// Pre-fetched GitHub data consumed by the PDF render phase.
///
/// Separating the fetch phase from the render phase keeps the render logic
/// testable without any network I/O.
pub(crate) struct UserReportData {
    pub user: GitHubUser,
    pub total_stars: u64,
    pub starred_repos: Vec<GitHubRepo>,
    pub active_repos: Vec<GitHubRepo>,
    pub pushed_repos: Vec<GitHubRepo>,
    pub events: Vec<GitHubEvent>,
    pub commit_msgs: std::collections::HashMap<String, String>,
    pub commit_details: Vec<(String, CommitDetail)>,
}

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
    // Propagate rate-limit errors so the user sees an actionable message; for
    // other transient failures degrade gracefully to an empty commit list.
    let search_commits = match search_commits_res {
        Ok(commits) => commits,
        Err(e) if e.to_string().contains("rate limit") => return Err(e),
        Err(_) => vec![],
    };
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
        // Propagate rate-limit errors; silently drop other per-commit failures (e.g. 404).
        let mut details: Vec<(String, CommitDetail)> = set
            .join_all()
            .await
            .into_iter()
            .filter_map(|r| match r {
                Ok(pair) => Some(Ok(pair)),
                Err(e) if e.to_string().contains("rate limit") => Some(Err(e)),
                Err(_) => None,
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        // Sort newest-first so displayed messages are in chronological order.
        details.sort_unstable_by(|(_, a), (_, b)| b.commit.author.date.cmp(&a.commit.author.date));
        details
    } else {
        vec![]
    };

    // ── Phase 3: PDF assembly (sequential) ────────────────────────────────────
    eprintln!("Rendering PDF...");
    let data = UserReportData {
        user,
        total_stars,
        starred_repos,
        active_repos,
        pushed_repos,
        events,
        commit_msgs,
        commit_details,
    };
    let (doc, total_pages) = render_to_doc(config, &data)?;
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

/// Render the user report PDF from pre-fetched data.
///
/// Returns the assembled `PdfDocument` (ready to save) and the page count.
/// No network I/O is performed — all data must be supplied via `data`.
pub(crate) fn render_to_doc(
    config: &UserReportConfig,
    data: &UserReportData,
) -> anyhow::Result<(printpdf::PdfDocument, usize)> {
    let mut doc = printpdf::PdfDocument::new(&format!("{} — GitHub User Report", config.username));
    let fonts = pdf::fonts::load_fonts(&mut doc)?;
    let mut builder = pdf::create_user_builder(config, fonts);

    // Cover page
    pdf::user_cover::render(&mut builder, &data.user, data.total_stars);

    // Activity feed — capped to the requested display limit.
    let display_events = &data.events[..config.events.min(data.events.len())];
    pdf::user_activity::render(&mut builder, display_events, &data.commit_msgs);

    // Repository sections — pass events + fetched commit msgs for rich context
    render_repos_section(
        &mut builder,
        "Top Starred Repositories",
        &data.starred_repos,
        5,
        &data.events,
        &data.commit_msgs,
    );
    render_repos_section(
        &mut builder,
        "Repos You Were Active In",
        &data.active_repos,
        5,
        &data.events,
        &data.commit_msgs,
    );
    render_repos_section(
        &mut builder,
        "Repos User Pushed To",
        &data.pushed_repos,
        config.last_committed,
        &data.events,
        &data.commit_msgs,
    );

    // Commit diffs
    if !data.commit_details.is_empty() {
        // Build SHA → branch from push events so each diff header can show the branch.
        let sha_to_branch: std::collections::HashMap<&str, &str> = data
            .events
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
        data.commit_details.iter().for_each(|(repo, detail)| {
            let branch = sha_to_branch.get(detail.sha.as_str()).copied();
            pdf::diff::render_commit(&mut builder, detail, repo, branch, config.font_size as f32);
        });
    }

    let pages = builder.finish();
    let page_count = pages.len();
    doc.with_pages(pages);
    Ok((doc, page_count))
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
    use crate::github::{CommitAuthor, CommitFile, CommitInfo, EventRepo, GitHubUser};
    use crate::types::{ActivityFilter, PaperSize};

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

    // ── render_to_doc offline tests ───────────────────────────────────────────

    fn mock_user() -> GitHubUser {
        GitHubUser {
            login: "alice".to_string(),
            name: Some("Alice".to_string()),
            bio: None,
            location: None,
            company: None,
            blog: None,
            email: None,
            public_repos: 5,
            followers: 10,
            following: 5,
            created_at: "2020-01-01T00:00:00Z".to_string(),
            html_url: "https://github.com/alice".to_string(),
        }
    }

    fn mock_config(commits: usize) -> UserReportConfig {
        UserReportConfig {
            username: "alice".to_string(),
            output_path: "/tmp/test-commits.pdf".into(),
            paper_size: PaperSize::A4,
            landscape: false,
            last_committed: 0,
            commits,
            no_diffs: false,
            font_size: 8.0,
            github_token: None,
            since: None,
            until: None,
            activity: ActivityFilter::All,
            events: 0,
        }
    }

    /// Generate a mock `(repo, CommitDetail)` with a large diff patch so it
    /// occupies several lines in the rendered PDF.
    fn mock_commit_detail(idx: usize) -> (String, CommitDetail) {
        let patch: String = (0..80).map(|i| format!("+added line {i}\n")).collect();
        (
            format!("alice/repo{idx}"),
            CommitDetail {
                sha: format!("{idx:040x}"),
                html_url: format!("https://github.com/alice/repo{idx}/commit/{idx:040x}"),
                commit: CommitInfo {
                    message: format!("commit #{idx}: add many lines"),
                    author: CommitAuthor {
                        name: "Alice".to_string(),
                        date: format!("2024-03-{:02}T12:00:00Z", idx % 28 + 1),
                    },
                },
                files: vec![CommitFile {
                    filename: format!("src/module{idx}.rs"),
                    status: "modified".to_string(),
                    additions: 80,
                    deletions: 0,
                    patch: Some(patch),
                }],
            },
        )
    }

    fn empty_report_data() -> UserReportData {
        UserReportData {
            user: mock_user(),
            total_stars: 0,
            starred_repos: vec![],
            active_repos: vec![],
            pushed_repos: vec![],
            events: vec![],
            commit_msgs: std::collections::HashMap::new(),
            commit_details: vec![],
        }
    }

    #[test]
    fn render_to_doc_no_commits_succeeds() {
        let (_, pages) = render_to_doc(&mock_config(0), &empty_report_data()).unwrap();
        assert!(pages > 0);
    }

    /// More commits with large diffs must produce more PDF pages than zero commits.
    /// This verifies the `--commits` flag actually drives the diff render path.
    #[test]
    fn more_commits_yields_more_pages() {
        let (_, pages_baseline) = render_to_doc(&mock_config(0), &empty_report_data()).unwrap();

        let data_with_commits = UserReportData {
            commit_details: (0..10).map(mock_commit_detail).collect(),
            ..empty_report_data()
        };
        let (_, pages_with_commits) = render_to_doc(&mock_config(10), &data_with_commits).unwrap();

        assert!(
            pages_with_commits > pages_baseline,
            "expected more pages with commits ({pages_with_commits}) than without ({pages_baseline})"
        );
    }
}

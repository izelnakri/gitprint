use std::path::PathBuf;

use clap::Parser;

/// Parse a human- or machine-readable date string into a `YYYY-MM-DD` string.
///
/// Accepted formats:
/// - ISO 8601: `2024-01-15` or `2024-01-15T00:00:00Z`
/// - Relative:  `today`, `yesterday`, `N days ago`, `N weeks ago`, `N months ago`
fn parse_date_filter(s: &str) -> anyhow::Result<String> {
    let s = s.trim();
    // ISO 8601: starts with four-digit year followed by '-'
    if s.len() >= 10 && s[..4].parse::<u32>().is_ok() && s.as_bytes().get(4) == Some(&b'-') {
        return Ok(s[..10].to_string());
    }
    let lower = s.to_lowercase();
    let days: u64 = if lower == "today" {
        0
    } else if lower == "yesterday" {
        1
    } else if let Some(n) = lower
        .strip_suffix(" days ago")
        .or_else(|| lower.strip_suffix(" day ago"))
    {
        n.trim()
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("invalid date: {s:?}"))?
    } else if let Some(n) = lower
        .strip_suffix(" weeks ago")
        .or_else(|| lower.strip_suffix(" week ago"))
    {
        n.trim()
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("invalid date: {s:?}"))?
            * 7
    } else if let Some(n) = lower
        .strip_suffix(" months ago")
        .or_else(|| lower.strip_suffix(" month ago"))
    {
        n.trim()
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("invalid date: {s:?}"))?
            * 30
    } else {
        anyhow::bail!(
            "unrecognized date: {s:?}\n\
             Accepted: 2024-01-15 · today · yesterday · 30 days ago · 2 weeks ago · 1 month ago\n\
             (singular and plural both accepted: \"1 day ago\" or \"2 days ago\")"
        );
    };
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!(e))?
        .as_secs()
        .saturating_sub(days * 86_400);
    Ok(unix_secs_to_date(secs))
}

/// Convert a Unix timestamp (seconds, UTC) to a `YYYY-MM-DD` string without external crates.
fn unix_secs_to_date(secs: u64) -> String {
    let mut days = secs / 86_400;
    let mut year = 1970u32;
    loop {
        let in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < in_year {
            break;
        }
        days -= in_year;
        year += 1;
    }
    let month_lengths = if is_leap_year(year) {
        [31u64, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u32;
    for &ml in &month_lengths {
        if days < ml {
            break;
        }
        days -= ml;
        month += 1;
    }
    let day = days + 1;
    format!("{year:04}-{month:02}-{day:02}")
}

fn is_leap_year(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[tokio::main]
async fn main() {
    let args = gitprint::cli::Args::parse();

    if args.list_themes {
        gitprint::highlight::list_themes()
            .iter()
            .for_each(|t| println!("  {t}"));
        return;
    }

    // ── User report mode ───────────────────────────────────────────────────────
    if let Some(username) = args.user {
        let output_path = args
            .output
            .unwrap_or_else(|| PathBuf::from(format!("{username}.pdf")));

        // Parse date range flags — exit early with a clear message on bad input.
        let since = match args.since.as_deref().map(parse_date_filter) {
            Some(Err(e)) => {
                eprintln!("error: --since: {e}");
                std::process::exit(1);
            }
            other => other.and_then(Result::ok),
        };
        let until = match args.until.as_deref().map(parse_date_filter) {
            Some(Err(e)) => {
                eprintln!("error: --until: {e}");
                std::process::exit(1);
            }
            other => other.and_then(Result::ok),
        };

        let config = gitprint::types::UserReportConfig {
            output_path,
            paper_size: args.paper_size,
            landscape: args.landscape,
            last_committed: args.last_committed,
            commits: args.commits,
            no_diffs: args.no_diffs,
            font_size: args.font_size,
            github_token: std::env::var("GITHUB_TOKEN").ok(),
            since,
            until,
            activity: args.activity,
            events: args.events,
            username,
        };

        if let Err(e) = gitprint::user_report::run(&config).await {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // ── Repository mode ────────────────────────────────────────────────────────
    let path = match args.path {
        Some(p) => p,
        None => {
            eprintln!("error: a path or -u/--user is required");
            std::process::exit(1);
        }
    };

    let is_remote = gitprint::git::is_remote_url(&path);

    // Clone remote URL to a temp dir; hold it alive until after run().
    let temp_dir = if is_remote {
        eprintln!("Cloning {path}...");
        match gitprint::git::TempCloneDir::new().await {
            Ok(t) => {
                if let Err(e) = gitprint::git::clone_repo(
                    &path,
                    t.path(),
                    args.branch.as_deref(),
                    args.commit.as_deref(),
                )
                .await
                {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
                Some(t)
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let repo_path = temp_dir
        .as_ref()
        .map(|t| t.path().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(&path));

    let output_path = args.output.unwrap_or_else(|| {
        let name = if is_remote {
            gitprint::git::repo_name_from_url(&path)
        } else {
            PathBuf::from(&path)
                .canonicalize()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "output".to_string())
        };
        PathBuf::from(format!("{name}.pdf"))
    });

    let config = gitprint::types::Config {
        repo_path,
        output_path,
        include_patterns: args.include,
        exclude_patterns: args.exclude,
        theme: args.theme,
        font_size: args.font_size,
        no_line_numbers: args.no_line_numbers,
        toc: !args.no_toc,
        file_tree: !args.no_file_tree,
        branch: args.branch,
        commit: args.commit,
        paper_size: args.paper_size,
        landscape: args.landscape,
        remote_url: is_remote.then(|| path.clone()),
    };

    if let Err(e) = gitprint::run(&config).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_iso_date() {
        assert_eq!(parse_date_filter("2024-01-15").unwrap(), "2024-01-15");
        // ISO with time component — truncated to date
        assert_eq!(
            parse_date_filter("2024-06-30T12:00:00Z").unwrap(),
            "2024-06-30"
        );
    }

    #[test]
    fn parse_relative_dates() {
        // "today" and "yesterday" produce plausible YYYY-MM-DD strings
        let today = parse_date_filter("today").unwrap();
        assert_eq!(today.len(), 10);
        assert!(today.starts_with("20"));

        let yesterday = parse_date_filter("yesterday").unwrap();
        assert!(yesterday <= today);
    }

    #[test]
    fn parse_n_days_ago() {
        let d = parse_date_filter("30 days ago").unwrap();
        assert_eq!(d.len(), 10);
        let today = parse_date_filter("today").unwrap();
        assert!(d <= today);
    }

    #[test]
    fn parse_n_weeks_ago() {
        let d = parse_date_filter("2 weeks ago").unwrap();
        assert_eq!(d.len(), 10);
    }

    #[test]
    fn parse_n_months_ago() {
        let d = parse_date_filter("1 month ago").unwrap();
        assert_eq!(d.len(), 10);
    }

    #[test]
    fn parse_invalid_date_errors() {
        assert!(parse_date_filter("not a date").is_err());
        assert!(parse_date_filter("abc days ago").is_err());
    }

    #[test]
    fn unix_secs_known_dates() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        assert_eq!(unix_secs_to_date(1_704_067_200), "2024-01-01");
        // 2000-03-01 (leap year 2000, day after Feb 29)
        assert_eq!(unix_secs_to_date(951_868_800), "2000-03-01");
    }
}

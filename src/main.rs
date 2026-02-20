use std::path::PathBuf;

use clap::Parser;

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

        let config = gitprint::types::UserReportConfig {
            output_path,
            paper_size: args.paper_size,
            landscape: args.landscape,
            top_starred: args.top_starred,
            last_repos: args.last_repos,
            last_committed: args.last_committed,
            commits: args.commits,
            no_diffs: args.no_diffs,
            font_size: args.font_size,
            github_token: std::env::var("GITHUB_TOKEN").ok(),
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

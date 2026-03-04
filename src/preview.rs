//! Terminal preview mode — renders repository or user data to stdout
//! instead of generating a PDF.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::filter::FileFilter;
use crate::git;
use crate::github::{CommitDetail, GitHubEvent, GitHubRepo};
use crate::types::{Config, UserReportConfig};
use crate::user_report::fetch_data;
use crate::{format_size, format_utc_now};

// ── ANSI helpers ───────────────────────────────────────────────────────────────

struct Ansi {
    color: bool,
}

impl Ansi {
    fn new() -> Self {
        use std::io::IsTerminal;
        Self {
            color: std::io::stdout().is_terminal(),
        }
    }

    fn bold(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[1m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    fn dim(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[2m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    fn cyan(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[1;36m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    fn yellow(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[33m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    fn green(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[32m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    fn red(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[31m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    fn magenta(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[35m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }
}

fn divider(a: &Ansi) -> String {
    a.dim(&"─".repeat(64))
}

fn section_header(a: &Ansi, title: &str) {
    println!();
    println!("{}", divider(a));
    println!();
    println!("  {}", a.cyan(title));
    println!();
}

fn box_header(a: &Ansi, title: &str) {
    let inner = format!("  {title}  ");
    let width = 64usize.max(inner.len() + 4);
    let bar = "─".repeat(width - 2);
    println!("{}", a.dim(&format!("┌{bar}┐")));
    println!("{}", a.bold(&format!("│{inner:<pad$}│", pad = width - 2)));
    println!("{}", a.dim(&format!("└{bar}┘")));
}

fn kv(a: &Ansi, key: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    println!("  {}  {}", a.cyan(&format!("{key:<12}")), value);
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    s.chars().rev().enumerate().for_each(|(i, c)| {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    });
    result.chars().rev().collect()
}

fn fmt_u64(n: u64) -> String {
    format_number(n as usize)
}

// ── Repository preview ─────────────────────────────────────────────────────────

/// Previews a repository or file in the terminal.
pub async fn repo(config: &Config) -> anyhow::Result<()> {
    let a = Ansi::new();
    let info = git::verify_repo(&config.repo_path).await?;

    // ── Single-file mode ───────────────────────────────────────────────────────
    if let Some(ref single_file) = info.single_file {
        let (content_res, last_modified) = tokio::join!(
            git::read_file_content(&info.root, single_file, config),
            git::file_last_modified(&info.root, single_file, config, info.is_git),
        );
        let content = content_res?;
        let line_count = content.lines().count();
        let size_str = format_size(content.len() as u64);

        box_header(&a, &single_file.display().to_string());
        println!();
        kv(&a, "LINES", &format_number(line_count));
        kv(&a, "SIZE", &size_str);
        kv(&a, "MODIFIED", &last_modified);
        println!();
        println!("{}", divider(&a));
        println!();
        content.lines().enumerate().take(200).for_each(|(i, line)| {
            println!("  {}  {line}", a.dim(&format!("{:4}", i + 1)));
        });
        if line_count > 200 {
            println!(
                "  {}",
                a.dim(&format!(
                    "  … {} more lines",
                    format_number(line_count - 200)
                ))
            );
        }
        println!();
        return Ok(());
    }

    // ── Multi-file / repository mode ───────────────────────────────────────────
    let repo_path = info.root.clone();
    let is_git = info.is_git;
    let scope = info.scope.clone();
    let is_remote = config.remote_url.is_some();
    let generated_at = format_utc_now();
    let repo_path2 = repo_path.clone();
    let config2 = config.clone();
    let repo_path3 = repo_path.clone();

    let (metadata_res, all_paths_res, date_map_res, fs_owner_group, git_repo_size, fs_size) = tokio::join!(
        git::get_metadata(&repo_path, config, is_git, scope.as_deref()),
        git::list_tracked_files(&repo_path, config, is_git, scope.as_deref()),
        git::file_last_modified_dates(&repo_path, config, is_git, scope.as_deref()),
        async {
            if is_remote {
                (None, None)
            } else {
                git::fs_owner_group(&config.repo_path).await
            }
        },
        async {
            if is_git {
                git::git_tracked_size(&repo_path2, &config2).await
            } else {
                String::new()
            }
        },
        async {
            if is_remote {
                String::new()
            } else {
                git::fs_dir_size(&repo_path3).await
            }
        },
    );

    let mut metadata = metadata_res?;
    if let Some(ref url) = config.remote_url {
        metadata.name = git::repo_name_from_url(url);
    }
    metadata.fs_owner = fs_owner_group.0;
    metadata.fs_group = fs_owner_group.1;
    metadata.generated_at = generated_at;
    metadata.repo_size = git_repo_size;
    metadata.fs_size = fs_size;
    if !is_remote {
        metadata.repo_absolute_path = Some(repo_path.clone());
    }

    let date_map = Arc::new(date_map_res?);
    let file_filter = FileFilter::new(&config.include_patterns, &config.exclude_patterns)?;
    let mut paths: Vec<PathBuf> = file_filter.filter_paths(all_paths_res?).collect();
    paths.sort_unstable();

    // Read file contents in parallel to get LOC + size info.
    let mut read_set: tokio::task::JoinSet<Option<(PathBuf, usize, String, String)>> =
        tokio::task::JoinSet::new();
    paths.iter().for_each(|path| {
        let p = path.clone();
        let r = repo_path.clone();
        let c = config.clone();
        let dates = Arc::clone(&date_map);
        read_set.spawn(async move {
            let content = git::read_file_content(&r, &p, &c).await.ok()?;
            if crate::filter::is_binary(content.as_bytes()) || crate::filter::is_minified(&content)
            {
                return None;
            }
            let line_count = content.lines().count();
            let size_str = format_size(content.len() as u64);
            let last_modified = dates.get(&p).cloned().unwrap_or_default();
            Some((p, line_count, size_str, last_modified))
        });
    });

    let mut files: Vec<(PathBuf, usize, String, String)> =
        read_set.join_all().await.into_iter().flatten().collect();
    files.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    metadata.file_count = files.len();
    metadata.total_lines = files.iter().map(|(_, lc, _, _)| lc).sum();

    let effective_remote = config
        .remote_url
        .as_deref()
        .or(metadata.detected_remote_url.as_deref());

    // ── Header ─────────────────────────────────────────────────────────────────
    box_header(&a, &metadata.name);
    println!();

    let commit_first_line = metadata.commit_message.lines().next().unwrap_or("");
    let commit_line = format!(
        "{}  ·  {}  ·  {}",
        a.yellow(&metadata.commit_hash_short),
        metadata
            .commit_date
            .get(..10)
            .unwrap_or(&metadata.commit_date),
        commit_first_line,
    );
    kv(&a, "BRANCH", &metadata.branch);
    kv(&a, "COMMIT", &commit_line);
    let author = if metadata.commit_author_email.is_empty() {
        metadata.commit_author.clone()
    } else {
        format!(
            "{}  <{}>",
            metadata.commit_author, metadata.commit_author_email
        )
    };
    kv(&a, "AUTHOR", &author);
    if let Some(url) = effective_remote {
        kv(&a, "REMOTE", url);
    }
    if let Some(path) = &metadata.repo_absolute_path {
        kv(&a, "PATH", &path.display().to_string());
    }
    let size_info = match (metadata.repo_size.is_empty(), metadata.fs_size.is_empty()) {
        (false, false) => {
            format!(
                "{}  (git tracked)  ·  {}  (disk)",
                metadata.repo_size, metadata.fs_size
            )
        }
        (false, true) => metadata.repo_size.clone(),
        (true, false) => metadata.fs_size.clone(),
        _ => String::new(),
    };
    kv(&a, "SIZE", &size_info);
    if let (Some(owner), Some(group)) = (&metadata.fs_owner, &metadata.fs_group) {
        kv(&a, "OWNER", &format!("{owner}:{group}"));
    }
    kv(&a, "GENERATED", &metadata.generated_at);
    println!();
    println!(
        "  {}  {}    {}  {}",
        a.cyan("FILES"),
        a.bold(&format_number(metadata.file_count)),
        a.cyan("LINES"),
        a.bold(&format_number(metadata.total_lines)),
    );

    // ── Directory tree ─────────────────────────────────────────────────────────
    if config.file_tree {
        section_header(&a, "DIRECTORY TREE");
        let tree_paths: Vec<PathBuf> = files.iter().map(|(p, _, _, _)| p.clone()).collect();
        print_tree(&a, &tree_paths, &metadata.name);
    }

    // ── File list ──────────────────────────────────────────────────────────────
    section_header(&a, &format!("FILES  ({})", format_number(files.len())));

    let max_path = files
        .iter()
        .map(|(p, _, _, _)| p.display().to_string().len())
        .max()
        .unwrap_or(4)
        .min(60);
    let max_loc = files
        .iter()
        .map(|(_, lc, _, _)| format_number(*lc).len())
        .max()
        .unwrap_or(3)
        .max(3);
    let max_size = files
        .iter()
        .map(|(_, _, s, _)| s.len())
        .max()
        .unwrap_or(4)
        .max(4);

    println!(
        "  {}  {:<path_w$}  {:>loc_w$}  {:<size_w$}  {}",
        a.dim("    "),
        a.dim("PATH"),
        a.dim("LOC"),
        a.dim("SIZE"),
        a.dim("MODIFIED"),
        path_w = max_path,
        loc_w = max_loc,
        size_w = max_size,
    );
    println!(
        "  {}",
        a.dim(&"─".repeat(max_path + max_loc + max_size + 26))
    );

    files
        .iter()
        .enumerate()
        .for_each(|(i, (path, line_count, size_str, last_modified))| {
            println!(
                "  {}  {:<path_w$}  {:>loc_w$}  {:<size_w$}  {}",
                a.dim(&format!("{:4}.", i + 1)),
                path.display(),
                a.bold(&format_number(*line_count)),
                size_str,
                a.dim(last_modified),
                path_w = max_path,
                loc_w = max_loc,
                size_w = max_size,
            );
        });

    println!();
    Ok(())
}

// ── User report preview ────────────────────────────────────────────────────────

/// Previews a GitHub user report in the terminal.
pub async fn user(config: &UserReportConfig) -> anyhow::Result<()> {
    let a = Ansi::new();
    eprintln!("Fetching GitHub data for @{}...", config.username);
    let data = fetch_data(config).await?;

    // ── Header ─────────────────────────────────────────────────────────────────
    let title = match data.user.name.as_deref().filter(|n| !n.is_empty()) {
        Some(name) => format!("{name}  (@{})", data.user.login),
        None => format!("@{}", data.user.login),
    };
    box_header(&a, &title);
    println!();

    if let Some(ref bio) = data.user.bio {
        kv(&a, "BIO", bio);
    }
    if let Some(ref loc) = data.user.location {
        kv(&a, "LOCATION", loc);
    }
    if let Some(ref company) = data.user.company {
        kv(&a, "COMPANY", company);
    }
    if let Some(blog) = data.user.blog.as_deref().filter(|s| !s.is_empty()) {
        kv(&a, "BLOG", blog);
    }
    if let Some(email) = data.user.email.as_deref().filter(|s| !s.is_empty()) {
        kv(&a, "EMAIL", email);
    }
    kv(&a, "PROFILE", &data.user.html_url);

    let joined = data
        .user
        .created_at
        .get(..10)
        .unwrap_or(&data.user.created_at);
    println!();
    println!(
        "  {}  {}    {}  {}    {}  {}    {}  {}    {}  {}",
        a.cyan("REPOS"),
        a.bold(&fmt_u64(data.user.public_repos)),
        a.cyan("FOLLOWERS"),
        a.bold(&fmt_u64(data.user.followers)),
        a.cyan("FOLLOWING"),
        a.bold(&fmt_u64(data.user.following)),
        a.cyan("STARS"),
        a.bold(&fmt_u64(data.total_stars)),
        a.cyan("JOINED"),
        a.bold(joined),
    );

    // ── Activity feed ──────────────────────────────────────────────────────────
    let display_events = &data.events[..config.events.min(data.events.len())];
    if !display_events.is_empty() {
        section_header(
            &a,
            &format!("RECENT ACTIVITY  ({})", format_number(display_events.len())),
        );
        display_events.iter().for_each(|event| {
            print_event(&a, event, &data.commit_msgs);
        });
    }

    // ── Repositories ──────────────────────────────────────────────────────────
    if !data.starred_repos.is_empty() {
        section_header(&a, "TOP STARRED REPOSITORIES");
        data.starred_repos
            .iter()
            .take(5)
            .for_each(|r| print_repo(&a, r));
    }
    if !data.active_repos.is_empty() {
        section_header(&a, "REPOS YOU WERE ACTIVE IN");
        data.active_repos
            .iter()
            .take(5)
            .for_each(|r| print_repo(&a, r));
    }
    if !data.pushed_repos.is_empty() {
        section_header(&a, "RECENTLY PUSHED TO");
        data.pushed_repos
            .iter()
            .take(config.last_repos)
            .for_each(|r| print_repo(&a, r));
    }

    // ── Commits ───────────────────────────────────────────────────────────────
    if !data.commit_details.is_empty() {
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

        section_header(&a, "RECENT COMMITS");
        data.commit_details.iter().for_each(|(repo, detail)| {
            let branch = sha_to_branch
                .get(detail.sha.as_str())
                .copied()
                .unwrap_or("main");
            print_commit(&a, repo, detail, branch);
        });
    }

    println!();
    Ok(())
}

// ── Event printer ──────────────────────────────────────────────────────────────

fn print_event(
    a: &Ansi,
    event: &GitHubEvent,
    commit_msgs: &std::collections::HashMap<String, String>,
) {
    let date = event.created_at.get(..10).unwrap_or(&event.created_at);
    let repo = &event.repo.name;

    match event.kind.as_str() {
        "PushEvent" => {
            let branch = event.payload["ref"]
                .as_str()
                .unwrap_or("")
                .trim_start_matches("refs/heads/");
            let head_sha = event.payload["head"].as_str().unwrap_or("");
            let msg = head_sha
                .get(..7)
                .and_then(|short| commit_msgs.get(short).or_else(|| commit_msgs.get(head_sha)))
                .map(|s| s.as_str())
                .unwrap_or("");
            let commit_count = event.payload["commits"]
                .as_array()
                .map(|c| c.len())
                .unwrap_or(0);
            print!(
                "  {}  {}  {}  {}",
                a.dim(date),
                a.bold("push   "),
                a.cyan(repo),
                a.dim(&format!("· {branch}")),
            );
            if commit_count > 0 {
                print!("  {}", a.dim(&format!("[{commit_count} commit(s)]")));
            }
            println!();
            if !msg.is_empty() {
                println!(
                    "         {}  {}",
                    a.dim(&head_sha[..7.min(head_sha.len())]),
                    msg
                );
            }
        }
        "PullRequestEvent" => {
            let action = event.payload["action"].as_str().unwrap_or("?");
            let title = event.payload["pull_request"]["title"]
                .as_str()
                .unwrap_or("");
            let number = event.payload["number"].as_u64().unwrap_or(0);
            println!(
                "  {}  {}  {}  {}",
                a.dim(date),
                a.magenta("pr     "),
                a.cyan(repo),
                a.dim(&format!("#{number} {action}")),
            );
            if !title.is_empty() {
                println!("                   {title}");
            }
        }
        "IssuesEvent" => {
            let action = event.payload["action"].as_str().unwrap_or("?");
            let title = event.payload["issue"]["title"].as_str().unwrap_or("");
            let number = event.payload["issue"]["number"].as_u64().unwrap_or(0);
            println!(
                "  {}  {}  {}  {}",
                a.dim(date),
                a.yellow("issue  "),
                a.cyan(repo),
                a.dim(&format!("#{number} {action}")),
            );
            if !title.is_empty() {
                println!("                   {title}");
            }
        }
        "IssueCommentEvent" => {
            let number = event.payload["issue"]["number"].as_u64().unwrap_or(0);
            println!(
                "  {}  {}  {}  {}",
                a.dim(date),
                a.dim("comment"),
                a.cyan(repo),
                a.dim(&format!("issue #{number}")),
            );
        }
        "CreateEvent" => {
            let ref_type = event.payload["ref_type"].as_str().unwrap_or("?");
            let ref_name = event.payload["ref"].as_str().unwrap_or("");
            println!(
                "  {}  {}  {}  {}",
                a.dim(date),
                a.green("create "),
                a.cyan(repo),
                a.dim(&format!("{ref_type} {ref_name}")),
            );
        }
        "DeleteEvent" => {
            let ref_type = event.payload["ref_type"].as_str().unwrap_or("?");
            let ref_name = event.payload["ref"].as_str().unwrap_or("");
            println!(
                "  {}  {}  {}  {}",
                a.dim(date),
                a.red("delete "),
                a.cyan(repo),
                a.dim(&format!("{ref_type} {ref_name}")),
            );
        }
        "ForkEvent" => {
            let forkee = event.payload["forkee"]["full_name"].as_str().unwrap_or("");
            println!(
                "  {}  {}  {}  {}",
                a.dim(date),
                a.dim("fork   "),
                a.cyan(repo),
                a.dim(&format!("→ {forkee}")),
            );
        }
        "WatchEvent" => {
            println!("  {}  {}  {}", a.dim(date), a.dim("star   "), a.cyan(repo),);
        }
        "ReleaseEvent" => {
            let tag = event.payload["release"]["tag_name"].as_str().unwrap_or("");
            let name = event.payload["release"]["name"].as_str().unwrap_or("");
            println!(
                "  {}  {}  {}  {}",
                a.dim(date),
                a.green("release"),
                a.cyan(repo),
                a.dim(if name.is_empty() { tag } else { name }),
            );
        }
        "PullRequestReviewEvent" => {
            let state = event.payload["review"]["state"].as_str().unwrap_or("?");
            let number = event.payload["pull_request"]["number"]
                .as_u64()
                .unwrap_or(0);
            println!(
                "  {}  {}  {}  {}",
                a.dim(date),
                a.dim("review "),
                a.cyan(repo),
                a.dim(&format!("pr #{number} {state}")),
            );
        }
        other => {
            let label = other.trim_end_matches("Event");
            println!(
                "  {}  {}  {}",
                a.dim(date),
                a.dim(&format!("{label:<7}")),
                a.cyan(repo),
            );
        }
    }
}

// ── Repo printer ───────────────────────────────────────────────────────────────

fn print_repo(a: &Ansi, repo: &GitHubRepo) {
    let lang = repo.language.as_deref().unwrap_or("—");
    let desc = repo.description.as_deref().unwrap_or("");
    let pushed = repo
        .pushed_at
        .as_deref()
        .and_then(|d| d.get(..10))
        .unwrap_or("");

    println!(
        "  {:<42}  {}  {:>7}  {}  {}",
        a.bold(&repo.full_name),
        a.dim(&format!("{lang:<12}")),
        a.yellow(&format!("★ {}", fmt_u64(repo.stargazers_count))),
        a.dim(&format!("⑂ {}", fmt_u64(repo.forks_count))),
        a.dim(pushed),
    );
    if !desc.is_empty() {
        println!("  {}", a.dim(desc));
    }
}

// ── Commit printer ─────────────────────────────────────────────────────────────

fn print_commit(a: &Ansi, repo: &str, detail: &CommitDetail, branch: &str) {
    let short_sha = detail.sha.get(..7).unwrap_or(&detail.sha);
    let date = detail
        .commit
        .author
        .date
        .get(..10)
        .unwrap_or(&detail.commit.author.date);
    let msg = detail.commit.message.lines().next().unwrap_or("");
    let author = &detail.commit.author.name;

    println!(
        "  {}  {}  {}  {}  {}",
        a.yellow(short_sha),
        a.dim(date),
        a.cyan(repo),
        a.dim(&format!("({branch})")),
        a.bold(msg),
    );
    println!("  {}  {}", a.dim("              "), a.dim(author));

    let max_fname = detail
        .files
        .iter()
        .map(|f| f.filename.len())
        .max()
        .unwrap_or(0)
        .min(50);

    detail.files.iter().for_each(|f| {
        let status_label = match f.status.as_str() {
            "added" => a.green("added   "),
            "removed" | "deleted" => a.red("deleted "),
            "renamed" => a.magenta("renamed "),
            _ => a.dim("modified"),
        };
        println!(
            "  {}  {:<fname_w$}  {}  {}",
            a.dim("         "),
            f.filename,
            status_label,
            a.dim(&format!(
                "+{:<5} -{:<5}",
                fmt_u64(f.additions),
                fmt_u64(f.deletions)
            )),
            fname_w = max_fname,
        );
    });
    println!();
}

// ── Tree renderer ──────────────────────────────────────────────────────────────

struct TreeNode {
    children: BTreeMap<String, TreeNode>,
    is_file: bool,
}

fn build_tree(paths: &[PathBuf]) -> TreeNode {
    let mut root = TreeNode {
        children: BTreeMap::new(),
        is_file: false,
    };
    for path in paths {
        let components: Vec<String> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let total = components.len();
        let mut node = &mut root;
        for (i, name) in components.into_iter().enumerate() {
            node = node.children.entry(name).or_insert(TreeNode {
                children: BTreeMap::new(),
                is_file: i == total - 1,
            });
        }
    }
    root
}

fn print_tree(a: &Ansi, paths: &[PathBuf], root_name: &str) {
    let tree = build_tree(paths);
    println!("  {}/", a.bold(root_name));
    print_tree_children(a, &tree, "  ");
}

fn print_tree_children(a: &Ansi, node: &TreeNode, prefix: &str) {
    // Directories first (sorted), then files (sorted) — BTreeMap is already sorted.
    let dirs: Vec<(&String, &TreeNode)> =
        node.children.iter().filter(|(_, n)| !n.is_file).collect();
    let files: Vec<(&String, &TreeNode)> =
        node.children.iter().filter(|(_, n)| n.is_file).collect();

    let children: Vec<(&String, &TreeNode)> = dirs.into_iter().chain(files).collect();
    let last = children.len().saturating_sub(1);

    children.iter().enumerate().for_each(|(i, (name, child))| {
        let (connector, continuation) = if i == last {
            ("└── ", "    ")
        } else {
            ("├── ", "│   ")
        };
        if child.is_file {
            println!("{prefix}{connector}{name}");
        } else {
            println!("{prefix}{connector}{}/", a.bold(name));
            print_tree_children(a, child, &format!("{prefix}{continuation}"));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── format_number ──────────────────────────────────────────────────────────

    #[test]
    fn format_number_zero() {
        assert_eq!(format_number(0), "0");
    }

    #[test]
    fn format_number_small() {
        assert_eq!(format_number(42), "42");
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn format_number_thousands() {
        assert_eq!(format_number(1_000), "1,000");
        assert_eq!(format_number(12_345), "12,345");
    }

    #[test]
    fn format_number_millions() {
        assert_eq!(format_number(1_000_000), "1,000,000");
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

    #[test]
    fn fmt_u64_rounds_trip() {
        assert_eq!(fmt_u64(0), "0");
        assert_eq!(fmt_u64(1_000), "1,000");
        assert_eq!(fmt_u64(1_000_000), "1,000,000");
    }

    // ── build_tree ─────────────────────────────────────────────────────────────

    #[test]
    fn build_tree_empty() {
        let tree = build_tree(&[]);
        assert!(tree.children.is_empty());
        assert!(!tree.is_file);
    }

    #[test]
    fn build_tree_single_top_level_file() {
        let tree = build_tree(&[PathBuf::from("main.rs")]);
        assert_eq!(tree.children.len(), 1);
        let node = &tree.children["main.rs"];
        assert!(node.is_file);
        assert!(node.children.is_empty());
    }

    #[test]
    fn build_tree_nested_file() {
        let tree = build_tree(&[PathBuf::from("src/lib.rs")]);
        assert_eq!(tree.children.len(), 1);
        let src = &tree.children["src"];
        assert!(!src.is_file);
        assert_eq!(src.children.len(), 1);
        assert!(src.children["lib.rs"].is_file);
    }

    #[test]
    fn build_tree_dirs_and_files_separated() {
        let paths = vec![
            PathBuf::from("README.md"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/lib.rs"),
        ];
        let tree = build_tree(&paths);
        assert_eq!(tree.children.len(), 2);
        assert!(!tree.children["src"].is_file);
        assert!(tree.children["README.md"].is_file);
        assert_eq!(tree.children["src"].children.len(), 2);
    }

    #[test]
    fn build_tree_deeply_nested() {
        let paths = vec![PathBuf::from("a/b/c/deep.rs")];
        let tree = build_tree(&paths);
        let a = &tree.children["a"];
        assert!(!a.is_file);
        let b = &a.children["b"];
        assert!(!b.is_file);
        let c = &b.children["c"];
        assert!(!c.is_file);
        assert!(c.children["deep.rs"].is_file);
    }

    #[test]
    fn build_tree_sorted_alphabetically() {
        let paths = vec![
            PathBuf::from("z.rs"),
            PathBuf::from("a.rs"),
            PathBuf::from("m.rs"),
        ];
        let tree = build_tree(&paths);
        let keys: Vec<&String> = tree.children.keys().collect();
        assert_eq!(keys, vec!["a.rs", "m.rs", "z.rs"]);
    }

    #[test]
    fn build_tree_duplicate_paths_idempotent() {
        let paths = vec![PathBuf::from("src/main.rs"), PathBuf::from("src/main.rs")];
        let tree = build_tree(&paths);
        assert_eq!(tree.children["src"].children.len(), 1);
    }

    // ── preview::repo ─────────────────────────────────────────────────────────
    // Verify the full preview pipeline returns Ok on the same inputs as the PDF
    // pipeline.  Stdout output is not captured — Ok(()) is the meaningful contract.

    async fn git(dir: &std::path::Path, args: &[&str]) -> anyhow::Result<()> {
        let p = dir
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("non-utf8 path"))?;
        tokio::process::Command::new("git")
            .args(["-C", p])
            .args(args)
            .output()
            .await?;
        Ok(())
    }

    fn base_config(repo_path: std::path::PathBuf) -> crate::types::Config {
        crate::types::Config {
            repo_path,
            output_path: std::path::PathBuf::from("/tmp/unused.pdf"),
            include_patterns: vec![],
            exclude_patterns: vec![],
            theme: "InspiredGitHub".to_string(),
            font_size: 8.0,
            no_line_numbers: false,
            toc: true,
            file_tree: true,
            branch: None,
            commit: None,
            paper_size: crate::types::PaperSize::A4,
            landscape: false,
            remote_url: None,
        }
    }

    #[tokio::test]
    async fn preview_repo_git_repo() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        git(dir.path(), &["init", "-b", "main"]).await?;
        git(dir.path(), &["config", "user.email", "t@t.com"]).await?;
        git(dir.path(), &["config", "user.name", "T"]).await?;
        tokio::fs::write(dir.path().join("main.rs"), "fn main() {}\n").await?;
        git(dir.path(), &["add", "."]).await?;
        git(dir.path(), &["commit", "-m", "init"]).await?;

        crate::preview::repo(&base_config(dir.path().to_path_buf())).await
    }

    #[tokio::test]
    async fn preview_repo_plain_directory() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        tokio::fs::write(dir.path().join("hello.rs"), "fn main() {}\n").await?;

        crate::preview::repo(&base_config(dir.path().to_path_buf())).await
    }

    #[tokio::test]
    async fn preview_repo_empty_directory() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        crate::preview::repo(&base_config(dir.path().to_path_buf())).await
    }

    #[tokio::test]
    async fn preview_repo_single_file_plain() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        let file = dir.path().join("main.rs");
        tokio::fs::write(&file, "fn main() { println!(\"hi\"); }\n").await?;

        crate::preview::repo(&crate::types::Config {
            repo_path: file,
            ..base_config(dir.path().to_path_buf())
        })
        .await
    }

    #[tokio::test]
    async fn preview_repo_single_file_in_git_repo() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        git(dir.path(), &["init", "-b", "main"]).await?;
        git(dir.path(), &["config", "user.email", "t@t.com"]).await?;
        git(dir.path(), &["config", "user.name", "T"]).await?;
        tokio::fs::write(dir.path().join("lib.rs"), "pub fn f() {}\n").await?;
        git(dir.path(), &["add", "."]).await?;
        git(dir.path(), &["commit", "-m", "init"]).await?;

        crate::preview::repo(&crate::types::Config {
            repo_path: dir.path().join("lib.rs"),
            ..base_config(dir.path().to_path_buf())
        })
        .await
    }

    #[tokio::test]
    async fn preview_repo_no_file_tree() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        tokio::fs::write(dir.path().join("a.rs"), "fn a() {}\n").await?;

        crate::preview::repo(&crate::types::Config {
            file_tree: false,
            ..base_config(dir.path().to_path_buf())
        })
        .await
    }

    #[tokio::test]
    async fn preview_repo_include_filter() -> anyhow::Result<()> {
        let dir = TempDir::new()?;
        tokio::try_join!(
            tokio::fs::write(dir.path().join("main.rs"), "fn main() {}\n"),
            tokio::fs::write(dir.path().join("README.md"), "# hi\n"),
        )?;

        crate::preview::repo(&crate::types::Config {
            include_patterns: vec!["*.rs".to_string()],
            ..base_config(dir.path().to_path_buf())
        })
        .await
    }

    #[tokio::test]
    async fn preview_repo_nonexistent_path_errors() {
        assert!(
            crate::preview::repo(&base_config(std::path::PathBuf::from(
                "/nonexistent/preview/path"
            )))
            .await
            .is_err()
        );
    }
}

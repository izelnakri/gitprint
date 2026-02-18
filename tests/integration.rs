use std::path::{Path, PathBuf};

use tempfile::TempDir;

use gitprint::types::{Config, PaperSize};

async fn git_in(dir: &str, args: &[&str]) {
    let output = tokio::process::Command::new("git")
        .args(["-C", dir])
        .args(args)
        .output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn create_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let p = dir.path().to_str().unwrap().to_string();

    git_in(&p, &["init", "-b", "main"]).await;

    // git config writes must be sequential (both modify .git/config, git's file lock
    // is advisory and concurrent writes fail). File writes are independent so they
    // run in parallel with the sequential git config block.
    tokio::join!(
        async {
            git_in(&p, &["config", "user.email", "test@test.com"]).await;
            git_in(&p, &["config", "user.name", "Test"]).await;
        },
        async {
            tokio::try_join!(
                tokio::fs::write(
                    dir.path().join("main.rs"),
                    "fn main() {\n    println!(\"hello\");\n}\n",
                ),
                tokio::fs::write(
                    dir.path().join("lib.rs"),
                    "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
                ),
                tokio::fs::write(dir.path().join("README.md"), "# Test Repo\n"),
                tokio::fs::create_dir_all(dir.path().join("src")),
            )
            .unwrap();
            // src/ now exists; write util.rs after create_dir_all completes.
            tokio::fs::write(
                dir.path().join("src/util.rs"),
                "// utility\npub fn noop() {}\n",
            )
            .await
            .unwrap();
        },
    );

    git_in(&p, &["add", "."]).await;
    git_in(&p, &["commit", "-m", "initial commit"]).await;

    dir
}

fn test_config(repo_path: PathBuf, output_path: PathBuf) -> Config {
    Config {
        repo_path,
        output_path,
        include_patterns: vec![],
        exclude_patterns: vec![],
        theme: "InspiredGitHub".to_string(),
        font_size: 8.0,
        no_line_numbers: false,
        toc: true,
        file_tree: true,
        branch: None,
        commit: None,
        paper_size: PaperSize::A4,
        landscape: false,
        remote_url: None,
    }
}

// ── git module tests ──────────────────────────────────────────────

#[tokio::test]
async fn git_verify_repo_valid() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let info = gitprint::git::verify_repo(repo.path()).await?;
    assert!(info.is_git);
    assert!(info.scope.is_none());
    assert!(info.single_file.is_none());
    Ok(())
}

#[tokio::test]
async fn git_verify_repo_subdir() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let info = gitprint::git::verify_repo(&repo.path().join("src")).await?;
    assert!(info.is_git);
    assert_eq!(info.scope, Some(PathBuf::from("src")));
    assert!(info.single_file.is_none());
    Ok(())
}

#[tokio::test]
async fn git_verify_repo_single_file_in_git() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let info = gitprint::git::verify_repo(&repo.path().join("main.rs")).await?;
    assert!(info.is_git);
    assert_eq!(info.single_file, Some(PathBuf::from("main.rs")));
    assert!(info.scope.is_none());
    Ok(())
}

#[tokio::test]
async fn git_verify_repo_plain_directory() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let info = gitprint::git::verify_repo(dir.path()).await?;
    assert!(!info.is_git);
    assert!(info.single_file.is_none());
    Ok(())
}

#[tokio::test]
async fn git_verify_repo_plain_file() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    tokio::fs::write(dir.path().join("hello.rs"), "fn main() {}")
        .await
        .unwrap();
    let info = gitprint::git::verify_repo(&dir.path().join("hello.rs")).await?;
    assert!(!info.is_git);
    assert_eq!(info.single_file, Some(PathBuf::from("hello.rs")));
    Ok(())
}

#[tokio::test]
async fn git_verify_repo_nonexistent_path() {
    assert!(
        gitprint::git::verify_repo(Path::new("/nonexistent/path"))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn git_get_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let config = test_config(repo.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    let metadata = gitprint::git::get_metadata(repo.path(), &config, true, None).await?;

    assert!(!metadata.name.is_empty());
    assert_eq!(metadata.branch, "main");
    assert_eq!(metadata.commit_hash.len(), 40);
    assert!(metadata.commit_hash.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(metadata.commit_hash_short.len(), 7);
    assert_eq!(metadata.commit_message, "initial commit");
    assert!(!metadata.commit_date.is_empty());
    Ok(())
}

#[tokio::test]
async fn git_get_metadata_plain_directory() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    let config = test_config(dir.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    let metadata = gitprint::git::get_metadata(dir.path(), &config, false, None).await?;

    assert!(!metadata.name.is_empty());
    assert!(metadata.branch.is_empty());
    assert!(metadata.commit_hash.is_empty());
    assert!(metadata.commit_date.is_empty());
    Ok(())
}

#[tokio::test]
async fn git_get_metadata_with_branch() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let mut config = test_config(repo.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    config.branch = Some("main".to_string());
    let metadata = gitprint::git::get_metadata(repo.path(), &config, true, None).await?;
    assert_eq!(metadata.branch, "main");
    Ok(())
}

#[tokio::test]
async fn git_list_tracked_files() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let config = test_config(repo.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    let files = gitprint::git::list_tracked_files(repo.path(), &config, true, None).await?;

    assert!(files.contains(&PathBuf::from("main.rs")));
    assert!(files.contains(&PathBuf::from("lib.rs")));
    assert!(files.contains(&PathBuf::from("src/util.rs")));
    assert!(files.contains(&PathBuf::from("README.md")));
    assert_eq!(files.len(), 4);
    Ok(())
}

#[tokio::test]
async fn git_list_files_plain_directory() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    tokio::try_join!(
        tokio::fs::write(dir.path().join("hello.rs"), "fn main() {}"),
        tokio::fs::create_dir(dir.path().join("sub")),
    )
    .unwrap();
    tokio::fs::write(dir.path().join("sub/world.rs"), "pub fn world() {}")
        .await
        .unwrap();
    let config = test_config(dir.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    let files = gitprint::git::list_tracked_files(dir.path(), &config, false, None).await?;

    assert!(files.contains(&PathBuf::from("hello.rs")));
    assert!(files.contains(&PathBuf::from("sub/world.rs")));
    assert_eq!(files.len(), 2);
    Ok(())
}

#[tokio::test]
async fn git_read_file_content() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let config = test_config(repo.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    let content =
        gitprint::git::read_file_content(repo.path(), Path::new("main.rs"), &config).await?;

    assert!(content.contains("fn main()"));
    assert!(content.contains("println!"));
    Ok(())
}

#[tokio::test]
async fn git_read_file_content_nonexistent() {
    let repo = create_test_repo().await;
    let config = test_config(repo.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    let result =
        gitprint::git::read_file_content(repo.path(), Path::new("nonexistent.rs"), &config).await;
    assert!(result.is_err());
}

// ── full pipeline tests ───────────────────────────────────────────

#[tokio::test]
async fn full_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let config = test_config(repo.path().to_path_buf(), output_path.clone());

    gitprint::run(&config).await?;

    assert!(output_path.exists());
    assert!(std::fs::metadata(&output_path)?.len() > 0);
    Ok(())
}

#[tokio::test]
async fn full_pipeline_with_include_filter() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.include_patterns = vec!["*.rs".to_string()];

    gitprint::run(&config).await?;

    assert!(output_path.exists());
    assert!(std::fs::metadata(&output_path)?.len() > 0);
    Ok(())
}

#[tokio::test]
async fn full_pipeline_with_exclude_filter() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.exclude_patterns = vec!["*.md".to_string()];

    gitprint::run(&config).await?;
    assert!(output_path.exists());
    Ok(())
}

#[tokio::test]
async fn full_pipeline_no_toc_no_tree() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.toc = false;
    config.file_tree = false;

    gitprint::run(&config).await?;
    assert!(output_path.exists());
    Ok(())
}

#[tokio::test]
async fn full_pipeline_no_line_numbers() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.no_line_numbers = true;

    gitprint::run(&config).await?;
    assert!(output_path.exists());
    Ok(())
}

#[tokio::test]
async fn full_pipeline_landscape() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.landscape = true;

    gitprint::run(&config).await?;
    assert!(output_path.exists());
    Ok(())
}

#[tokio::test]
async fn full_pipeline_letter_paper() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.paper_size = PaperSize::Letter;

    gitprint::run(&config).await?;
    assert!(output_path.exists());
    Ok(())
}

#[tokio::test]
async fn full_pipeline_subdir() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let config = test_config(repo.path().join("src"), output_path.clone());

    gitprint::run(&config).await?;

    assert!(output_path.exists());
    assert!(std::fs::metadata(&output_path)?.len() > 0);
    Ok(())
}

#[tokio::test]
async fn full_pipeline_single_file() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let config = test_config(repo.path().join("main.rs"), output_path.clone());

    gitprint::run(&config).await?;

    assert!(output_path.exists());
    assert!(std::fs::metadata(&output_path)?.len() > 0);
    Ok(())
}

#[tokio::test]
async fn full_pipeline_plain_directory() -> Result<(), Box<dyn std::error::Error>> {
    let dir = TempDir::new()?;
    tokio::try_join!(
        tokio::fs::write(dir.path().join("main.rs"), "fn main() {}\n"),
        tokio::fs::write(
            dir.path().join("lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
        ),
    )
    .unwrap();
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let config = test_config(dir.path().to_path_buf(), output_path.clone());

    gitprint::run(&config).await?;

    assert!(output_path.exists());
    assert!(std::fs::metadata(&output_path)?.len() > 0);
    Ok(())
}

#[tokio::test]
async fn full_pipeline_nonexistent_repo() {
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let config = test_config(PathBuf::from("/nonexistent/repo"), output_path);

    assert!(gitprint::run(&config).await.is_err());
}

#[tokio::test]
async fn full_pipeline_invalid_theme() {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path);
    config.theme = "NonExistentTheme".to_string();

    let err = gitprint::run(&config).await.unwrap_err();
    assert!(err.to_string().contains("NonExistentTheme"));
    assert!(err.to_string().contains("--list-themes"));
}

#[tokio::test]
async fn full_pipeline_include_excludes_everything() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.include_patterns = vec!["*.nonexistent".to_string()];

    gitprint::run(&config).await?;
    assert!(output_path.exists());
    Ok(())
}

#[tokio::test]
async fn full_pipeline_custom_font_size() -> Result<(), Box<dyn std::error::Error>> {
    let repo = create_test_repo().await;
    let out_dir = TempDir::new()?;
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.font_size = 12.0;

    gitprint::run(&config).await?;
    assert!(output_path.exists());
    Ok(())
}

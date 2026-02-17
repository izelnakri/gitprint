use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use gitprint::error::Error;
use gitprint::types::{Config, PaperSize};

/// Create a temporary git repository with a few committed files.
fn create_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let p = dir.path().to_str().unwrap();

    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(["-C", p])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    };

    git(&["init", "-b", "main"]);
    git(&["config", "user.email", "test@test.com"]);
    git(&["config", "user.name", "Test"]);

    std::fs::write(
        dir.path().join("main.rs"),
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/util.rs"),
        "// utility\npub fn noop() {}\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("README.md"), "# Test Repo\n").unwrap();

    git(&["add", "."]);
    git(&["commit", "-m", "initial commit"]);

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
    }
}

// ── git module tests ──────────────────────────────────────────────

#[test]
fn git_verify_repo_valid() {
    let repo = create_test_repo();
    assert!(gitprint::git::verify_repo(repo.path()).is_ok());
}

#[test]
fn git_verify_repo_not_a_repo() {
    let dir = TempDir::new().unwrap();
    let result = gitprint::git::verify_repo(dir.path());
    assert!(result.is_err());
}

#[test]
fn git_verify_repo_nonexistent_path() {
    let result = gitprint::git::verify_repo(Path::new("/nonexistent/path"));
    assert!(result.is_err());
}

#[test]
fn git_get_metadata() {
    let repo = create_test_repo();
    let config = test_config(repo.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    let metadata = gitprint::git::get_metadata(repo.path(), &config).unwrap();

    assert!(!metadata.name.is_empty());
    assert_eq!(metadata.branch, "main");
    assert_eq!(metadata.commit_hash.len(), 40);
    assert!(metadata.commit_hash.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(metadata.commit_hash_short.len(), 7);
    assert_eq!(metadata.commit_message, "initial commit");
    assert!(!metadata.commit_date.is_empty());
}

#[test]
fn git_get_metadata_with_branch() {
    let repo = create_test_repo();
    let mut config = test_config(repo.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    config.branch = Some("main".to_string());
    let metadata = gitprint::git::get_metadata(repo.path(), &config).unwrap();
    assert_eq!(metadata.branch, "main");
}

#[test]
fn git_list_tracked_files() {
    let repo = create_test_repo();
    let config = test_config(repo.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    let files = gitprint::git::list_tracked_files(repo.path(), &config).unwrap();

    assert!(files.contains(&PathBuf::from("main.rs")));
    assert!(files.contains(&PathBuf::from("lib.rs")));
    assert!(files.contains(&PathBuf::from("src/util.rs")));
    assert!(files.contains(&PathBuf::from("README.md")));
    assert_eq!(files.len(), 4);
}

#[test]
fn git_read_file_content() {
    let repo = create_test_repo();
    let config = test_config(repo.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    let content =
        gitprint::git::read_file_content(repo.path(), Path::new("main.rs"), &config).unwrap();

    assert!(content.contains("fn main()"));
    assert!(content.contains("println!"));
}

#[test]
fn git_read_file_content_nonexistent() {
    let repo = create_test_repo();
    let config = test_config(repo.path().to_path_buf(), PathBuf::from("/tmp/test.pdf"));
    let result =
        gitprint::git::read_file_content(repo.path(), Path::new("nonexistent.rs"), &config);
    assert!(result.is_err());
}

// ── full pipeline tests ───────────────────────────────────────────

#[test]
fn full_pipeline() {
    let repo = create_test_repo();
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let config = test_config(repo.path().to_path_buf(), output_path.clone());

    gitprint::run(&config).unwrap();

    assert!(output_path.exists());
    assert!(std::fs::metadata(&output_path).unwrap().len() > 0);
}

#[test]
fn full_pipeline_with_include_filter() {
    let repo = create_test_repo();
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.include_patterns = vec!["*.rs".to_string()];

    gitprint::run(&config).unwrap();

    assert!(output_path.exists());
    assert!(std::fs::metadata(&output_path).unwrap().len() > 0);
}

#[test]
fn full_pipeline_with_exclude_filter() {
    let repo = create_test_repo();
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.exclude_patterns = vec!["*.md".to_string()];

    gitprint::run(&config).unwrap();

    assert!(output_path.exists());
}

#[test]
fn full_pipeline_no_toc_no_tree() {
    let repo = create_test_repo();
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.toc = false;
    config.file_tree = false;

    gitprint::run(&config).unwrap();

    assert!(output_path.exists());
}

#[test]
fn full_pipeline_no_line_numbers() {
    let repo = create_test_repo();
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.no_line_numbers = true;

    gitprint::run(&config).unwrap();

    assert!(output_path.exists());
}

#[test]
fn full_pipeline_landscape() {
    let repo = create_test_repo();
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.landscape = true;

    gitprint::run(&config).unwrap();

    assert!(output_path.exists());
}

#[test]
fn full_pipeline_letter_paper() {
    let repo = create_test_repo();
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.paper_size = PaperSize::Letter;

    gitprint::run(&config).unwrap();

    assert!(output_path.exists());
}

#[test]
fn full_pipeline_nonexistent_repo() {
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let config = test_config(PathBuf::from("/nonexistent/repo"), output_path);

    let result = gitprint::run(&config);
    assert!(result.is_err());
}

#[test]
fn full_pipeline_invalid_theme() {
    let repo = create_test_repo();
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path);
    config.theme = "NonExistentTheme".to_string();

    let result = gitprint::run(&config);
    assert!(matches!(result, Err(Error::ThemeNotFound(_))));
}

#[test]
fn full_pipeline_include_excludes_everything() {
    let repo = create_test_repo();
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.include_patterns = vec!["*.nonexistent".to_string()];

    // Should still succeed, just with 0 files
    gitprint::run(&config).unwrap();
    assert!(output_path.exists());
}

#[test]
fn full_pipeline_custom_font_size() {
    let repo = create_test_repo();
    let out_dir = TempDir::new().unwrap();
    let output_path = out_dir.path().join("output.pdf");
    let mut config = test_config(repo.path().to_path_buf(), output_path.clone());
    config.font_size = 12.0;

    gitprint::run(&config).unwrap();

    assert!(output_path.exists());
}

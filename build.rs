fn main() {
    // Rerun whenever the checked-out branch changes.
    println!("cargo:rerun-if-changed=.git/HEAD");

    // Only embed the branch name in debug builds so `cargo run` shows the
    // current branch while release binaries show the Cargo.toml version.
    if std::env::var("PROFILE").as_deref() != Ok("debug") {
        return;
    }

    let branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "HEAD"); // HEAD = detached, skip

    if let Some(branch) = branch {
        println!("cargo:rustc-env=GITPRINT_GIT_BRANCH={branch}");
    }
}

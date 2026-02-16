use std::path::PathBuf;

use clap::Parser;

fn main() {
    let args = gitprint::cli::Args::parse();

    if args.list_themes {
        gitprint::highlight::list_themes()
            .iter()
            .for_each(|t| println!("  {t}"));
        return;
    }

    let output_path = args.output.unwrap_or_else(|| {
        let name = args
            .path
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "output".to_string());
        PathBuf::from(format!("{name}.pdf"))
    });

    let config = gitprint::types::Config {
        repo_path: args.path,
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
    };

    if let Err(e) = gitprint::run(&config) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

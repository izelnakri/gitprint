use std::path::Path;

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use gitprint::filter::FileFilter;
use gitprint::highlight::Highlighter;
use gitprint::types::HighlightedLine;

const SAMPLE_RUST: &str = r#"
use std::collections::HashMap;

fn main() {
    let mut map = HashMap::new();
    map.insert("key", 42);

    for (k, v) in &map {
        println!("{k}: {v}");
    }

    let result: Vec<_> = (0..100)
        .filter(|n| n % 2 == 0)
        .map(|n| n * n)
        .collect();

    println!("{result:?}");
}
"#;

fn bench_highlight(c: &mut Criterion) {
    let highlighter = Highlighter::new("InspiredGitHub").unwrap();
    let path = Path::new("sample.rs");

    c.bench_function("highlight_rust_file", |b| {
        b.iter(|| {
            let lines: Vec<HighlightedLine> = highlighter
                .highlight_lines(black_box(SAMPLE_RUST), path)
                .collect();
            black_box(lines);
        });
    });

    let large_content = SAMPLE_RUST.repeat(50);
    c.bench_function("highlight_large_file", |b| {
        b.iter(|| {
            let lines: Vec<HighlightedLine> = highlighter
                .highlight_lines(black_box(&large_content), path)
                .collect();
            black_box(lines);
        });
    });
}

fn bench_filter(c: &mut Criterion) {
    let paths: Vec<std::path::PathBuf> = (0..1000)
        .flat_map(|i| {
            vec![
                format!("src/module_{i}/mod.rs").into(),
                format!("src/module_{i}/test.rs").into(),
                format!("docs/page_{i}.md").into(),
                format!("assets/image_{i}.png").into(),
                format!("node_modules/pkg_{i}/index.js").into(),
            ]
        })
        .collect();

    c.bench_function("filter_5000_paths", |b| {
        b.iter(|| {
            let filter = FileFilter::new(&["*.rs".to_string()], &["*test*".to_string()]).unwrap();
            let filtered: Vec<_> = filter.filter_paths(black_box(paths.clone())).collect();
            black_box(filtered);
        });
    });
}

fn bench_highlighter_creation(c: &mut Criterion) {
    c.bench_function("highlighter_new", |b| {
        b.iter(|| {
            black_box(Highlighter::new("InspiredGitHub").unwrap());
        });
    });
}

criterion_group!(
    benches,
    bench_highlight,
    bench_filter,
    bench_highlighter_creation
);
criterion_main!(benches);

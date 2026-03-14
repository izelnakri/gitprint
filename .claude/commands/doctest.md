# doctest

When writing or modifying public functions, methods, or types in this codebase:

1. Every `pub fn`, `pub async fn`, and `pub` method must have a `///` doc comment describing what it does.
2. Every such doc comment must include a `# Examples` section with a working ```` ```rust ```` doctest that compiles and passes via `cargo test --doc`.
3. When editing an existing public item that already has a doctest, keep it — update it only if the signature or behaviour changed.
4. Doctests must be self-contained: import everything they need, construct any required inputs inline, and assert an observable output or side-effect.

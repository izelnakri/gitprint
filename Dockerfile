# cargo-chef is pre-installed in this image — no source compilation needed.
# 0.1.75-rust-alpine3.23 is an immutable tag (pinned cargo-chef + Alpine version)
# so the chef and deps layers are never invalidated by a base image update.
# Bump the tag deliberately when upgrading Rust or cargo-chef.
FROM lukemathwalker/cargo-chef:0.1.75-rust-alpine3.23 AS chef
RUN apk add --no-cache git
WORKDIR /app

# Planner: fast — inspects Cargo.toml/Cargo.lock and emits recipe.json.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage 1 — dependencies only.
# Cached until recipe.json changes (i.e. Cargo.lock / Cargo.toml change).
# LTO and codegen-units are overridden here for Docker: fat LTO + CU=1 would
# cause 3+ min of link time on every run (it cannot be Docker-layer-cached).
# Thin LTO + CU=16 (Cargo default) cuts the link step to ~20 s with no measurable size
# difference for a nightly image. The release binaries shipped to users still
# use fat LTO via Cargo.toml — only the Docker image uses these overrides.
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN CARGO_PROFILE_RELEASE_LTO=thin \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
    cargo chef cook --release --recipe-path recipe.json

# Builder stage 2 — gitprint source only (~20-30 s on every push).
COPY . .
RUN CARGO_PROFILE_RELEASE_LTO=thin \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
    cargo build --release

# Pre-built stage: copies an already-compiled musl binary from the build context.
# Used by release.yml with --target prebuilt and context pointing at the extracted
# artifact directory — skips all compilation stages entirely.
FROM alpine:latest AS prebuilt
RUN apk add --no-cache git
COPY gitprint /usr/local/bin/gitprint
ENTRYPOINT ["gitprint"]

# Default stage: compiled from source via the builder above.
FROM alpine:latest
RUN apk add --no-cache git
COPY --from=builder /app/target/release/gitprint /usr/local/bin/gitprint
ENTRYPOINT ["gitprint"]

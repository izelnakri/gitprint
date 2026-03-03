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
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Builder stage 2 — gitprint source only (~30-60 s on every push).
COPY . .
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache git
COPY --from=builder /app/target/release/gitprint /usr/local/bin/gitprint
ENTRYPOINT ["gitprint"]

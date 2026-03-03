# Pin the Rust version so the chef/deps layers are never invalidated by a
# rust:alpine:latest update. Bump this deliberately when upgrading Rust.
FROM rust:1.88.0-alpine AS chef
RUN apk add --no-cache git && cargo install cargo-chef --locked
WORKDIR /app

# Planner: inspect the full workspace and emit a recipe.json that captures
# exactly which crates (and features) need to be compiled. Runs on every push
# but is trivially fast — no compilation happens here.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage 1 — dependencies only.
# recipe.json changes only when Cargo.toml / Cargo.lock change, so this layer
# is almost always restored from Docker GHA cache (~5 s). It rebuilds (~5 min)
# only when you add/remove/update a dependency.
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Builder stage 2 — gitprint source only.
# Runs on every push but only recompiles gitprint itself (~30-60 s).
COPY . .
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache git
COPY --from=builder /app/target/release/gitprint /usr/local/bin/gitprint
ENTRYPOINT ["gitprint"]

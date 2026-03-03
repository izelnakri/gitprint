FROM rust:alpine AS builder
RUN apk add --no-cache git
WORKDIR /app

# Compile all dependencies in a separate layer so they are cached by Docker's
# GHA layer cache as long as Cargo.toml / Cargo.lock / build.rs don't change.
# Empty stubs for src/main.rs and src/lib.rs satisfy cargo without pulling in
# any project source — all external crates still get fully compiled.
COPY Cargo.toml Cargo.lock build.rs ./
RUN mkdir -p src benches && \
    printf 'fn main() {}\n' > src/main.rs && \
    touch src/lib.rs benches/pipeline.rs && \
    cargo build --release && \
    cargo clean -p gitprint

# Build the real binary. Only this layer re-runs on source changes.
COPY . .
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache git
COPY --from=builder /app/target/release/gitprint /usr/local/bin/gitprint
ENTRYPOINT ["gitprint"]

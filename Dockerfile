# Stage 1: compile
# rust:alpine uses musl libc by default — produces a fully static binary.
FROM rust:alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY . .
RUN cargo build --release

# Stage 2: minimal runtime
# git is required because gitprint shells out to the git CLI.
FROM alpine:latest
RUN apk add --no-cache git
COPY --from=builder /app/target/release/gitprint /usr/local/bin/gitprint
ENTRYPOINT ["gitprint"]

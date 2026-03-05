# ── Build stage ────────────────────────────────────────────────────────────────
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y musl-tools && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build

# Cache dependencies before copying source
COPY Cargo.toml Cargo.lock ./
COPY crates/config/Cargo.toml  crates/config/Cargo.toml
COPY crates/openapi/Cargo.toml crates/openapi/Cargo.toml
COPY crates/client/Cargo.toml  crates/client/Cargo.toml
COPY crates/tui/Cargo.toml     crates/tui/Cargo.toml

RUN for dir in crates/config crates/openapi crates/client; do \
      mkdir -p $dir/src && echo "pub fn _stub() {}" > $dir/src/lib.rs; \
    done && \
    mkdir -p crates/tui/src && echo "fn main() {}" > crates/tui/src/main.rs

RUN cargo build --release --bin oat --target x86_64-unknown-linux-musl 2>/dev/null; true

COPY crates crates
RUN touch crates/*/src/*.rs && \
    cargo build --release --bin oat --target x86_64-unknown-linux-musl

# ── Runtime stage (UBI9) ───────────────────────────────────────────────────────
FROM registry.access.redhat.com/ubi9/ubi-minimal

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/oat /usr/local/bin/oat

ENTRYPOINT ["/usr/local/bin/oat"]

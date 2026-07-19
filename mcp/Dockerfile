# Builds the `svix-mcp` binary and packages it as a minimal image that serves
# both MCP servers over HTTP (see main.rs / README.md).

# ---- Builder
# Pin to the same Rust toolchain CI uses (see .github/workflows/mcp-ci.yml).
FROM rust:1.97.1-slim-trixie AS builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src


RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked && \
    strip target/release/svix-mcp && \
    cp target/release/svix-mcp /svix-mcp

# ---- Runtime
FROM debian:trixie-slim AS runtime

# ca-certificates for rustls; curl for the health check; tini for a proper PID 1 init.
RUN --mount=target=/var/lib/apt/lists,type=cache,sharing=locked \
    --mount=target=/var/cache/apt,type=cache,sharing=locked <<EOF
set -eux
apt-get update
apt-get install -y --no-install-recommends ca-certificates curl tini
EOF

# Run as an unprivileged user.
RUN useradd --system --uid 10001 --user-group --home-dir /nonexistent \
    --shell /usr/sbin/nologin svix
USER svix

COPY --from=builder /svix-mcp /usr/local/bin/svix-mcp


ENV MCP_TRANSPORT=http \
    MCP_BIND_ADDR=0.0.0.0:8080
EXPOSE 8080

# Although this is going to return 401, it's fine to see if the server is up
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -sS -o /dev/null http://127.0.0.1:8080/ingest || exit 1

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/bin/svix-mcp"]

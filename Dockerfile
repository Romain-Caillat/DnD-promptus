# syntax=docker/dockerfile:1
#
# Promptus production image: the Axum server and the built front in one
# image, one process, one origin (docs/install.md). TLS is terminated in
# front of it (the server's reverse proxy), so it speaks plain HTTP.

# ---------------------------------------------------------------- front
FROM oven/bun:1 AS front-builder

WORKDIR /app/front
COPY front/package.json front/bun.lock ./
RUN bun install --frozen-lockfile
COPY front/ ./
RUN bun run build

# ---------------------------------------------------------------- server
FROM rust:1-slim-trixie AS server-builder

# webauthn-rs (GM passkeys) verifies signatures with OpenSSL.
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY shared/ shared/
COPY back/ back/
# The starter sprite packs and looks are compiled into the server.
COPY content/sprites/ content/sprites/
# The desktop shell is a workspace member but has no place in the server
# image: drop it so cargo neither looks for its sources nor builds Tauri.
# rust-toolchain.toml is not copied on purpose: the image's stable
# toolchain builds the server, without downloading clippy and rustfmt.
RUN sed -i 's/, "src-tauri"//' Cargo.toml
# Queries checked at compile time read the committed `.sqlx` metadata,
# never a live database, inside the build.
ENV SQLX_OFFLINE=true
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --package promptus_back \
    && cp target/release/promptus_back /usr/local/bin/promptus-server

# ---------------------------------------------------------------- runtime
FROM debian:trixie-slim

# curl is the health check's only dependency (HEALTHCHECK below);
# libssl3t64 is OpenSSL for the passkey verification.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl libssl3t64 \
    && rm -rf /var/lib/apt/lists/*

# An unprivileged user with a fixed uid, so a host folder mounted later
# (media) can be handed to it once.
RUN groupadd --system --gid 10001 promptus \
    && useradd --system --uid 10001 --gid promptus \
        --home-dir /var/lib/promptus --create-home --shell /usr/sbin/nologin promptus

COPY --from=server-builder /usr/local/bin/promptus-server /usr/local/bin/promptus-server
COPY --from=front-builder /app/front/dist /srv/front

WORKDIR /var/lib/promptus
USER promptus:promptus

ENV PORT=4333 \
    FRONT_DIR=/srv/front \
    RUST_LOG=info

EXPOSE 4333

# 200 only when the server answers *and* reaches its database.
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:${PORT}/api/health" > /dev/null || exit 1

CMD ["promptus-server"]

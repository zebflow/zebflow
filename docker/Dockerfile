# syntax=docker/dockerfile:1
# zebflow platform
# Build:  docker build -f docker/Dockerfile -t zebflow:local .
# Run:    docker run --rm -p 10610:10610 -e ZEBFLOW_PLATFORM_DEFAULT_PASSWORD=... zebflow:local
# Data:   -v zebflow-data:/var/lib/zebflow/data

# ── Stage 1: install cargo-chef ────────────────────────────────────────────
FROM rust:bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

# ── Stage 2: dependency planner ────────────────────────────────────────────
# Reads the full source tree to produce recipe.json (the dep manifest).
# Only re-runs if Cargo.toml / Cargo.lock / build.rs / libraries change.
FROM chef AS planner
COPY Cargo.toml Cargo.lock build.rs ./
COPY libraries ./libraries
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 3: build dependencies (cached) + app ────────────────────────────
FROM chef AS builder
COPY libraries ./libraries
COPY --from=planner /app/recipe.json recipe.json

# Cook: builds ALL dependencies only.
# This layer is cached as long as recipe.json (= Cargo.lock) is unchanged.
# deno_core / V8 / tokio / axum etc. only recompile when deps change.
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
# Copy build-script asset directories BEFORE cook so build.rs (which runs during
# cook) can embed them into the binary. Without this, src/platform/help/ and
# src/platform/web/templates/ don't exist when build.rs runs, producing empty
# HELP and TEMPLATE arrays that are then cached and never corrected.
COPY src/platform/help ./src/platform/help
COPY src/platform/web/templates ./src/platform/web/templates
RUN cargo chef cook --release --recipe-path recipe.json

# Now copy source and build only the application code (fast after cache hit).
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
ARG BUILD_VERSION
RUN BUILD_VERSION=${BUILD_VERSION} cargo build --release --bin zebflow_platform

# ── Stage 4: runtime image ────────────────────────────────────────────────
FROM dhi.io/debian-base:trixie-debian13-dev

USER root
RUN apt-get update && apt-get install -y --no-install-recommends \
        libssl3t64 \
        git \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /var/lib/zebflow/data \
    && chown -R nonroot:nonroot /var/lib/zebflow

COPY --from=builder --chmod=755 /app/target/release/zebflow_platform /usr/local/bin/zebflow_platform

USER nonroot
WORKDIR /home/nonroot

ENV ZEBFLOW_PLATFORM_DATA_DIR=/var/lib/zebflow/data \
    ZEBFLOW_PLATFORM_HOST=0.0.0.0 \
    ZEBFLOW_PLATFORM_PORT=10610

EXPOSE 10610

ENTRYPOINT ["/usr/local/bin/zebflow_platform"]

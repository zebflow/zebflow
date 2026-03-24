# syntax=docker/dockerfile:1
# zebflow platform — runtime: dhi.io/debian-base:trixie-debian13-dev
# Authenticate: docker login dhi.io
# Build:  docker build -f docker/Dockerfile -t zebflow:local .
# Run:    docker run --rm -p 10610:10610 -e ZEBFLOW_PLATFORM_DEFAULT_PASSWORD=... zebflow:local
# Data:   -v zebflow-data:/var/lib/zebflow/data

# Latest stable on Bookworm (deps e.g. oxc require rustc ≥ 1.91).
FROM rust:bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock build.rs ./
COPY libraries ./libraries
COPY src ./src

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN cargo build --release --bin zebflow_platform

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

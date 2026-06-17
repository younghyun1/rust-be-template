#!/bin/sh
set -eu

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required for the glibc znver3 build but was not found in PATH" >&2
  exit 1
fi

if ! command -v zstd >/dev/null 2>&1; then
  echo "zstd is required for frontend asset compression but was not found in PATH" >&2
  exit 1
fi

if ! command -v gzip >/dev/null 2>&1; then
  echo "gzip is required for frontend asset compression but was not found in PATH" >&2
  exit 1
fi

APP_NAME="${APP_NAME:-rust-be-template}"
TARGET_TRIPLE="${TARGET_TRIPLE:-x86_64-unknown-linux-gnu}"
RUST_DOCKER_TAG="${RUST_DOCKER_TAG:-latest}"
BUILDER_IMAGE="${BUILDER_IMAGE:-rust-be-template-znver3-builder:${RUST_DOCKER_TAG}}"
DOCKER_PLATFORM="${DOCKER_PLATFORM:-linux/amd64}"
BUILDER_CONTEXT="$(mktemp -d)"

cleanup_builder_context() {
  rm -rf "$BUILDER_CONTEXT"
}
trap cleanup_builder_context EXIT

git pull --ff-only
cd ../solid-csr-spa-template/
git pull --ff-only
npm update
npm install
./deploy_to_be.sh
cd ../rust-be-template/

CACHEBUST="$(date +%s)"

docker build \
  --platform "$DOCKER_PLATFORM" \
  --build-arg RUST_DOCKER_TAG="$RUST_DOCKER_TAG" \
  --build-arg TARGET_TRIPLE="$TARGET_TRIPLE" \
  --build-arg CACHEBUST="$CACHEBUST" \
  --pull \
  -t "$BUILDER_IMAGE" \
  -f - "$BUILDER_CONTEXT" <<'DOCKERFILE'
ARG RUST_DOCKER_TAG=latest
FROM rust:${RUST_DOCKER_TAG}

ARG TARGET_TRIPLE=x86_64-unknown-linux-gnu

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      ca-certificates \
      clang \
      git \
      gzip \
      libpq-dev \
      libzstd-dev \
      make \
      mold \
      perl \
      pkg-config \
      zstd && \
    rm -rf /var/lib/apt/lists/*

# Cache-bust this layer so the nightly toolchain is re-fetched on every build.
# Docker would otherwise cache the RUN below and pin whatever nightly existed
# when the layer was first built (the reason builds kept shipping an old
# nightly). build.sh passes a fresh CACHEBUST value per invocation; referencing
# it in the RUN forces the layer to re-run, and `rustup update` then pulls the
# latest stable + nightly.
ARG CACHEBUST=0
RUN echo "rust toolchain refresh: ${CACHEBUST}" && \
    rustup update && \
    rustup toolchain install nightly --component rust-src && \
    rustup update nightly && \
    rustup target add --toolchain nightly "${TARGET_TRIPLE}"

WORKDIR /app
DOCKERFILE

docker run --rm -i \
  --platform "$DOCKER_PLATFORM" \
  -e APP_NAME="$APP_NAME" \
  -e TARGET_TRIPLE="$TARGET_TRIPLE" \
  -e HOST_UID="$(id -u)" \
  -e HOST_GID="$(id -g)" \
  -v "$PWD":/app \
  -w /app \
  "$BUILDER_IMAGE" \
  /bin/sh -s <<'DOCKER_SCRIPT'
set -eu

cleanup() {
  chown -R "$HOST_UID:$HOST_GID" /app/target /app/Cargo.toml /app/Cargo.lock /app/src/build_info.rs 2>/dev/null || true
}
trap cleanup EXIT

cargo update

RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C target-cpu=znver3" \
cargo +nightly build \
  -Z build-std=std,core,alloc,panic_unwind \
  --target "$TARGET_TRIPLE" \
  --release

ldd "/app/target/$TARGET_TRIPLE/release/$APP_NAME"
DOCKER_SCRIPT

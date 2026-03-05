#!/bin/sh
set -eu

if ! command -v zstd >/dev/null 2>&1; then
  echo "zstd is required for frontend asset compression but was not found in PATH" >&2
  exit 1
fi

if ! command -v gzip >/dev/null 2>&1; then
  echo "gzip is required for frontend asset compression but was not found in PATH" >&2
  exit 1
fi

git reset --hard && git pull
cd ../solid-csr-spa-template/
git reset --hard && git pull
npm update
npm install
./deploy_to_be.sh
cd ../rust-be-template/
rustup update
rustup target add x86_64-unknown-linux-musl
cargo upgrade --incompatible
cargo update
cargo +nightly build -Z build-std=std,core,alloc,panic_unwind --target x86_64-unknown-linux-musl --release

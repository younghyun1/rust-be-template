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

MUSL_CC=""
if command -v x86_64-linux-musl-gcc >/dev/null 2>&1; then
  MUSL_CC="x86_64-linux-musl-gcc"
elif command -v musl-gcc >/dev/null 2>&1; then
  MUSL_CC="musl-gcc"
else
  echo "musl toolchain is required but no musl C compiler was found (x86_64-linux-musl-gcc or musl-gcc)" >&2
  echo "Install package 'musl' and re-run this script." >&2
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
CC_x86_64_unknown_linux_musl="$MUSL_CC" \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER="$MUSL_CC" \
cargo +nightly build -Z build-std=std,core,alloc,panic_unwind --target x86_64-unknown-linux-musl --release

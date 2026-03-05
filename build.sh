#!/bin/sh
set -eu

is_arch_like() {
  if [ -r /etc/os-release ]; then
    if grep -Eqi '^(ID|ID_LIKE)=(.*arch|.*cachyos)' /etc/os-release; then
      return 0
    fi
  fi
  return 1
}

ensure_pacman_pkg() {
  pkg="$1"
  if pacman -Q "$pkg" >/dev/null 2>&1; then
    return 0
  fi

  if command -v sudo >/dev/null 2>&1; then
    sudo pacman -S --needed --noconfirm "$pkg"
    return 0
  fi

  echo "Missing package '$pkg'. Install it with: pacman -S --needed $pkg" >&2
  exit 1
}

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required but was not found in PATH" >&2
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

if is_arch_like; then
  if ! command -v x86_64-linux-musl-gcc >/dev/null 2>&1 && ! command -v musl-gcc >/dev/null 2>&1; then
    ensure_pacman_pkg musl
  fi
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
rustup toolchain install nightly
rustup target add --toolchain nightly x86_64-unknown-linux-musl
cargo upgrade --incompatible
cargo update
CC_x86_64_unknown_linux_musl="$MUSL_CC" \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C link-arg=-lgcc_eh" \
cargo +nightly build -Z build-std=std,core,alloc,panic_unwind --target x86_64-unknown-linux-musl --release

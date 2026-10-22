#!/bin/sh
# Builds clawd.nro inside devkitPro's devkitA64 image (as CI does): installs the Rust toolchain the Makefile names,
# then runs make. The version shows in the Homebrew Menu.
#   docker run --rm -v "$PWD":/src -w /src/clawd-3d/apps/switch devkitpro/devkita64 ./ci-build.sh 1.23.0
set -eu

VERSION="${1:-1.0.0}"
cd "$(dirname "$0")"
TOOLCHAIN=$(sed -n 's/^RUST_TOOLCHAIN[[:space:]]*?=[[:space:]]*//p' Makefile)

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
fi
PATH="$HOME/.cargo/bin:$PATH"
export PATH
# rust-src: the Makefile rebuilds core and alloc position-independent (-Z build-std).
rustup toolchain install "$TOOLCHAIN" --profile minimal --component rust-src

make APP_VERSION="$VERSION"
ls -l clawd.nro

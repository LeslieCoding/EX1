#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
REAL_HOME=$(getent passwd "$(id -un)" | cut -d: -f6)

export HOME="$REAL_HOME"
export PATH="$ROOT/.tooling/node/bin:$REAL_HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
export XDG_CACHE_HOME="$ROOT/.cache"
export COREPACK_HOME="$ROOT/.tooling/corepack"
export PNPM_HOME="$ROOT/.tooling/pnpm"
export CARGO_HOME="$ROOT/.tooling/cargo"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-1.95.0-x86_64-unknown-linux-gnu}"

if [ -z "${RUSTUP_HOME:-}" ]; then
  if [ -d "$REAL_HOME/.rustup" ]; then
    export RUSTUP_HOME="$REAL_HOME/.rustup"
  elif [ -d "$REAL_HOME/Kimi/.rustup" ]; then
    export RUSTUP_HOME="$REAL_HOME/Kimi/.rustup"
  fi
fi

cd "$ROOT"
corepack pnpm web:build
corepack pnpm tauri build --config src-tauri/tauri.linux.conf.json

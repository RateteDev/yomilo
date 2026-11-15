#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
crate_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd -- "$crate_dir/../.." && pwd)"
cd -- "$root"

cargo run -p yomilo-core --features typescript --bin export-types -- "$crate_dir/types"
cargo build -p yomilo-wasm --target wasm32-unknown-unknown --release
wasm-bindgen --target nodejs --out-dir "$crate_dir/pkg" --out-name yomilo_wasm \
  "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/yomilo_wasm.wasm"
printf '%s\n' '{"type":"commonjs"}' > "$crate_dir/pkg/package.json"

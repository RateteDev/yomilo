set shell := ["bash", "-euo", "pipefail", "-c"]

# Install the locked JS workspace and commit hooks
[group('Setup')]
setup:
    git config core.hooksPath .githooks
    just install

# Install JS dependencies without changing the lockfile
[group('Setup')]
install:
    bun install --frozen-lockfile

# Check first-party TS/JSON/CSS formatting and lint
[group('Repo')]
check:
    bunx biome check .

# Run Rust checks without requiring Bun or HTML assets
[group('Rust')]
core-ci:
    cargo fmt --all -- --check
    cargo test --workspace --locked

# Prepare WASM and exported types explicitly before JS consumers run
[group('WASM')]
wasm-build:
    bash crates/yomilo-wasm/build.sh

# Check the JS workspace independently of Rust tests
[group('HTML')]
html-ci: check
    bunx tsc --project packages/yomilo-html/tsconfig.json --noEmit
    bun test packages/yomilo-html/tests/unit/
    node crates/yomilo-wasm/smoke.mjs crates/yomilo-core/tests/fixtures/conformance.md > /dev/null

# Aggregate the independent checks; WASM preparation is not implicit
[group('CI')]
ci-local:
    #!/usr/bin/env bash
    set -euo pipefail
    just core-ci & rust_pid=$!
    just html-ci & html_pid=$!
    failed=0
    wait "$rust_pid" || failed=1
    wait "$html_pid" || failed=1
    exit "$failed"

# NAPP — Nederlandse autoriteit politieke partijen (demo)

# Path to the regelrecht checkout (engine, validate binary, schema)
regelrecht := env_var_or_default("REGELRECHT_DIR", "../regelrecht")

# List available commands
default:
    @just --list

# Validate all law YAML files against the regelrecht schema
law-validate:
    cargo run --manifest-path {{regelrecht}}/packages/engine/Cargo.toml \
        --features validate --bin validate --release -- \
        law/wet_op_de_politieke_partijen/2026-01-01.yaml \
        law/regeling_subsidiebedragen/2026-01-01.yaml \
        law/algemene_wet_bestuursrecht/1994-01-01.yaml \
        law/algemene_termijnenwet/1964-04-01.yaml

# Run the BDD scenario suite against the engine
bdd *ARGS:
    cargo test --test bdd -- {{ARGS}}

# Run Rust unit tests
test:
    cargo test --workspace --exclude-test bdd 2>/dev/null || cargo test --lib --bins

# Check formatting
format:
    cargo fmt --all -- --check

# Run clippy
lint:
    cargo clippy --workspace --all-targets

# Build the engine to WASM for the in-browser scenario runner
wasm:
    cargo build --manifest-path {{regelrecht}}/packages/engine/Cargo.toml \
        --features wasm --target wasm32-unknown-unknown --release
    wasm-bindgen --target web --out-dir frontend/public/wasm/pkg \
        {{regelrecht}}/packages/target/wasm32-unknown-unknown/release/regelrecht_engine.wasm

# Run the backend API server
backend:
    cargo run --bin napp-backend

# Run the frontend dev server
frontend:
    cd frontend && npm run dev

# Run backend + frontend together (dev)
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    just backend &
    BACKEND_PID=$!
    trap "kill $BACKEND_PID 2>/dev/null" EXIT
    just frontend

# All quality checks
check: format lint law-validate bdd

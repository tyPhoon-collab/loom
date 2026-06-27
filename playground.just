# Install Playground dependencies
install:
    npm --prefix apps/playground install

# Run Playground dev server
dev *args: wasm-build
    npm --prefix apps/playground run dev -- --host 127.0.0.1 --port 5174 {{ args }}

# Build Rust wasm package for Playground
wasm-build:
    wasm-pack build --target web --out-dir apps/playground/src/generated/loom_wasm --no-opt --features wasm --no-default-features

# Build Playground web app
app-build:
    npm --prefix apps/playground run build

# Check Rust wasm API
wasm-check:
    cargo check --lib --no-default-features --features wasm --target wasm32-unknown-unknown

# Smoke test Rust wasm API
wasm-smoke:
    cargo test --lib --no-default-features --features wasm wasm

# Preview built Playground
preview *args:
    npm --prefix apps/playground run preview -- {{ args }}

# Audit Playground dependencies
audit:
    npm --prefix apps/playground audit --audit-level=high

# Run Playground quality checks
check: wasm-check wasm-smoke build audit

# Build Playground
build: wasm-build app-build

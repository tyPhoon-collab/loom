# Install site dependencies. You NEED "cd site && pnpm approve-builds" before this.
install:
    pnpm -C site install

# Run docs dev server
dev:
    pnpm -C site docs:dev

# Generate site docs from source-of-truth
gen:
    cargo xtask gen-docs

# Verify generated site docs are up-to-date
check:
    cargo xtask check-docs

# Build docs site
build:
    pnpm -C site docs:build

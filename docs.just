# Install docs dependencies. You NEED "cd docs && pnpm approve-builds" before this.
install:
    pnpm -C docs install

# Run docs dev server
dev:
    pnpm -C docs docs:dev

# Generate docs from source-of-truth (currently diagnostic codes -> docs/reference/errors.md)
gen:
    cargo xtask gen-docs

# Verify generated docs are up-to-date (fails when docs/reference/errors.md needs regeneration)
check:
    cargo xtask check-docs

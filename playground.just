# Install Playground dependencies
install:
    npm --prefix apps/playground install

# Run Playground dev server
dev *args:
    npm --prefix apps/playground run dev -- {{ args }}

# Build Playground
build:
    npm --prefix apps/playground run build

# Preview built Playground
preview *args:
    npm --prefix apps/playground run preview -- {{ args }}

# Audit Playground dependencies
audit:
    npm --prefix apps/playground audit --audit-level=high

# Run Playground quality checks
check: build audit

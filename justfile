prefix := "/usr"
rootdir := ""

# Build the application in release mode (default)
default: build

# Alias for build-release
build: build-release

# Build the application in release mode
build-release:
    cargo build --release

# Build the application in debug mode
build-debug:
    cargo build

# Run the application
run:
    cargo run

# Install the application into the system (build first with `cargo build --release`)
install:
    install -Dm755 target/release/cosmic-watch {{rootdir}}{{prefix}}/bin/cosmic-watch
    install -Dm644 resources/icons/hicolor/scalable/apps/com.github.Moon-Mind.cosmic-watch.svg {{rootdir}}{{prefix}}/share/icons/hicolor/scalable/apps/com.github.Moon-Mind.cosmic-watch.svg
    install -Dm644 com.github.Moon-Mind.cosmic-watch.desktop {{rootdir}}{{prefix}}/share/applications/com.github.Moon-Mind.cosmic-watch.desktop

# Vendor dependencies locally
vendor:
    cargo vendor

# Build with vendored dependencies
build-vendored:
    cargo build --release --frozen --offline

# Run clippy on the project to check for linter warnings
check:
    cargo clippy --all-targets --all-features

# Run cargo check with JSON output for IDEs supporting LSP
check-json:
    cargo check --message-format=json

# Clean build artifacts
clean:
    cargo clean
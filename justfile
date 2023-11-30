# List all the things you can do with this justfile, with descriptions
help:
  @just --list

# Run the project for local development
dev:
    cargo tauri dev

alias setup := install
# Install all project dependencies
install:
    echo "cargo doesn't do installing! All done"

# Build the binary for production deployment
build:
    cargo tauri build

alias fmt := format
# Run the code formatter
format:
    cd src-tauri/ && cargo fmt

# Run static analysis on the code
lint:
    cd src-tauri/ && cargo clippy


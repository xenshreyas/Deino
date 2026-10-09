#!/bin/bash

# Exit on error
set -e

# Add log level selector with default to INFO
LOG_LEVEL=${1:-info}

case $LOG_LEVEL in
    info|debug|trace)
        export RUST_LOG=$LOG_LEVEL
        ;;
    *)
        echo "Invalid log level: $LOG_LEVEL. Valid options: info, debug, trace"
        exit 1
        ;;
esac

# Check and install CMake if needed
echo "Checking CMake version..."
if ! command -v cmake &> /dev/null; then
    echo "CMake not found. Installing via Homebrew..."
    brew install cmake
else
    CMAKE_VERSION=$(cmake --version | head -n1 | cut -d" " -f3)
    if [[ "$CMAKE_VERSION" < "3.5" ]]; then
        echo "CMake version $CMAKE_VERSION is too old. Updating via Homebrew..."
        brew upgrade cmake
    fi
fi

# Clean up previous builds
echo "Cleaning up previous builds..."
rm -rf target/
rm -rf src-tauri/target
rm -rf src-tauri/gen

# Clean up dependencies and the UI build
echo "Cleaning up dependencies and the UI build..."
rm -rf node_modules
rm -rf .pnp.cjs
rm -rf dist

echo "Installing dependencies..."
pnpm install

# Build the UI (Vite) first
echo "Building the UI..."
pnpm run build

# Set environment variables for the build

echo "Building Tauri app..."
pnpm run tauri build
sleep


#!/usr/bin/env bash

set -e

# Resolve the root of the project relative to this script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR/.."

# Paths
BIN_DIR="/c/Users/charl/bin"
BUILD_PATH="$PROJECT_ROOT/target/release/mis.exe"
DEST_PATH="$BIN_DIR/mis-latest.exe"

# Build
echo "🔧 Building mis CLI..."
cd "$PROJECT_ROOT"
cargo build --release

# Move and rename
echo "📦 Moving to $DEST_PATH..."
cp -f "$BUILD_PATH" "$DEST_PATH"

echo "✅ mis-latest.exe is ready!"

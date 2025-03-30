#!/bin/bash

set -e

echo "Building Make It So..."
cargo build --release

BIN_PATH="./target/release/mis"
INSTALL_PATH="$HOME/.cargo/bin/mis"

if [ ! -f "$BIN_PATH" ]; then
  echo "❌ Build failed: binary not found at $BIN_PATH"
  exit 1
fi

echo "📦 Installing to $INSTALL_PATH"
cp "$BIN_PATH" "$INSTALL_PATH"

echo "✅ Make It So installed successfully!"
which mis || echo "⚠️ Make It So not found in PATH"
mis --version || echo "⚠️ Make It So installed but unable to run --version"

#!/bin/bash

set -e

echo "🚢 Building Shipwreck..."
cargo build --release

BIN_PATH="./target/release/shipwreck"
INSTALL_PATH="$HOME/.cargo/bin/shipwreck"

if [ ! -f "$BIN_PATH" ]; then
  echo "❌ Build failed: binary not found at $BIN_PATH"
  exit 1
fi

echo "📦 Installing to $INSTALL_PATH"
cp "$BIN_PATH" "$INSTALL_PATH"

echo "✅ Shipwreck installed successfully!"
which shipwreck
shipwreck --version || echo "⚠️ Shipwreck installed but unable to run --version"

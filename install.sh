#!/bin/bash

set -e

echo "Building PLIC in release mode..."
cargo build --release

BIN_SRC="target/release/plic"
BIN_DEST="/bin/plic"

if [ ! -f "$BIN_SRC" ]; then
    echo "Error: build failed, binary not found at $BIN_SRC"
    exit 1
fi

echo "Installing binary to $BIN_DEST (requires sudo)..."
sudo cp "$BIN_SRC" "$BIN_DEST"
sudo chmod 755 "$BIN_DEST"

echo "Installation complete! You can now run 'plic'."

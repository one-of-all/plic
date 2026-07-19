#!/bin/bash

set -e

echo "Building PLIC in debug mode..."
cargo build

BIN_SRC="target/debug/plic"
BIN_DEST="/bin/plic"

if [ ! -f "$BIN_SRC" ]; then
    echo "Error: build failed, binary not found at $BIN_SRC"
    exit 1
fi

echo "Installing binary to $BIN_DEST (requires sudo)..."
sudo cp "$BIN_SRC" "$BIN_DEST"
sudo chmod 755 "$BIN_DEST"

echo "Installation complete! You can now run 'plic'."

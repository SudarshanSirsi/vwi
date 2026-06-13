#!/usr/bin/env bash
# VWi Release Build Script (Unix)
# Builds a single optimized binary and copies it to the project root.

set -e

OUTPUT_PATH="${1:-./vwi}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building VWi release binary..."

# Build optimized release
cargo build --release

# Copy binary to requested output path
SRC="./target/release/vwi"
cp "$SRC" "$OUTPUT_PATH"

FILE_SIZE=$(du -sh "$OUTPUT_PATH" | cut -f1)
echo ""
echo "Success! Binary saved to: $OUTPUT_PATH"
echo "File size: $FILE_SIZE"

# Print instructions
echo ""
echo "To run VWi:"
echo "  1. Create config dir:  mkdir -p ~/.config/vwi"
echo "  2. Copy config:        cp config.example.toml ~/.config/vwi/config.toml"
echo "  3. Edit config:        nano ~/.config/vwi/config.toml"
echo "  4. Run:                ./vwi"
echo ""
echo "To add to startup (Linux desktop):"
echo "  cp build/vwi.desktop ~/.config/autostart/"

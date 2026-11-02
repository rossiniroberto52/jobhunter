#!/usr/bin/env bash
# Build do JobHunter para Windows — ZIP portable (sem Inno Setup)
# Pré-requisitos:
#   rustup target add x86_64-pc-windows-gnu
#   sudo apt install mingw-w64 zip
#
# Uso: ./installer/windows/build-zip.sh
# Saída: dist/jobhunter-windows.zip

set -euo pipefail

cd "$(dirname "$0")/../.."

echo "==> Garantindo target Windows..."
rustup target add x86_64-pc-windows-gnu 2>/dev/null || true

echo "==> Cross-compile para Windows..."
cargo build --release --target x86_64-pc-windows-gnu

echo "==> Empacotando ZIP..."
mkdir -p dist
EXE="target/x86_64-pc-windows-gnu/release/jobhunter.exe"
ZIP="dist/jobhunter-windows.zip"

rm -f "$ZIP"
zip -j "$ZIP" "$EXE" config.toml

echo "==> Pronto!"
echo "   ZIP: $ZIP"
echo "   Coloque esse arquivo na pasta compartilhada do FileZilla."

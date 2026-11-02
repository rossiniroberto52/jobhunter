#!/usr/bin/env bash
# Build do instalador Windows para JobHunter
# Pré-requisitos no Linux:
#   rustup target add x86_64-pc-windows-gnu
#   sudo apt install mingw-w64 inno-setup
#
# Uso: ./installer/windows/build-windows.sh

set -euo pipefail

cd "$(dirname "$0")/../.."  # raiz do repositório jobhunter/

echo "==> Adicionando target Windows (se necessário)..."
rustup target add x86_64-pc-windows-gnu 2>/dev/null || true

echo "==> Fazendo cross-compile para Windows..."
cargo build --release --target x86_64-pc-windows-gnu

echo "==> Copiando assets..."
mkdir -p installer/windows/output
cp target/x86_64-pc-windows-gnu/release/jobhunter.exe installer/windows/output/
cp config.toml installer/windows/output/

echo "==> Gerando instalador com Inno Setup..."
iscc installer/windows/install.iss

echo "==> Pronto! Instalador em: dist/jobhunter-setup-0.1.0.exe"
echo "    Coloque esse .exe na pasta compartilhada do FileZilla."

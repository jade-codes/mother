#!/bin/bash
# Devcontainer setup script - installs language servers and tools

set -e

echo "=== Mother Devcontainer Setup ==="

# Show versions
echo "Rust: $(rustc --version)"
echo "Cargo: $(cargo --version)"
echo "Node: $(node --version)"
echo "Go: $(go version)"
echo "Python: $(python3 --version)"

# Install TypeScript/JavaScript LSP
echo ""
echo "Installing TypeScript language server..."
npm install -g typescript typescript-language-server

# Install Python LSP
echo ""
echo "Installing Python language server (pyright)..."
npm install -g pyright

# Install Go LSP
echo ""
echo "Installing Go language server (gopls)..."
go install golang.org/x/tools/gopls@latest

# Install SysML LSP
echo ""
echo "Installing SysML language server (syster-lsp)..."
cargo install --git https://github.com/jade-codes/syster syster-lsp

echo ""
echo "=== Setup Complete ==="
echo "Installed language servers:"
echo "  - rust-analyzer (bundled with Rust)"
echo "  - typescript-language-server"
echo "  - pyright"
echo "  - gopls"
echo "  - syster-lsp"

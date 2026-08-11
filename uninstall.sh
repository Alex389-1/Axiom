#!/bin/bash
set -e

echo "🗑️ Starting Axiom Uninstallation..."

# 1. Ask for confirmation
read -p "Are you sure you want to completely remove Axiom and its dependencies? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]
then
    echo "❌ Uninstallation aborted."
    exit 1
fi

# 2. Remove Node modules
echo "🧹 Removing Node.js dependencies..."
rm -rf node_modules package-lock.json

# 3. Remove Rust build artifacts
echo "🧹 Removing Rust build artifacts..."
rm -rf src-tauri/target src-tauri/Cargo.lock

# 4. Remove daemon data if any (optional, let's just delete the socket/logs if they exist)
echo "🧹 Removing Axiom daemon artifacts..."
rm -f /run/user/$(id -u)/axiom/axiom-daemon.sock 2>/dev/null || true
rm -rf ~/.gemini/antigravity-ide/brain/ 2>/dev/null || true

echo "✅ Uninstallation Complete!"
echo "If you want to remove the source code entirely, simply delete this directory:"
echo "rm -rf $(pwd)"

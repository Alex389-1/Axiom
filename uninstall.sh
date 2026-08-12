#!/bin/bash
set -e

echo "🗑️ Starting Axiom Complete Uninstallation..."

# 1. Stop all Axiom background processes
echo "🛑 Stopping running Axiom processes..."
pkill -9 -f axiom-daemon 2>/dev/null || true
pkill -9 -f axiom-tauri 2>/dev/null || true

# 2. Remove installed system binaries & directories
echo "🧹 Removing installed binaries..."
if [ -d "/opt/Axiom" ]; then
    sudo rm -rf /opt/Axiom
fi
sudo rm -f /usr/local/bin/axiom 2>/dev/null || true
sudo rm -f /usr/bin/axiom-tauri 2>/dev/null || true
sudo rm -f /usr/bin/axiom-daemon 2>/dev/null || true

# 3. Remove Desktop Launcher entries
echo "🧹 Removing Desktop menu entries..."
rm -f ~/.local/share/applications/Axiom.desktop 2>/dev/null || true
if [ -f "/usr/share/applications/Axiom.desktop" ]; then
    sudo rm -f /usr/share/applications/Axiom.desktop 2>/dev/null || true
fi
update-desktop-database ~/.local/share/applications 2>/dev/null || true

# 4. Remove runtime sockets and configs
echo "🧹 Cleaning runtime sockets & data..."
rm -rf /run/user/$(id -u)/axiom/ 2>/dev/null || true
rm -rf ~/.config/axiom/ 2>/dev/null || true
rm -rf ~/.cache/axiom/ 2>/dev/null || true

echo "✅ Axiom has been completely uninstalled from your system!"

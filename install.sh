#!/bin/bash
set -e

echo "🚀 Starting Axiom Setup..."

# 1. Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed. Please install Node.js (v18+) and try again."
    exit 1
fi

# 2. Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust and Cargo are not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# 3. Install Linux Dependencies (Ubuntu/Debian)
if [ -f /etc/debian_version ]; then
    echo "📦 Installing Linux system dependencies for Tauri (Debian/Ubuntu)..."
    sudo apt-get update
    sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev speech-dispatcher espeak-ng
fi

# 3b. Install Linux Dependencies (Arch/Manjaro)
if [ -f /etc/arch-release ]; then
    echo "📦 Installing Linux system dependencies for Tauri (Arch/Manjaro)..."
    sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file xdotool openssl libappindicator-gtk3 librsvg speech-dispatcher espeak-ng squashfs-tools
    # Create Debian-compatible WebKit symlinks needed by the Tauri AppImage bundler
    sudo mkdir -p /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1/injected-bundle
    sudo ln -sf /usr/lib/webkit2gtk-4.1/WebKitNetworkProcess \
        /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1/WebKitNetworkProcess 2>/dev/null || true
    sudo ln -sf /usr/lib/webkit2gtk-4.1/injected-bundle/libwebkit2gtkinjectedbundle.so \
        /usr/lib/x86_64-linux-gnu/webkit2gtk-4.1/injected-bundle/libwebkit2gtkinjectedbundle.so 2>/dev/null || true
fi

# 4. Install Node dependencies
echo "📦 Installing Node.js dependencies..."
npm install
node node_modules/esbuild/install.js 2>/dev/null || true

echo "✅ Setup Complete!"
echo "👉 To start the development server, run: npm run tauri dev"
echo "👉 To build for production (Linux), run: npm run tauri build"

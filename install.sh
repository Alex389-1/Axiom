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
    echo "📦 Installing Linux system dependencies for Tauri..."
    sudo apt-get update
    sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
fi

# 4. Install Node dependencies
echo "📦 Installing Node.js dependencies..."
npm install

echo "✅ Setup Complete!"
echo "👉 To start the development server, run: npm run tauri dev"
echo "👉 To build for production, run: npm run tauri build"

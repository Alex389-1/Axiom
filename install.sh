#!/bin/bash
set -e

echo "🚀 Installing Axiom Cleanly..."

# 1. Kill running instances
pkill -9 -f axiom-daemon 2>/dev/null || true
pkill -9 -f axiom-tauri 2>/dev/null || true
rm -rf /run/user/$(id -u)/axiom/ 2>/dev/null || true

# 2. Build release AppImage package if needed
if [ ! -f "target/release/bundle/appimage/Axiom_0.1.0_amd64.AppImage" ]; then
    echo "📦 Building release AppImage..."
    cargo build --bin axiom-daemon --release
    npm run tauri build -- --bundles appimage
fi

# 3. Clean target install location
echo "🧹 Preparing /opt/Axiom..."
if [ -d "/opt/Axiom" ]; then
    sudo rm -rf /opt/Axiom
fi
sudo mkdir -p /opt/Axiom

# 4. Extract AppImage to /opt/Axiom
echo "📦 Extracting package to /opt/Axiom..."
TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"
/home/alex/Desktop/Axiom/target/release/bundle/appimage/Axiom_0.1.0_amd64.AppImage --appimage-extract >/dev/null 2>&1
sudo cp -r squashfs-root/* /opt/Axiom/
rm -rf "$TMP_DIR"
cd /home/alex/Desktop/Axiom

# Fix APPDIR in /opt/Axiom/AppRun so GTK plugin hooks find /opt/Axiom/usr
sudo sed -i '7a export APPDIR="$this_dir"' /opt/Axiom/AppRun 2>/dev/null || true

# 5. Set up binaries and symlinks
echo "🔗 Creating system symlinks..."
sudo ln -sf /opt/Axiom/AppRun /usr/local/bin/axiom
sudo mkdir -p /opt/Axiom/usr/bin
if [ -f "/opt/Axiom/usr/lib/Axiom/_up_/target/release/axiom-daemon" ]; then
    sudo ln -sf /opt/Axiom/usr/lib/Axiom/_up_/target/release/axiom-daemon /opt/Axiom/usr/bin/axiom-daemon
fi

# Clean old /usr/bin files if present
if [ -f "/usr/bin/axiom-tauri" ]; then
    sudo rm -f /usr/bin/axiom-tauri 2>/dev/null || true
fi
if [ -f "/usr/bin/axiom-daemon" ]; then
    sudo rm -f /usr/bin/axiom-daemon 2>/dev/null || true
fi

# 6. Install desktop launcher
echo "🖥️ Installing Desktop entry..."
mkdir -p ~/.local/share/applications
cat << 'EOF' > ~/.local/share/applications/Axiom.desktop
[Desktop Entry]
Categories=Utility;Development;
Comment=Axiom — GUI for local LLMs
Exec=env APPIMAGE_EXTRACT_AND_RUN=1 /home/alex/Desktop/Axiom/target/release/bundle/appimage/Axiom_0.1.0_amd64.AppImage
StartupWMClass=axiom-tauri
Icon=axiom-tauri
Name=Axiom
Terminal=false
Type=Application
EOF
chmod +x ~/.local/share/applications/Axiom.desktop
update-desktop-database ~/.local/share/applications 2>/dev/null || true

echo "✅ Axiom installation complete!"
echo "Run 'axiom' in your terminal or click Axiom in your App Launcher."

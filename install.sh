#!/usr/bin/env bash
set -e

# Get the directory where the script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd 2>/dev/null || pwd)"

echo "[uid-agent] Starting installation on Linux..."

# Create target directories
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

ICON_DIR="$HOME/.local/share/icons"
mkdir -p "$ICON_DIR"

DESKTOP_DIR="$HOME/.local/share/applications"
mkdir -p "$DESKTOP_DIR"

# Stop active service if running to avoid text file busy errors
if systemctl --user is-active --quiet uid-agent.service; then
    echo "[uid-agent] Stopping active background service..."
    systemctl --user stop uid-agent.service
fi

# Stop desktop app if running
pkill -9 -f uid-agent-desktop || true

# Check if rust/cargo is installed to build from source (only if Cargo.toml exists)
if [ -f "$SCRIPT_DIR/Cargo.toml" ] && command -v cargo &> /dev/null; then
    echo "[uid-agent] Building latest binaries from source..."
    
    # 1. Build CLI/daemon binary
    echo "[uid-agent] Compiling CLI daemon..."
    cargo build --release
    
    # 2. Build UI assets
    if [ -d "$SCRIPT_DIR/ui" ]; then
        echo "[uid-agent] Compiling UI assets..."
        (cd "$SCRIPT_DIR/ui" && pnpm install && pnpm build)
    fi
    
    # 3. Build Tauri desktop application
    if [ -d "$SCRIPT_DIR/src-tauri" ]; then
        echo "[uid-agent] Compiling Tauri desktop app..."
        (cd "$SCRIPT_DIR/src-tauri" && cargo build --release)
    fi
fi

# Copy Daemon Binary
TARGET_BIN="$SCRIPT_DIR/target/release/uid-agent"
if [ ! -f "$TARGET_BIN" ] && [ -f "$SCRIPT_DIR/uid-agent" ]; then
    TARGET_BIN="$SCRIPT_DIR/uid-agent"
fi

if [ ! -f "$TARGET_BIN" ]; then
    echo "[uid-agent] Local executable not found, downloading precompiled binary..."
    TEMP_DIR=$(mktemp -d)
    TARGET_BIN="$TEMP_DIR/uid-agent"
    if command -v curl &> /dev/null; then
        curl -sSL -o "$TARGET_BIN" https://raw.githubusercontent.com/oneuid/uid-agent/main/uid-agent
    elif command -v wget &> /dev/null; then
        wget -q -O "$TARGET_BIN" https://raw.githubusercontent.com/oneuid/uid-agent/main/uid-agent
    fi
fi

if [ -f "$TARGET_BIN" ]; then
    cp "$TARGET_BIN" "$BIN_DIR/uid-agent"
    chmod +x "$BIN_DIR/uid-agent"
    echo "[uid-agent] Installed daemon binary to $BIN_DIR/uid-agent"
    if [ -n "$TEMP_DIR" ] && [ -d "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi
else
    echo "[WARNING] Daemon binary not found. Background service might not start."
fi

# Copy GUI Desktop Binary
TARGET_GUI="$SCRIPT_DIR/src-tauri/target/release/uid-agent-desktop"
if [ ! -f "$TARGET_GUI" ] && [ -f "$SCRIPT_DIR/uid-agent-desktop" ]; then
    TARGET_GUI="$SCRIPT_DIR/uid-agent-desktop"
fi

if [ -f "$TARGET_GUI" ]; then
    cp "$TARGET_GUI" "$BIN_DIR/uid-agent-desktop"
    chmod +x "$BIN_DIR/uid-agent-desktop"
    echo "[uid-agent] Installed GUI desktop binary to $BIN_DIR/uid-agent-desktop"
    
    # Copy App Icon
    if [ -f "$SCRIPT_DIR/src-tauri/icons/128x128.png" ]; then
        cp "$SCRIPT_DIR/src-tauri/icons/128x128.png" "$ICON_DIR/uid-agent-desktop.png"
    fi
    
    # Create Desktop Launcher Entry
    cat << DESKTOP_EOF > "$DESKTOP_DIR/uid-agent-desktop.desktop"
[Desktop Entry]
Name=UID Agent
Comment=Endpoint Security Attestation Agent
Exec=$BIN_DIR/uid-agent-desktop
Icon=uid-agent-desktop
Terminal=false
Type=Application
Categories=Security;System;
DESKTOP_EOF
    chmod +x "$DESKTOP_DIR/uid-agent-desktop.desktop"
    echo "[uid-agent] Created desktop applications launcher at $DESKTOP_DIR/uid-agent-desktop.desktop"
else
    echo "[WARNING] GUI desktop binary not found. Desktop application will not be registered."
fi

# Set up systemd user service for background daemon
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER_DIR"

SERVICE_FILE="$SYSTEMD_USER_DIR/uid-agent.service"

cat << SYSTEMD_EOF > "$SERVICE_FILE"
[Unit]
Description=UID Agent — Local Endpoint Signing and Security Daemon
After=network.target

[Service]
ExecStart=%h/.local/bin/uid-agent daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
SYSTEMD_EOF

echo "[uid-agent] Created systemd service at $SERVICE_FILE"

# Reload systemd user configuration
systemctl --user daemon-reload

# Enable and start the background daemon service
if [ -f "$BIN_DIR/uid-agent" ]; then
    systemctl --user enable uid-agent.service
    systemctl --user restart uid-agent.service
    echo "[uid-agent] Service successfully registered and started."
    echo "  Check daemon status:  systemctl --user status uid-agent.service"
fi

echo "[uid-agent] Installation completed successfully!"
echo "  You can open UID Agent from your desktop Applications Menu."

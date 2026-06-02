#!/usr/bin/env bash
set -e

# Get the directory where the script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd 2>/dev/null || pwd)"

echo "[uid-agent] Starting installation on Linux..."

# Create target directory for user binaries
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

# Stop service if running to avoid "text file busy" error
if systemctl --user is-active --quiet uid-agent.service; then
    echo "[uid-agent] Stopping active background service..."
    systemctl --user stop uid-agent.service
fi

# Copy binary to local bin directory
TARGET_BIN="$SCRIPT_DIR/uid-agent"
if [ ! -f "$TARGET_BIN" ]; then
    TARGET_BIN="$SCRIPT_DIR/target/release/uid-agent"
fi

if [ ! -f "$TARGET_BIN" ]; then
    echo "[uid-agent] Local binary not found, downloading precompiled binary..."
    TARGET_BIN="/tmp/uid-agent"
    if curl -sSL --fail "https://raw.githubusercontent.com/oneuid/uid-agent/main/uid-agent" -o "$TARGET_BIN"; then
        chmod +x "$TARGET_BIN"
    elif wget -q "https://raw.githubusercontent.com/oneuid/uid-agent/main/uid-agent" -O "$TARGET_BIN"; then
        chmod +x "$TARGET_BIN"
    else
        echo "[ERROR] Precompiled binary not found, and failed to download from GitHub."
        echo "Please build the project first using 'cargo build --release'."
        exit 1
    fi
fi

cp "$TARGET_BIN" "$BIN_DIR/uid-agent"
chmod +x "$BIN_DIR/uid-agent"
echo "[uid-agent] Copied binary to $BIN_DIR/uid-agent"

# Set up systemd user service
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

# Enable and start the service
systemctl --user enable uid-agent.service
systemctl --user restart uid-agent.service

echo "[uid-agent] Service successfully registered and started."
echo "[uid-agent] You can check its status using:"
echo "  systemctl --user status uid-agent.service"
echo "[uid-agent] Or read logs using:"
echo "  journalctl --user -u uid-agent.service -f"

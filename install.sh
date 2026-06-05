#!/usr/bin/env bash
set -e

# Get the directory where the script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd 2>/dev/null || pwd)"

echo "[uid-agent] Starting installation on Linux..."

# Install runtime and build dependencies if missing on Debian/Ubuntu
if [ -f /etc/debian_version ]; then
    MISSING_DEPS=()
    
    # 1. Runtime clipboard and notification dependencies
    if ! command -v wl-paste &> /dev/null; then
        MISSING_DEPS+=("wl-clipboard")
    fi
    if ! command -v xclip &> /dev/null; then
        MISSING_DEPS+=("xclip")
    fi
    if ! command -v notify-send &> /dev/null; then
        MISSING_DEPS+=("libnotify-bin")
    fi
    
    # 2. Build dependencies (only if building from source is possible)
    if [ -f "$SCRIPT_DIR/Cargo.toml" ]; then
        BUILD_DEPS=("libdbus-1-dev" "pkg-config" "libwebkit2gtk-4.1-dev" "libssl-dev" "libgtk-3-dev" "libayatana-appindicator3-dev" "build-essential")
        for dep in "${BUILD_DEPS[@]}"; do
            if ! dpkg -s "$dep" &> /dev/null; then
                MISSING_DEPS+=("$dep")
            fi
        done
        
        # Check node/npm for UI assets compilation
        if [ -d "$SCRIPT_DIR/ui" ]; then
            if ! command -v node &> /dev/null; then
                MISSING_DEPS+=("nodejs")
            fi
            if ! command -v npm &> /dev/null; then
                MISSING_DEPS+=("npm")
            fi
        fi
    fi
    
    if [ ${#MISSING_DEPS[@]} -ne 0 ]; then
        echo "[uid-agent] Installing missing dependencies: ${MISSING_DEPS[*]}..."
        if command -v sudo &> /dev/null; then
            sudo apt-get update && sudo apt-get install -y "${MISSING_DEPS[@]}"
        else
            echo "[WARNING] sudo is not available. Please install manually: apt-get install -y ${MISSING_DEPS[*]}"
        fi
    fi
fi

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
        if ! command -v pnpm &> /dev/null; then
            echo "[uid-agent] pnpm not found. Installing pnpm..."
            if command -v npm &> /dev/null; then
                if command -v sudo &> /dev/null; then
                    sudo npm install -g pnpm
                else
                    npm install -g pnpm || true
                fi
            else
                echo "[WARNING] npm not found. Please install nodejs and pnpm manually."
            fi
        fi
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

# -------------------------------------------------------------
# OS Context Menu Setup (Sign with UID)
# -------------------------------------------------------------
echo "[uid-agent] Installing 'Sign with UID' OS context menu integration..."

# 1. Install wrapper launcher script
cat << 'LAUNCHER_EOF' > "$BIN_DIR/uid-sign-file"
#!/usr/bin/env bash
# UID Digital Signature file launcher wrapper
set -e

FILE_PATH="$1"
if [ -z "$FILE_PATH" ]; then
    echo "Usage: uid-sign-file <pdf_file_path>"
    exit 1
fi

ABS_PATH=$(realpath "$FILE_PATH")

# Find a supported Chromium-based browser
BROWSER_CMD=""
for b in google-chrome chromium brave-browser microsoft-edge; do
    if command -v "$b" >/dev/null 2>&1; then
        BROWSER_CMD="$b"
        break
    fi
done

if [ -z "$BROWSER_CMD" ]; then
    if command -v zenity >/dev/null 2>&1; then
        zenity --error --text="No supported Chromium-based browser (Chrome, Chromium, Brave, Edge) was found. Please install one to use Sign with UID."
    elif command -v notify-send >/dev/null 2>&1; then
        notify-send "Sign with UID Error" "No supported browser found."
    else
        echo "Error: No supported browser found."
    fi
    exit 1
fi

# Dynamically resolve Extension ID based on what the user has loaded in Chrome
EXT_ID=$(python3 -c '
import json, os, glob
chrome_dir = os.path.expanduser("~/.config/google-chrome")
paths = glob.glob(os.path.join(chrome_dir, "*", "Preferences"))
target_keywords = ["uid-link/dist/chrome", "extensions/coobgfinhhjocjlhjiaegcfolhdgiinb", "uid-extension/dist"]
resolved_id = None
for kw in target_keywords:
    for pref_path in paths:
        try:
            with open(pref_path, "r", encoding="utf-8") as f:
                d = json.load(f)
            extensions = d.get("extensions", {}).get("settings", {})
            for ext_id, info in extensions.items():
                path = info.get("path", "")
                if kw in path:
                    resolved_id = ext_id
                    break
        except Exception:
            continue
        if resolved_id:
            break
    if resolved_id:
        break
print(resolved_id or "coobgfinhhjocjlhjiaegcfolhdgiinb")
' 2>/dev/null || echo "coobgfinhhjocjlhjiaegcfolhdgiinb")

# Open extension PDF signer page
exec "$BROWSER_CMD" "chrome-extension://$EXT_ID/pdf-signer.html?url=file://$ABS_PATH"
LAUNCHER_EOF
chmod +x "$BIN_DIR/uid-sign-file"

# 2. Install desktop application association
cat << DESKTOP_EOF > "$DESKTOP_DIR/uid-signer.desktop"
[Desktop Entry]
Type=Application
Name=Sign with UID
Comment=Digitally sign PDF documents using UID.one
MimeType=application/pdf;
Exec=$BIN_DIR/uid-sign-file %f
Icon=uid-agent-desktop
Terminal=false
NoDisplay=false
Categories=Utility;Security;
DESKTOP_EOF
chmod +x "$DESKTOP_DIR/uid-signer.desktop"

# Register PDF mimetype association
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$DESKTOP_DIR" || true
fi

# 3. Install GNOME Nautilus Script
NAUTILUS_SCRIPTS_DIR="$HOME/.local/share/nautilus/scripts"
mkdir -p "$NAUTILUS_SCRIPTS_DIR"
cat << 'SCRIPT_EOF' > "$NAUTILUS_SCRIPTS_DIR/Sign with UID"
#!/usr/bin/env bash
# Nautilus Script to sign PDF files with UID
for file in "$@"; do
    uid-sign-file "$file" &
done
SCRIPT_EOF
chmod +x "$NAUTILUS_SCRIPTS_DIR/Sign with UID"

echo "[uid-agent] Installation completed successfully!"
echo "  You can open UID Agent from your desktop Applications Menu."

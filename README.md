# UID Agent

A cross-platform endpoint security agent written in Rust, providing continuous session integrity monitoring, hardware-bound identity attestation, and containerized secure workspaces.

**Supported platforms:** Linux (Ubuntu/Debian) · macOS · Windows

For detailed architectural specifications, see the [UID Platform Specifications](../uid-web/docs/uid-platform-specs.md).

---

## UID Desktop App (GUI & Linux App Sandbox)

UID Agent now features a beautiful **Tauri-powered Desktop GUI Dashboard** and a **Secure App Sandbox** that lets you run Windows enterprise applications (like Zalo Messenger) safely inside containerized Wine Docker sandboxes on Linux.

### Key Features:
- **SOC 2 Posture Dashboard**: View device compliance controls (disk encryption, firewall status, secure boot, OS updates).
- **USB Security Tokens**: Scan and view plugged-in PKCS#11 hardware keys and certificates.
- **Enterprise App Sandbox (Docker + Wine)**: Run Windows applications locally in a secure, containerized sandbox. 
- **System Tray Integration**: Minimizes cleanly to the Linux system tray, protecting the background daemon from accidental exits.
- **Persistent Data Volume**: All sandbox app data, chat logs, and logins are persisted locally on the host at `~/.local/share/uid/apps/` - ensuring updates to the agent never lose your work.

---

## Desktop GUI Build & Installation (Linux)

### 1. Install Build Dependencies
To compile Tauri and its windowing controls on Debian/Ubuntu, run:
```bash
sudo apt update && sudo apt install -y libdbus-1-dev pkg-config libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev libayatana-appindicator3-dev
```

### 2. Build and Package (.deb)
To compile and package the desktop app as a native Debian installer:
```bash
# Navigate to the workspace and build
npx -y @tauri-apps/cli build
```
This will compile the Rust core and React interface in release mode and generate:
- **Debian Package**: `src-tauri/target/release/bundle/deb/uid-agent-desktop_3.0.0_amd64.deb`
- **Portable AppImage**: `src-tauri/target/release/bundle/appimage/uid-agent-desktop_3.0.0_amd64.AppImage`

### 3. Install the App
Install the compiled Debian package on Ubuntu:
```bash
sudo dpkg -i src-tauri/target/release/bundle/deb/uid-agent-desktop_3.0.0_amd64.deb
```

### 4. Pin to Ubuntu Dock/Toolbar
Once installed via `.deb`:
1. Press the **Super (Windows) key** to open the GNOME Applications menu.
2. Search for **"UID Agent"**.
3. Right-click the UID Agent icon and select **"Add to Favorites"** (Thêm vào danh sách ưa thích).
4. The app is now pinned to your toolbar/Dock for quick launch.

---

## Desktop GUI Build & Installation (Windows)

### 1. Install Build Dependencies
To compile the Tauri desktop app on Windows, make sure you have installed:
1. **Microsoft C++ Build Tools**: Download and install the [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and select the **Desktop development with C++** workload.
2. **Rustup (Rust compiler)**: Install Rust via [rustup.rs](https://rustup.rs/), selecting the `stable-x86_64-pc-windows-msvc` toolchain.
3. **Node.js**: Install Node.js (v18 or newer).

### 2. Build and Package (.msi / .exe)
Open PowerShell inside the cloned repository directory and run:
```powershell
# Compile and bundle the desktop application
npx -y @tauri-apps/cli build
```
This will compile the Rust core and React interface, packaging them into native Windows installers:
* **MSI Installer**: `src-tauri/target/release/bundle/msi/uid-agent-desktop_3.0.0_x64_en-US.msi`
* **NSIS Setup Exe**: `src-tauri/target/release/bundle/nsis/uid-agent-desktop_3.0.0_x64-setup.exe`

### 3. Install and Pin
1. Double-click the generated `.msi` or `-setup.exe` installer inside the release bundle directory and follow the prompt.
2. Once installed, search for **"UID Agent"** in the Windows Start Menu.
3. Right-click the app icon and select **"Pin to Taskbar"** or **"Pin to Start"** for quick access.

### 4. Uninstalling Old CLI Version (To Prevent Port Conflicts)
If you previously installed the minimal CLI background daemon, run this single command in PowerShell to stop, unregister, and clean it up completely:
```powershell
Stop-Process -Name "uid-agent" -Force; Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "UIDAgent" -ErrorAction SilentlyContinue; Remove-Item -Path "$env:LOCALAPPDATA\uid-agent" -Recurse -Force -ErrorAction SilentlyContinue
```

---

## CLI Installation & Usage

For headless servers or environments where only the CLI daemon is required:

### Linux CLI installation:
```bash
curl -sSL https://raw.githubusercontent.com/oneuid/uid-agent/main/install.sh | bash
```

### Windows CLI installation:
```bash
Invoke-Expression (Invoke-WebRequest -Headers @{"Cache-Control"="no-cache"} -Uri "https://raw.githubusercontent.com/oneuid/uid-agent/main/install.ps1?t=$(Get-Date -UFormat %s)" -UseBasicParsing).Content
```

### CLI Command Reference:
```
uid-agent register          — Generate hardware-bound Ed25519 keypair
uid-agent posture           — Collect device compliance posture (SOC 2 evidence)
uid-agent sign <data>       — Cryptographically sign a payload
uid-agent daemon            — Run background SSH agent socket and HTTP server
uid-agent approve <token>   — Connect, listen, and approve authentication challenge
```

## License

[Apache License 2.0](LICENSE) — Open source for complete transparency in how endpoint attestation and secure sandboxes are handled.

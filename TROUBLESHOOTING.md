# UID Agent Troubleshooting Guide: "Connection Refused"

This guide documents the causes and resolution steps for the `Could not connect to 127.0.0.1: Connection refused` (or similar connection refused) errors encountered when interacting with the UID Agent background service or desktop application.

---

## Table of Contents
1. [Cause 1: Background Daemon is Stopped or Rebuilding](#cause-1-background-daemon-is-stopped-or-rebuilding)
2. [Cause 2: Desktop GUI App in Development Mode (Missing Vite Server)](#cause-2-desktop-gui-app-in-development-mode-missing-vite-server)
3. [Cause 3: Browser Extension / Client Page Cache](#cause-3-browser-extension--client-page-cache)

---

## Cause 1: Background Daemon is Stopped or Rebuilding

### Explanation
The background agent daemon (`uid-agent daemon`) runs as a systemd user service (`uid-agent.service`) and listens on local port `13013`.
When you run `./install.sh`, the script:
1. Stops the service (`systemctl --user stop uid-agent.service`).
2. Compiles the Rust binaries (which takes 30–45 seconds).
3. Restarts the service at the end of the installation.

During this compilation window, the port `13013` is not listening, causing any active client requests (from Chrome, Firefox, or local web apps) to fail with `Connection refused`.

### Verification
Check if the daemon is currently running and listening:
```bash
# Check systemd user service status
systemctl --user status uid-agent.service

# Verify if port 13013 is listening
ss -tulpn | grep 13013
```

### Resolution
* **Wait for Installation to Finish**: If you just ran `./install.sh`, wait for the script to print `Installation completed successfully!`. The service restarts automatically.
* **Manually Restart Daemon**: If the service did not start or is inactive:
  ```bash
  systemctl --user daemon-reload
  systemctl --user restart uid-agent.service
  ```

---

## Cause 2: Desktop GUI App in Development Mode (Missing Vite Server)

### Explanation
The desktop GUI app (`uid-agent-desktop`) is built using Tauri.
* **Development/Debug Profile**: If run or compiled without the `--release` flag (e.g., `cargo build` or running via dev scripts), Tauri attempts to load the user interface from the development server URL defined in `tauri.conf.json` (`http://127.0.0.1:5173`). If the Vite dev server is not running on port 5173, the app window will be blank and show a `Connection refused` error.
* **Release Profile**: In release mode (`cargo build --release`), Tauri bundles the compiled UI assets from `ui/dist` directly into the binary and loads them locally, making it independent of any running dev server.

### Resolution
Ensure you build the application in production mode:
1. **Build the UI first**:
   ```bash
   cd ui
   pnpm install
   pnpm build
   ```
2. **Compile the Tauri app with the release profile**:
   ```bash
   cd src-tauri
   cargo build --release
   ```
3. If running in development mode, always start the Vite dev server first:
   ```bash
   cd ui
   pnpm dev
   # In another terminal:
   cd src-tauri
   cargo tauri dev
   ```

---

## Cause 3: Browser Extension / Client Page Cache

### Explanation
If the agent daemon is restarted, some web applications or browser extensions (like the UID extension) do not automatically perform a retry loop or may keep displaying the cached connection error page.

### Resolution
* **Refresh the Page**: Hard-refresh the webpage (`Ctrl + F5` or `Cmd + Shift + R`) where the signature or login is initiated.
* **Reload the Extension**: Go to `chrome://extensions`, locate the UID extension, and toggle it off and back on to force a reconnection.

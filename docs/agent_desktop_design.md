# UID Agent Desktop: Architectural & UX/UI Design Plan

This document outlines the design and feature specifications for the **UID Agent Desktop** application, drawing inspiration from industry-standard security and certificate management tools (Cloudflare WARP, Zscaler, 1Password, and SafeNet Token Manager).

---

## 1. UX/UI Layout Architecture (Sidebar Model)

The interface is structured as a premium, modern dashboard utilizing a left-hand navigation sidebar (56px collapsed / 240px expanded) with smooth micro-animations, glassmorphism panel styles, and a responsive main content area.

```
+------------------------------------------------------------------------+
|  Logo  |  [CONNECTED] UID Agent Status: Secure                         |
|--------+---------------------------------------------------------------|
| ( ) DB |  Device Posture: Compliant                                    |
| (x) SP |  Active USB Token: Viettel-CA (Serial: 98AF8762)              |
| (v) CM |                                                               |
| (o) AL |  +--------------------------+  +---------------------------+  |
|        |  | Posture Checklist (6/6)  |  | Active Certificate        |  |
| (s) ST |  | [v] TPM 2.0 Enabled      |  | CN: CONG TY TNHH TRIP     |  |
|        |  | [v] OS Up to date        |  | Issuer: Viettel-CA        |  |
|        |  | [v] Firewall Active      |  | Expiry: Dec 12, 2027      |  |
|        |  +--------------------------+  +---------------------------+  |
+------------------------------------------------------------------------+
```

---

## 2. Core View Modules

### View A: Dashboard (Status & Active Bridge)
The primary viewport showing real-time connectivity status.
*   **Visual Status Indicator**: Large glowing shield badge indicating status:
    *   `Secure / Compliant` (Green)
    *   `Risk Detected` (Yellow/Red)
    *   `Connecting / Syncing` (Pulse Blue)
*   **Active Hardware Overview**: Real-time display of currently inserted USB Tokens, Smart Cards, or keychains.
*   **Quick Actions**: Button to check for updates, verify connection to `api.uid.one`, or run a diagnostic ping test.

### View B: Security Posture (Zero-Trust Compliance)
A dedicated compliance checklist showing the system's security status.
*   **TPM & Secure Boot Check**: Shows whether Trusted Platform Module 2.0 and Secure Boot are active.
*   **OS & Update Compliance**: Shows the current operating system patch level.
*   **Firewall & Disk Encryption**: Queries BitLocker (Windows) / LUKS (Linux) / FileVault (macOS) state.
*   **Malware & EDR Status**: Detects active antivirus/EDR systems running on the client.

### View C: USB Token & Certificate Manager
A dedicated console modeled after SafeNet Authentication Client, providing hardware diagnostic controls.
*   **Token Info Panel**: Reads hardware specs (Manufacturer, Card Serial, Reader Name, Middleware Driver).
*   **Certificate Viewer**: Lists all certificates stored in the hardware token:
    *   Subject / CN (Common Name)
    *   Issuer CA
    *   Validity Period (Issue & Expiration dates)
    *   Key Usage (Digital Signature, Non-Repudiation, Document Signing)
*   **PIN & Driver Operations**: Quick links to change the USB token PIN or download official CSP/PKCS#11 drivers.

### View D: Activity Logs & Audit Trails
A developer-friendly diagnostic interface showing real-time local API logs.
*   **Local Server Requests**: Logs requests coming from Chrome/Firefox via `http://127.0.0.1:8001` (e.g. `GET /v1/auth/certificates`, `POST /v1/auth/sign`).
*   **Audit Trail**: Records when keys were used, when USB tokens were plugged in/out, and posture scans were triggered.
*   **Download logs**: One-click download of `desktop.log` for corporate IT support debugging.

### View E: Settings & Browser Integration
Allows advanced customization and browser extension pairing.
*   **Extension Manager**: Status checks for Chrome, Edge, Safari, and Firefox extensions, with interactive "Install/Repair" buttons.
*   **Custom API Endpoint Configuration**: Allows B2B enterprise clients to point the agent to private self-hosted nodes.
*   **Auto-start Options**: Settings to toggle launching the app hidden in the system tray on boot.

---

## 3. Recommended Back-end & Local Daemon Tasks

To support these views, the Tauri Rust core needs background threads executing the following scheduled tasks:
1.  **Hardware Polling Thread**: Every 2 seconds, monitors USB VID/PID changes to instantly trigger certificate store refresh upon USB Token insertion/removal.
2.  **Continuous Posture Scanning**: A low-priority background worker checking system security metrics every 60 seconds, updating the local state and notifying the companion browser extension if a risk is detected.
3.  **Local API Server (`127.0.0.1:8001`)**: Secure server accepting requests only from local browser origins (`chrome-extension://...` or verified corporate web portals) to perform operations.

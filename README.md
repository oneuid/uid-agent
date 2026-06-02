# UID Agent

A cross-platform endpoint security agent written in Rust, providing continuous session integrity monitoring and hardware-bound identity attestation.

**Supported platforms:** Linux · macOS · Windows

For detailed architectural specifications, see the [UID Platform Specifications](../uid-web/docs/uid-platform-specs.md).

## Platform Support

| Platform | Disk Encryption | Firewall | Kernel/OS | Notes |
|----------|----------------|----------|-----------|-------|
| Linux    | LUKS / dm-crypt | ufw / iptables | `/proc` + `/sys` | eBPF features on Kernel 5.8+ |
| macOS    | FileVault 2     | Application Firewall (socketfilterfw) | `sw_vers` / `sysctl` | Requires macOS 12+ |
| Windows  | BitLocker       | Windows Defender Firewall | PowerShell / WMI | Requires Windows 10+ |

## Prerequisites

- [Rust](https://rustup.rs/) (1.75+)
- **Linux only:** `libbpf` and `clang` (for eBPF probes, Kernel 5.8+)
- **macOS:** No extra deps. Xcode CLT recommended.
- **Windows:** PowerShell 5.1+ (pre-installed on Windows 10/11)

## Getting Started

1. **Clone the repository**
   ```bash
   git clone <repo-url>
   cd uid-agent
   ```

2. **Build the agent**
   ```bash
   # Native platform
   cargo build --release

   # Cross-compile for Windows (from Linux/macOS):
   # rustup target add x86_64-pc-windows-gnu
   # cargo build --release --target x86_64-pc-windows-gnu
   ```

3. **Run**
   ```bash
   # Linux/macOS
   ./target/release/uid-agent posture

   # Windows
   .\target\release\uid-agent.exe posture
   ```

## Commands

```
uid-agent register    — Generate hardware-bound Ed25519 keypair
uid-agent posture     — Collect device compliance posture (SOC 2 evidence)
uid-agent sign <data> — Cryptographically sign a payload
uid-agent daemon      — Run background SSH agent socket
uid-agent approve <token> — Approve an authentication challenge
```

## Development

- eBPF probes (Linux only) go in `src/bpf/`
- Platform implementations: `src/posture.rs` uses `#[cfg(target_os)]` compile-time dispatch

## License

[Apache License 2.0](LICENSE) — Open source for complete transparency in how endpoint attestation is handled.

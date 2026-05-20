# UID Agent

A system-level endpoint security agent written in Rust, leveraging eBPF to provide continuous session integrity monitoring and hardware-bound identity attestation.

For detailed architectural specifications, please see the [UID Platform Specifications](../uid-web/docs/uid-platform-specs.md).

## Prerequisites
- [Rust](https://rustup.rs/) (1.75+)
- Linux Kernel 5.8+ (for eBPF features)
- `libbpf` and `clang` (for compiling eBPF probes)

## Getting Started

1. **Clone the repository**
   ```bash
   git clone <repo-url>
   cd uid-agent
   ```

2. **Build the agent**
   ```bash
   cargo build --release
   ```

3. **Run (Requires Root)**
   ```bash
   sudo ./target/release/uid-agent
   ```

## Development
- eBPF probes should be placed in `src/bpf/`.
- User-space agent logic is in `src/`.

## License

This project is licensed under the [Apache License 2.0](LICENSE). 

By open-sourcing our endpoint agent, we ensure complete transparency in how eBPF probes and hardware attestations are handled at the OS level. We welcome community audits and contributions.

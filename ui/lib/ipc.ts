export interface UserProfile {
  token: string;
  name: string;
  email: string;
  avatar?: string;
}

export interface Certificate {
  id: string;
  subject: string;
  issuer: string;
  valid_from: string;
  valid_to: string;
  serial: string;
  usage: string;
  active: boolean;
  algorithm?: string;
  driver?: string;
  token_label?: string;
}

export interface Posture {
  os_family: string;
  os_release: string;
  hostname: string;
  disk_encrypted: boolean;
  firewall_status: string;
  secure_boot: boolean;
  screen_lock_active: boolean;
  ssh_keys_secure: boolean;
  vpn_active: boolean;
  anti_keylogger_active: boolean;
}

export interface TpmInfo {
  present: boolean;
  version: string;
  vendor: string;
  type: string;
  attestation_support: string;
}

export interface DaemonStatus {
  running: boolean;
  port: number;
  extension_paired: boolean;
  requests_handled: number;
  uptime_seconds: number;
}

export interface SignatureLog {
  id: string;
  timestamp: string;
  domain: string;
  type: string;
  hash: string;
  status: 'APPROVED' | 'REJECTED' | 'EXPIRED';
}

// Check if running inside native Tauri runtime
export function isTauri(): boolean {
  if (typeof window === 'undefined') return false;
  return '__TAURI_INTERNALS__' in window || '__TAURI__' in window;
}

// Dynamically invoke Tauri command or return fallback mock data when previewing in browser
export async function invokeCommand<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (isTauri()) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      return await invoke<T>(command, args);
    } catch (err) {
      console.warn(`[Tauri IPC] Error invoking "${command}":`, err);
      throw err;
    }
  }

  // Graceful browser mock data for instant web preview / styling
  return getMockData<T>(command);
}

function getMockData<T>(command: string): T {
  switch (command) {
    case 'get_user_profile':
      return {
        token: 'mock-jwt-token-session-proof',
        name: 'Alexandre Morgan',
        email: 'alex.morgan@uid.one',
        avatar: ''
      } as unknown as T;

    case 'get_app_version':
      return '3.0.16 (Sovereign Core)' as unknown as T;

    case 'get_posture':
      return {
        os_family: 'Linux (Ubuntu 24.04 LTS)',
        os_release: '6.8.0-generic',
        hostname: 'sovereign-node-01',
        disk_encrypted: true,
        firewall_status: 'active',
        secure_boot: true,
        screen_lock_active: true,
        ssh_keys_secure: true,
        vpn_active: true,
        anti_keylogger_active: true
      } as unknown as T;

    case 'get_certificates':
      return [
        {
          id: 'cert-vnpt-01',
          subject: 'CN=NGUYEN VAN AN (0101234567-001)',
          issuer: 'VNPT-CA Global Qualified Root CA',
          valid_from: '2024-01-15T00:00:00Z',
          valid_to: '2027-01-15T23:59:59Z',
          serial: '54:02:11:AB:89:C4:F3:10',
          usage: 'Digital Signature, Non-Repudiation, Legal Invoicing',
          active: true,
          algorithm: 'RSA 2048-bit (SHA-256)',
          driver: 'libvnpt-ca.so',
          token_label: 'VNPT-CA USB Token HSM'
        },
        {
          id: 'cert-yubi-02',
          subject: 'CN=Alexandre Morgan (PIV Slot 9a)',
          issuer: 'SHAN Sovereign Enterprise Root CA',
          valid_from: '2025-06-01T08:00:00Z',
          valid_to: '2028-06-01T08:00:00Z',
          serial: '9F:12:34:56:78:9A:BC:DE',
          usage: 'FIDO2 / PIV Client Authentication',
          active: true,
          algorithm: 'ECC NIST P-256 (secp256r1)',
          driver: 'libykcs11.so',
          token_label: 'YubiKey 5 NFC (PKCS#11)'
        }
      ] as unknown as T;

    case 'get_tpm_info':
      return {
        present: true,
        version: '2.0',
        vendor: 'INTC (Intel Corporation)',
        type: 'fTPM (Intel Platform Trust Technology)',
        attestation_support: 'Hardware Enclave Bound (Level 3)'
      } as unknown as T;

    case 'check_daemon_status':
      return {
        running: true,
        port: 13013,
        extension_paired: true,
        requests_handled: 148,
        uptime_seconds: 86400
      } as unknown as T;

    case 'get_signature_history':
      return [
        {
          id: 'sig-01',
          timestamp: '2026-10-12T08:45:10Z',
          domain: 'tax.gdt.gov.vn',
          type: 'PDF Electronic Tax Invoice',
          hash: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
          status: 'APPROVED'
        },
        {
          id: 'sig-02',
          timestamp: '2026-10-12T07:12:33Z',
          domain: 'mail.google.com',
          type: 'Email Body Signature (RFC 9449)',
          hash: 'a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e',
          status: 'APPROVED'
        }
      ] as unknown as T;

    case 'logout_user':
    case 'remediate_firewall':
    case 'remediate_screen_lock':
    case 'pin_to_dock':
    case 'open_browser_url':
      return null as unknown as T;

    default:
      return null as unknown as T;
  }
}

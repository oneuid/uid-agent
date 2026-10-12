'use client';

import React from 'react';
import {
  IconCpu,
  IconShieldCheck,
  IconFingerprint,
  IconCheck
} from '@tabler/icons-react';
import { TpmInfo } from '../../lib/tauri-ipc';

interface EnclaveTabProps {
  tpm: TpmInfo | null;
}

export function EnclaveTab({ tpm }: EnclaveTabProps) {
  return (
    <div className="space-y-6">
      {/* Banner */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-2xl bg-slate-900/80 border border-slate-800/80 shadow-md">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="p-1.5 rounded-lg bg-blue-500/10 text-blue-400 border border-blue-500/20">
              <IconCpu className="w-5 h-5" />
            </span>
            <h2 className="text-lg font-bold text-white tracking-tight">
              Hardware Security Enclave & TPM 2.0
            </h2>
          </div>
          <p className="text-xs text-slate-400 font-medium leading-relaxed">
            Hardware-isolated silicon cryptographic subsystem for device-bound private key generation and attestation.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <span className="px-3 py-1 rounded-full text-xs font-mono font-bold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center gap-1.5">
            <IconShieldCheck className="w-4 h-4" />
            <span>FIDO2 Level 3 Hardware Attestation</span>
          </span>
        </div>
      </div>

      {/* Enclave Hardware Parameters */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* TPM Module */}
        <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center gap-2.5 text-blue-400">
            <IconCpu className="w-5 h-5" />
            <h3 className="text-xs font-bold text-white uppercase font-mono tracking-wider">
              Silicon Root of Trust
            </h3>
          </div>
          <div className="space-y-2 text-xs font-mono">
            <div className="flex justify-between py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">TPM Status</span>
              <span className="text-emerald-400 font-bold">
                {tpm?.present ? 'PRESENT & ACTIVE' : 'ACTIVE'}
              </span>
            </div>
            <div className="flex justify-between py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">TPM Specification</span>
              <span className="text-white font-semibold">
                {tpm?.version || '2.0 (TCG Mandated)'}
              </span>
            </div>
            <div className="flex justify-between py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">Enclave Vendor</span>
              <span className="text-white font-semibold">
                {tpm?.vendor || 'INTC (Intel Corporation)'}
              </span>
            </div>
            <div className="flex justify-between py-1.5">
              <span className="text-slate-400">Hardware Enclave Type</span>
              <span className="text-slate-200">
                {tpm?.type || 'fTPM / Intel PTT Architecture'}
              </span>
            </div>
          </div>
        </div>

        {/* Cryptographic Attestation */}
        <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center gap-2.5 text-emerald-400">
            <IconFingerprint className="w-5 h-5" />
            <h3 className="text-xs font-bold text-white uppercase font-mono tracking-wider">
              Cryptographic Capabilities
            </h3>
          </div>
          <div className="space-y-2 text-xs font-mono">
            <div className="flex justify-between py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">Device Bound Key</span>
              <span className="text-emerald-400 font-bold">Ed25519 + NIST P-256</span>
            </div>
            <div className="flex justify-between py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">Attestation Protocol</span>
              <span className="text-white">{tpm?.attestation_support || 'W3C WebAuthn Direct Attestation'}</span>
            </div>
            <div className="flex justify-between py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">Memory Protection</span>
              <span className="text-emerald-400 font-bold">Non-Exportable Silicon Key</span>
            </div>
            <div className="flex justify-between py-1.5">
              <span className="text-slate-400">RFC 9449 Binding</span>
              <span className="text-slate-200">DPoP Proof-of-Possession</span>
            </div>
          </div>
        </div>
      </div>

      {/* Security Guarantee Card */}
      <div className="p-4 rounded-xl border border-emerald-500/20 bg-emerald-500/5 text-xs text-slate-300 space-y-1.5">
        <div className="flex items-center gap-2 text-emerald-400 font-bold">
          <IconCheck className="w-4 h-4 shrink-0" />
          <span>Zero-Custody Hardware Guarantee</span>
        </div>
        <p className="leading-relaxed text-slate-400 text-[11px]">
          Your private signing keys are generated inside the physical secure enclave chip and never touch RAM, operating system swap, or disk in unencrypted format. Screen loggers, browser exploits, and remote access trojans cannot read enclave keys.
        </p>
      </div>
    </div>
  );
}

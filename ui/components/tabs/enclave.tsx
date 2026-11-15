'use client';

import React from 'react';
import {
  IconCpu,
  IconShieldCheck,
  IconFingerprint,
  IconCheck
} from '@tabler/icons-react';
import { TpmInfo } from '../../lib/ipc';
import { useI18n } from '../../lib/i18n';

interface EnclaveTabProps {
  tpm: TpmInfo | null;
}

export function EnclaveTab({ tpm }: EnclaveTabProps) {
  const { t } = useI18n();
  const isPresent = Boolean(tpm?.present);

  return (
    <div className="space-y-6">
      {/* Banner */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-2xl bg-slate-900/80 border border-slate-800/80 shadow-md">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="p-1.5 rounded-lg bg-blue-500/10 text-blue-400 border border-blue-500/20">
              <IconCpu className="w-5 h-5" />
            </span>
            <h2 className="text-lg font-sans font-bold text-white tracking-tight">
              {t('enclaveTitle', 'Hardware security enclave')}
            </h2>
          </div>
          <p className="text-xs text-slate-400 font-medium leading-relaxed">
            {t('enclaveSubtitle', 'Hardware-isolated cryptographic subsystem for device-bound key generation and attestation.')}
          </p>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <span className={`px-3 py-1 rounded-full text-xs font-sans font-semibold border flex items-center gap-1.5 ${
            isPresent
              ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
              : 'bg-slate-800 text-slate-400 border-slate-700/60'
          }`}>
            <IconShieldCheck className="w-4 h-4" />
            <span>
              {!tpm
                ? '—'
                : isPresent
                  ? t('fidoAttestation', 'FIDO2 Level 3 attestation')
                  : t('enclaveUnavailable', 'TPM 2.0 not detected')}
            </span>
          </span>
        </div>
      </div>

      {/* Enclave Hardware Parameters */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* TPM Module */}
        <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center gap-2.5 text-blue-400">
            <IconCpu className="w-5 h-5" />
            <h3 className="text-sm font-sans font-semibold text-white">
              {t('siliconRootTitle', 'Silicon root of trust')}
            </h3>
          </div>
          <div className="space-y-2 text-xs font-sans">
            <div className="flex justify-between gap-2 py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">{t('tpmStatus', 'TPM status')}</span>
              <span className={isPresent ? 'text-emerald-400 font-semibold' : 'text-slate-400 font-medium'}>
                {!tpm
                  ? '—'
                  : isPresent
                    ? t('tpmPresentActive', 'Present and active')
                    : t('notDetected', 'Not detected')}
              </span>
            </div>
            <div className="flex justify-between gap-2 py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">{t('tpmSpec', 'TPM specification')}</span>
              <span className="text-white font-mono tabular-nums font-semibold">
                {isPresent && tpm?.version ? tpm.version : '—'}
              </span>
            </div>
            <div className="flex justify-between gap-2 py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">{t('enclaveVendor', 'Enclave vendor')}</span>
              <span className="text-white font-mono font-semibold text-end">
                {isPresent && tpm?.vendor ? tpm.vendor : '—'}
              </span>
            </div>
            <div className="flex justify-between gap-2 py-1.5">
              <span className="text-slate-400">{t('enclaveType', 'Hardware enclave type')}</span>
              <span className="text-slate-200 font-mono text-end">
                {isPresent && tpm?.type ? tpm.type : '—'}
              </span>
            </div>
          </div>
        </div>

        {/* Cryptographic Attestation */}
        <div className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center gap-2.5 text-emerald-400">
            <IconFingerprint className="w-5 h-5" />
            <h3 className="text-sm font-sans font-semibold text-white">
              {t('cryptoCapsTitle', 'Cryptographic capabilities')}
            </h3>
          </div>
          <div className="space-y-2 text-xs font-sans">
            <div className="flex justify-between gap-2 py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">{t('deviceBoundKey', 'Device-bound key')}</span>
              <span className="text-emerald-400 font-mono font-semibold">
                {isPresent ? 'Ed25519 + NIST P-256' : '—'}
              </span>
            </div>
            <div className="flex justify-between gap-2 py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">{t('attestationProto', 'Attestation protocol')}</span>
              <span className="text-white text-end">
                {isPresent && tpm?.attestation_support ? tpm.attestation_support : '—'}
              </span>
            </div>
            <div className="flex justify-between gap-2 py-1.5 border-b border-slate-800/60">
              <span className="text-slate-400">{t('memoryProtection', 'Memory protection')}</span>
              <span className={isPresent ? 'text-emerald-400 font-semibold' : 'text-slate-400'}>
                {isPresent ? t('nonExportableKey', 'Non-exportable silicon key') : '—'}
              </span>
            </div>
            <div className="flex justify-between gap-2 py-1.5">
              <span className="text-slate-400">{t('dpopBinding', 'RFC 9449 binding')}</span>
              <span className="text-slate-200">
                {t('dpopDesc', 'DPoP proof of possession')}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Security Guarantee Card */}
      <div className="p-4 rounded-xl border border-emerald-500/20 bg-emerald-500/5 text-xs text-slate-300 space-y-1.5">
        <div className="flex items-center gap-2 text-emerald-400 font-semibold">
          <IconCheck className="w-4 h-4 shrink-0" />
          <span>{t('zeroCustodyTitle', 'Zero-custody hardware guarantee')}</span>
        </div>
        <p className="leading-relaxed text-slate-400 text-[11px]">
          {t('zeroCustodyDesc', 'Private signing keys are generated inside the secure hardware enclave and never leave silicon in plaintext.')}
        </p>
      </div>
    </div>
  );
}

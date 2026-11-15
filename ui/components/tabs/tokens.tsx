'use client';

import React from 'react';
import {
  IconDeviceUsb,
  IconCertificate,
  IconShieldCheck,
  IconCopy,
  IconCheck,
  IconAlertCircle,
  IconRefresh
} from '@tabler/icons-react';
import { Certificate } from '../../lib/ipc';
import { useI18n } from '../../lib/i18n';
import { toast } from 'sonner';

interface UsbTokensTabProps {
  certs: Certificate[];
  onRefresh: () => void;
  isLoading: boolean;
}

export function UsbTokensTab({ certs, onRefresh, isLoading }: UsbTokensTabProps) {
  const { t } = useI18n();
  const [copiedId, setCopiedId] = React.useState<string | null>(null);

  const copySerial = (serial: string, id: string) => {
    navigator.clipboard.writeText(serial);
    setCopiedId(id);
    toast.success(t('copiedClipboard', 'Copied certificate serial to clipboard.'));
    setTimeout(() => setCopiedId(null), 2000);
  };

  const hasTokens = certs.length > 0;

  return (
    <div className="space-y-6">
      {/* Header Banner */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-2xl bg-slate-900/80 border border-slate-800/80 shadow-md">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="p-1.5 rounded-lg bg-blue-500/10 text-blue-400 border border-blue-500/20">
              <IconDeviceUsb className="w-5 h-5" />
            </span>
            <h2 className="text-lg font-sans font-bold text-white tracking-tight">
              {t('tokensTitle', 'Hardware security tokens')}
            </h2>
          </div>
          <p className="text-xs text-slate-400 font-medium leading-relaxed">
            {t('tokensSubtitle', 'Attested PKCS#11 smart cards and USB tokens connected to this device.')}
          </p>
        </div>

        <div className="flex items-center gap-3 shrink-0">
          <div className={`px-3 py-1 rounded-full text-xs font-sans font-semibold border flex items-center gap-1.5 tabular-nums ${
            hasTokens
              ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
              : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
          }`}>
            <span className={`w-2 h-2 rounded-full ${hasTokens ? 'bg-emerald-400 animate-pulse' : 'bg-amber-400'}`}></span>
            <span>
              {hasTokens
                ? `${certs.length} ${t('activeTokens', 'active tokens')}`
                : t('statusNotDetected', 'No token inserted')}
            </span>
          </div>

          <button
            type="button"
            onClick={onRefresh}
            disabled={isLoading}
            className="inline-flex items-center gap-1.5 h-8 px-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 hover:text-white transition-colors duration-150 border border-slate-700/60 cursor-pointer disabled:opacity-50"
          >
            <IconRefresh className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />
            <span>{t('scanBus', 'Scan bus')}</span>
          </button>
        </div>
      </div>

      {/* Token List or Empty State */}
      {hasTokens ? (
        <div className="space-y-4">
          {certs.map((cert) => (
            <div
              key={cert.id}
              className="p-5 rounded-2xl bg-slate-900/60 border border-slate-800/90 hover:border-slate-700/80 transition-colors duration-150 shadow-sm space-y-4"
            >
              {/* Token Title & Badges */}
              <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-slate-800/70">
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400 shrink-0">
                    <IconCertificate className="w-4 h-4" />
                  </div>
                  <div>
                    <h3 className="text-sm font-bold text-white font-sans leading-tight">
                      {cert.token_label || cert.subject.split(',')[0]}
                    </h3>
                    <p className="text-[11px] text-slate-400 font-mono mt-0.5">
                      {t('driverLabel', 'Driver')}: {cert.driver || 'PKCS#11'}
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-2 flex-wrap">
                  <span className="px-2.5 py-0.5 rounded-full bg-blue-500/10 text-blue-400 border border-blue-500/20 text-[10px] font-mono font-bold tabular-nums">
                    {cert.algorithm || 'RSA 2048-bit'}
                  </span>
                  <span className="px-2.5 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-[10px] font-sans font-semibold flex items-center gap-1">
                    <IconShieldCheck className="w-3 h-3" />
                    <span>{t('tokenAttested', 'Attested HSM')}</span>
                  </span>
                </div>
              </div>

              {/* Certificate Metadata Grid — flattened inner wells without stacked inner borders */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs">
                <div className="space-y-1">
                  <span className="text-xs font-sans font-medium text-slate-400">
                    {t('tokenSubject', 'Subject')}
                  </span>
                  <p className="text-slate-200 font-mono font-medium break-all bg-slate-950/60 p-2.5 rounded-xl">
                    {cert.subject}
                  </p>
                </div>

                <div className="space-y-1">
                  <span className="text-xs font-sans font-medium text-slate-400">
                    {t('tokenIssuer', 'Issuer')}
                  </span>
                  <p className="text-slate-200 font-mono font-medium break-all bg-slate-950/60 p-2.5 rounded-xl">
                    {cert.issuer}
                  </p>
                </div>

                <div className="space-y-1">
                  <span className="text-xs font-sans font-medium text-slate-400">
                    {t('tokenSerial', 'Serial number')}
                  </span>
                  <div className="flex items-center justify-between bg-slate-950/60 px-2.5 py-1.5 rounded-xl">
                    <span className="text-slate-200 font-mono tabular-nums font-medium text-[11px] truncate pe-2">
                      {cert.serial}
                    </span>
                    <button
                      type="button"
                      onClick={() => copySerial(cert.serial, cert.id)}
                      aria-label={t('copySerial', 'Copy serial number')}
                      title={t('copySerial', 'Copy serial number')}
                      className="h-7 w-7 inline-flex items-center justify-center rounded-lg hover:bg-slate-800 text-slate-400 hover:text-white transition-colors duration-150 cursor-pointer shrink-0"
                    >
                      {copiedId === cert.id ? <IconCheck className="w-3.5 h-3.5 text-emerald-400" /> : <IconCopy className="w-3.5 h-3.5" />}
                    </button>
                  </div>
                </div>

                <div className="space-y-1">
                  <span className="text-xs font-sans font-medium text-slate-400">
                    {t('validityPeriod', 'Validity period')}
                  </span>
                  <div className="bg-slate-950/60 px-2.5 py-2 rounded-xl text-[11px] font-mono tabular-nums text-slate-300 flex items-center justify-between min-h-[40px]">
                    <span>{new Date(cert.valid_from).toLocaleDateString()}</span>
                    <span className="text-slate-500">–</span>
                    <span className="text-emerald-400 font-bold">{new Date(cert.valid_to).toLocaleDateString()}</span>
                  </div>
                </div>
              </div>

              {/* Usage Footer */}
              <div className="pt-2 text-[11px] text-slate-400 flex flex-col sm:flex-row sm:items-center justify-between gap-2 font-medium">
                <span className="text-slate-400 font-sans">
                  {cert.usage || t('tokenUsageDesc', 'Digital signature, authenticity, non-repudiation')}
                </span>
                <span className="text-emerald-400 font-sans text-xs font-semibold flex items-center gap-1">
                  <IconCheck className="w-3.5 h-3.5" />
                  <span>{t('bridgeReady', 'Ready for extension bridge')}</span>
                </span>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-center py-16 px-4 rounded-2xl border border-dashed border-slate-800/90 bg-slate-900/30 space-y-3">
          <div className="w-12 h-12 rounded-2xl bg-slate-800/60 border border-slate-700/60 flex items-center justify-center text-slate-400 mx-auto">
            <IconAlertCircle className="w-6 h-6" />
          </div>
          <h3 className="text-sm font-sans font-bold text-white">
            {t('noTokens', 'No USB security tokens detected')}
          </h3>
          <p className="text-xs text-slate-400 max-w-md mx-auto leading-relaxed">
            {t('insertTokenDesc', 'Insert a PKCS#11 hardware token or smart card to sign documents, tax filings, or authenticate sessions.')}
          </p>
          <div className="pt-2">
            <button
              type="button"
              onClick={onRefresh}
              className="inline-flex items-center justify-center h-9 px-4 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold shadow-xs transition-[background-color,transform] duration-150 cursor-pointer active:scale-[0.98]"
            >
              {t('refreshPorts', 'Refresh USB ports')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

'use client';

import React from 'react';
import {
  IconShieldCheck,
  IconLock,
  IconFlame,
  IconCpu,
  IconDeviceDesktop,
  IconKey,
  IconFileDownload,
  IconCheck,
  IconAlertTriangle,
  IconTool
} from '@tabler/icons-react';
import { Posture, invokeCommand } from '../../lib/ipc';
import { useI18n } from '../../lib/i18n';
import { toast } from 'sonner';

interface PostureTabProps {
  posture: Posture | null;
  onRefresh: () => void;
}

export function PostureTab({ posture, onRefresh }: PostureTabProps) {
  const { t } = useI18n();
  const [remediating, setRemediating] = React.useState<string | null>(null);

  const compliantCount = posture
    ? (posture.disk_encrypted ? 1 : 0) +
      (posture.firewall_status === 'active' ? 1 : 0) +
      (posture.secure_boot ? 1 : 0) +
      (posture.screen_lock_active ? 1 : 0) +
      (posture.ssh_keys_secure ? 1 : 0) +
      (posture.anti_keylogger_active ? 1 : 0)
    : 0;

  const totalChecks = 6;
  const isAllCompliant = Boolean(posture) && compliantCount === totalChecks;

  const handleRemediateFirewall = async () => {
    setRemediating('firewall');
    try {
      await invokeCommand('remediate_firewall');
      toast.success(t('firewallFixed', 'System firewall enabled.'));
      onRefresh();
    } catch (e) {
      toast.error(`${t('firewallFixFailed', 'Failed to configure firewall')}: ${String(e)}`);
    } finally {
      setRemediating(null);
    }
  };

  const handleRemediateScreenLock = async () => {
    setRemediating('screen_lock');
    try {
      await invokeCommand('remediate_screen_lock');
      toast.success(t('screenLockFixed', 'Screen lock timeout enforced.'));
      onRefresh();
    } catch (e) {
      toast.error(`${t('screenLockFixFailed', 'Failed to configure screen lock')}: ${String(e)}`);
    } finally {
      setRemediating(null);
    }
  };

  const handleExportJson = () => {
    if (!posture) return;
    const report = {
      device_node: {
        hostname: posture.hostname,
        os_family: posture.os_family,
        os_release: posture.os_release,
        timestamp: new Date().toISOString()
      },
      soc2_controls: {
        disk_encryption: posture.disk_encrypted ? 'PASS' : 'FAIL',
        firewall: posture.firewall_status === 'active' ? 'PASS' : 'FAIL',
        secure_boot: posture.secure_boot ? 'PASS' : 'FAIL',
        screen_lock_enforcement: posture.screen_lock_active ? 'PASS' : 'FAIL',
        ssh_keys_protected: posture.ssh_keys_secure ? 'PASS' : 'FAIL',
        anti_keylogger_enclave: posture.anti_keylogger_active ? 'PASS' : 'FAIL'
      },
      compliance_status: isAllCompliant ? 'COMPLIANT' : 'ATTENTION_REQUIRED'
    };

    const blob = new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `uid-soc2-audit-${posture.hostname}-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
    toast.success(t('auditDownloaded', 'SOC 2 audit report downloaded.'));
  };

  return (
    <div className="space-y-6">
      {/* Banner */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 p-5 rounded-2xl bg-slate-900/80 border border-slate-800/80 shadow-md">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <span className="p-1.5 rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
              <IconShieldCheck className="w-5 h-5" />
            </span>
            <h2 className="text-lg font-sans font-bold text-white tracking-tight">
              {t('postureTitle', 'System security posture')}
            </h2>
          </div>
          <p className="text-xs text-slate-400 font-medium leading-relaxed">
            {t('postureSubtitle', 'Real-time validation of device security controls and SOC 2 compliance policies.')}
          </p>
        </div>

        <div className="flex items-center gap-3 shrink-0">
          <div className={`px-3 py-1 rounded-full text-xs font-sans font-semibold border flex items-center gap-1.5 tabular-nums ${
            !posture
              ? 'bg-slate-800 text-slate-400 border-slate-700/60'
              : isAllCompliant
                ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
          }`}>
            {isAllCompliant ? <IconCheck className="w-3.5 h-3.5" /> : <IconAlertTriangle className="w-3.5 h-3.5" />}
            <span>
              {posture
                ? `${compliantCount} ${t('ofLabel', 'of')} ${totalChecks} ${t('controlsPass', 'controls pass')}`
                : '—'}
            </span>
          </div>

          <button
            type="button"
            onClick={handleExportJson}
            disabled={!posture}
            className="inline-flex items-center gap-1.5 h-8 px-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 hover:text-white transition-colors duration-150 border border-slate-700/60 cursor-pointer disabled:opacity-50"
          >
            <IconFileDownload className="w-3.5 h-3.5" />
            <span>{t('exportEvidence', 'Export evidence')}</span>
          </button>
        </div>
      </div>

      {/* Grid of Security Controls */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {/* 1. Host OS & Kernel */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between gap-2">
            <span className="p-2 rounded-xl bg-slate-800/80 text-blue-400 border border-slate-700/60">
              <IconDeviceDesktop className="w-4 h-4" />
            </span>
            <span className="px-2 py-0.5 rounded-full text-[10px] font-sans font-semibold bg-blue-500/10 text-blue-400 border border-blue-500/20">
              {posture ? t('activeHost', 'Active host') : '—'}
            </span>
          </div>
          <div className="space-y-1">
            <h4 className="text-sm font-sans font-semibold text-white">
              {t('osTitle', 'Operating system')}
            </h4>
            <p className="text-xs text-slate-300 font-medium truncate">
              {posture ? posture.os_family : '—'}
            </p>
            <p className="text-[11px] text-slate-400 font-mono tabular-nums truncate">
              {posture ? `${posture.hostname} · ${posture.os_release}` : '—'}
            </p>
          </div>
        </div>

        {/* 2. Disk Encryption */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between gap-2">
            <span className="p-2 rounded-xl bg-slate-800/80 text-emerald-400 border border-slate-700/60">
              <IconLock className="w-4 h-4" />
            </span>
            <span className={`px-2 py-0.5 rounded-full text-[10px] font-sans font-semibold border ${
              !posture
                ? 'bg-slate-800 text-slate-400 border-slate-700/60'
                : posture.disk_encrypted
                  ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                  : 'bg-red-500/10 text-red-400 border-red-500/20'
            }`}>
              {!posture
                ? '—'
                : posture.disk_encrypted
                  ? t('compliant', 'Compliant')
                  : t('notDetected', 'Not detected')}
            </span>
          </div>
          <div className="space-y-1">
            <h4 className="text-sm font-sans font-semibold text-white">
              {t('diskTitle', 'Disk encryption')}
            </h4>
            <p className="text-xs text-slate-300 font-medium">
              {!posture
                ? '—'
                : posture.disk_encrypted
                  ? t('diskEncryptedVal', 'Encrypted volume (LUKS, FileVault, BitLocker)')
                  : t('diskUnencryptedVal', 'Unencrypted volume')}
            </p>
            <p className="text-[11px] text-slate-400 font-medium">
              {t('diskDesc', 'Protects local cryptographic keys and data at rest.')}
            </p>
          </div>
        </div>

        {/* 3. System Firewall */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between gap-2">
            <span className="p-2 rounded-xl bg-slate-800/80 text-orange-400 border border-slate-700/60">
              <IconFlame className="w-4 h-4" />
            </span>
            <div className="flex items-center gap-1.5">
              <span className={`px-2 py-0.5 rounded-full text-[10px] font-sans font-semibold border ${
                !posture
                  ? 'bg-slate-800 text-slate-400 border-slate-700/60'
                  : posture.firewall_status === 'active'
                    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                    : 'bg-red-500/10 text-red-400 border-red-500/20'
              }`}>
                {!posture
                  ? '—'
                  : posture.firewall_status === 'active'
                    ? t('active', 'Active')
                    : t('disabled', 'Disabled')}
              </span>
              {posture && posture.firewall_status !== 'active' && (
                <button
                  type="button"
                  onClick={handleRemediateFirewall}
                  disabled={remediating === 'firewall'}
                  className="inline-flex items-center gap-1 h-7 px-2.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold transition-[background-color,transform] duration-150 active:scale-[0.98] cursor-pointer disabled:opacity-50"
                >
                  <IconTool className="w-3.5 h-3.5" />
                  <span>{t('fixAction', 'Fix')}</span>
                </button>
              )}
            </div>
          </div>
          <div className="space-y-1">
            <h4 className="text-sm font-sans font-semibold text-white">
              {t('firewallTitle', 'System firewall')}
            </h4>
            <p className="text-xs text-slate-300 font-medium">
              {t('firewallVal', 'Host packet filter')}
            </p>
            <p className="text-[11px] text-slate-400 font-medium">
              {t('firewallDesc', 'Blocks unauthorized inbound network connections.')}
            </p>
          </div>
        </div>

        {/* 4. Secure Boot */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between gap-2">
            <span className="p-2 rounded-xl bg-slate-800/80 text-cyan-400 border border-slate-700/60">
              <IconCpu className="w-4 h-4" />
            </span>
            <span className={`px-2 py-0.5 rounded-full text-[10px] font-sans font-semibold border ${
              !posture
                ? 'bg-slate-800 text-slate-400 border-slate-700/60'
                : posture.secure_boot
                  ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                  : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
            }`}>
              {!posture
                ? '—'
                : posture.secure_boot
                  ? t('enabled', 'Enabled')
                  : t('unverified', 'Unverified')}
            </span>
          </div>
          <div className="space-y-1">
            <h4 className="text-sm font-sans font-semibold text-white">
              {t('secureBootTitle', 'Secure boot')}
            </h4>
            <p className="text-xs text-slate-300 font-medium">
              {t('secureBootVal', 'UEFI hardware signature chain')}
            </p>
            <p className="text-[11px] text-slate-400 font-medium">
              {t('secureBootDesc', 'Prevents unauthorized bootloaders and unsigned kernel modules.')}
            </p>
          </div>
        </div>

        {/* 5. Screen Lock Auto-Timeout */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between gap-2">
            <span className="p-2 rounded-xl bg-slate-800/80 text-purple-400 border border-slate-700/60">
              <IconLock className="w-4 h-4" />
            </span>
            <div className="flex items-center gap-1.5">
              <span className={`px-2 py-0.5 rounded-full text-[10px] font-sans font-semibold border ${
                !posture
                  ? 'bg-slate-800 text-slate-400 border-slate-700/60'
                  : posture.screen_lock_active
                    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                    : 'bg-red-500/10 text-red-400 border-red-500/20'
              }`}>
                {!posture
                  ? '—'
                  : posture.screen_lock_active
                    ? t('enforced', 'Enforced')
                    : t('off', 'Off')}
              </span>
              {posture && !posture.screen_lock_active && (
                <button
                  type="button"
                  onClick={handleRemediateScreenLock}
                  disabled={remediating === 'screen_lock'}
                  className="inline-flex items-center gap-1 h-7 px-2.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold transition-[background-color,transform] duration-150 active:scale-[0.98] cursor-pointer disabled:opacity-50"
                >
                  <IconTool className="w-3.5 h-3.5" />
                  <span>{t('fixAction', 'Fix')}</span>
                </button>
              )}
            </div>
          </div>
          <div className="space-y-1">
            <h4 className="text-sm font-sans font-semibold text-white">
              {t('screenLockTitle', 'Screen lock timeout')}
            </h4>
            <p className="text-xs text-slate-300 font-medium tabular-nums">
              {t('screenLockVal', 'Auto-lock within 5 minutes')}
            </p>
            <p className="text-[11px] text-slate-400 font-medium">
              {t('screenLockDesc', 'Protects unattended workstations from physical access.')}
            </p>
          </div>
        </div>

        {/* 6. SSH Keys Passphrase */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between gap-2">
            <span className="p-2 rounded-xl bg-slate-800/80 text-yellow-400 border border-slate-700/60">
              <IconKey className="w-4 h-4" />
            </span>
            <span className={`px-2 py-0.5 rounded-full text-[10px] font-sans font-semibold border ${
              !posture
                ? 'bg-slate-800 text-slate-400 border-slate-700/60'
                : posture.ssh_keys_secure
                  ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                  : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
            }`}>
              {!posture
                ? '—'
                : posture.ssh_keys_secure
                  ? t('protected', 'Protected')
                  : t('unencrypted', 'Unencrypted')}
            </span>
          </div>
          <div className="space-y-1">
            <h4 className="text-sm font-sans font-semibold text-white">
              {t('sshTitle', 'SSH key protection')}
            </h4>
            <p className="text-xs text-slate-300 font-medium">
              {t('sshVal', 'Passphrase-protected keys')}
            </p>
            <p className="text-[11px] text-slate-400 font-medium">
              {t('sshDesc', 'Verifies local SSH keys require a passphrase.')}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

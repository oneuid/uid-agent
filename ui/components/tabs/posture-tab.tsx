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
import { Posture, invokeCommand } from '../../lib/tauri-ipc';
import { useI18n } from '../../lib/i18n';
import { toast } from 'sonner';

interface PostureTabProps {
  posture: Posture | null;
  onRefresh: () => void;
}

export function PostureTab({ posture, onRefresh }: PostureTabProps) {
  const { t } = useI18n();
  const [remediating, setRemediating] = React.useState<string | null>(null);

  const compliantCount = (posture?.disk_encrypted ? 1 : 0) +
    (posture?.firewall_status === 'active' ? 1 : 0) +
    (posture?.secure_boot ? 1 : 0) +
    (posture?.screen_lock_active ? 1 : 0) +
    (posture?.ssh_keys_secure ? 1 : 0) +
    (posture?.anti_keylogger_active ? 1 : 0);

  const totalChecks = 6;
  const isAllCompliant = compliantCount === totalChecks;

  const handleRemediateFirewall = async () => {
    setRemediating('firewall');
    try {
      await invokeCommand('remediate_firewall');
      toast.success('System firewall enabled successfully.');
      onRefresh();
    } catch (e) {
      toast.error('Failed to configure firewall: ' + String(e));
    } finally {
      setRemediating(null);
    }
  };

  const handleRemediateScreenLock = async () => {
    setRemediating('screen_lock');
    try {
      await invokeCommand('remediate_screen_lock');
      toast.success('Screen lock auto-timeout enforced.');
      onRefresh();
    } catch (e) {
      toast.error('Failed to configure screen lock: ' + String(e));
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
    toast.success('SOC 2 audit report downloaded.');
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
            <h2 className="text-lg font-bold text-white tracking-tight">
              {t('postureTitle', 'System Posture & SOC 2 Compliance')}
            </h2>
          </div>
          <p className="text-xs text-slate-400 font-medium leading-relaxed">
            {t('postureSubtitle', 'Real-time validation of device security controls and enterprise compliance policies.')}
          </p>
        </div>

        <div className="flex items-center gap-3">
          <div className={`px-3 py-1 rounded-full text-xs font-mono font-bold border flex items-center gap-1.5 ${
            isAllCompliant 
              ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' 
              : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
          }`}>
            {isAllCompliant ? <IconCheck className="w-3.5 h-3.5" /> : <IconAlertTriangle className="w-3.5 h-3.5" />}
            <span>{compliantCount}/{totalChecks} Controls Pass</span>
          </div>

          <button
            type="button"
            onClick={handleExportJson}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 hover:text-white transition-all border border-slate-700/60 cursor-pointer"
          >
            <IconFileDownload className="w-3.5 h-3.5" />
            <span>Export Evidence</span>
          </button>
        </div>
      </div>

      {/* Grid of Security Controls */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        
        {/* 1. Host OS & Kernel */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between">
            <span className="p-2 rounded-xl bg-slate-800/80 text-blue-400 border border-slate-700/60">
              <IconDeviceDesktop className="w-4 h-4" />
            </span>
            <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-blue-500/10 text-blue-400 border border-blue-500/20">
              Active Host
            </span>
          </div>
          <div className="space-y-1">
            <h4 className="text-xs font-bold text-white uppercase tracking-wider font-mono">
              Operating System
            </h4>
            <p className="text-xs text-slate-300 font-semibold truncate">
              {posture?.os_family || 'Linux'}
            </p>
            <p className="text-[11px] text-slate-400 font-mono truncate">
              {posture?.hostname || 'localhost'} • {posture?.os_release || '6.8.0'}
            </p>
          </div>
        </div>

        {/* 2. Disk Encryption */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between">
            <span className="p-2 rounded-xl bg-slate-800/80 text-emerald-400 border border-slate-700/60">
              <IconLock className="w-4 h-4" />
            </span>
            <span className={`px-2 py-0.5 rounded-full text-[10px] font-mono font-bold border ${
              posture?.disk_encrypted 
                ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' 
                : 'bg-red-500/10 text-red-400 border-red-500/20'
            }`}>
              {posture?.disk_encrypted ? 'COMPLIANT' : 'NOT DETECTED'}
            </span>
          </div>
          <div className="space-y-1">
            <h4 className="text-xs font-bold text-white uppercase tracking-wider font-mono">
              Disk Encryption
            </h4>
            <p className="text-xs text-slate-300 font-semibold">
              {posture?.disk_encrypted ? 'BitLocker / FileVault / LUKS' : 'Unencrypted Volume'}
            </p>
            <p className="text-[11px] text-slate-400 font-medium">
              Protects local cryptographic keys and data at rest.
            </p>
          </div>
        </div>

        {/* 3. System Firewall */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between">
            <span className="p-2 rounded-xl bg-slate-800/80 text-orange-400 border border-slate-700/60">
              <IconFlame className="w-4 h-4" />
            </span>
            <div className="flex items-center gap-1.5">
              <span className={`px-2 py-0.5 rounded-full text-[10px] font-mono font-bold border ${
                posture?.firewall_status === 'active' 
                  ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' 
                  : 'bg-red-500/10 text-red-400 border-red-500/20'
              }`}>
                {posture?.firewall_status === 'active' ? 'ACTIVE' : 'DISABLED'}
              </span>
              {posture?.firewall_status !== 'active' && (
                <button
                  type="button"
                  onClick={handleRemediateFirewall}
                  disabled={remediating === 'firewall'}
                  className="px-2 py-0.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-[10px] font-bold transition-all cursor-pointer"
                >
                  <IconTool className="w-3 h-3 inline mr-1" />
                  Fix
                </button>
              )}
            </div>
          </div>
          <div className="space-y-1">
            <h4 className="text-xs font-bold text-white uppercase tracking-wider font-mono">
              System Firewall
            </h4>
            <p className="text-xs text-slate-300 font-semibold">
              UFW / Windows Defender Firewall
            </p>
            <p className="text-[11px] text-slate-400 font-medium">
              Blocks unauthorized inbound network intrusions.
            </p>
          </div>
        </div>

        {/* 4. Secure Boot */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between">
            <span className="p-2 rounded-xl bg-slate-800/80 text-cyan-400 border border-slate-700/60">
              <IconCpu className="w-4 h-4" />
            </span>
            <span className={`px-2 py-0.5 rounded-full text-[10px] font-mono font-bold border ${
              posture?.secure_boot 
                ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' 
                : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
            }`}>
              {posture?.secure_boot ? 'ENABLED' : 'UNVERIFIED'}
            </span>
          </div>
          <div className="space-y-1">
            <h4 className="text-xs font-bold text-white uppercase tracking-wider font-mono">
              Secure Boot
            </h4>
            <p className="text-xs text-slate-300 font-semibold">
              UEFI Hardware Signature Chain
            </p>
            <p className="text-[11px] text-slate-400 font-medium">
              Prevents malicious bootkits and unsigned kernel modules.
            </p>
          </div>
        </div>

        {/* 5. Screen Lock Auto-Timeout */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between">
            <span className="p-2 rounded-xl bg-slate-800/80 text-purple-400 border border-slate-700/60">
              <IconLock className="w-4 h-4" />
            </span>
            <div className="flex items-center gap-1.5">
              <span className={`px-2 py-0.5 rounded-full text-[10px] font-mono font-bold border ${
                posture?.screen_lock_active 
                  ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' 
                  : 'bg-red-500/10 text-red-400 border-red-500/20'
              }`}>
                {posture?.screen_lock_active ? 'ENFORCED' : 'OFF'}
              </span>
              {!posture?.screen_lock_active && (
                <button
                  type="button"
                  onClick={handleRemediateScreenLock}
                  disabled={remediating === 'screen_lock'}
                  className="px-2 py-0.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-[10px] font-bold transition-all cursor-pointer"
                >
                  <IconTool className="w-3 h-3 inline mr-1" />
                  Fix
                </button>
              )}
            </div>
          </div>
          <div className="space-y-1">
            <h4 className="text-xs font-bold text-white uppercase tracking-wider font-mono">
              Screen Lock Timeout
            </h4>
            <p className="text-xs text-slate-300 font-semibold">
              Auto-Lock ≤ 5 Minutes
            </p>
            <p className="text-[11px] text-slate-400 font-medium">
              Prevents unauthorized physical access to unattended workstations.
            </p>
          </div>
        </div>

        {/* 6. SSH Keys Passphrase */}
        <div className="p-4 rounded-2xl bg-slate-900/60 border border-slate-800/90 shadow-sm space-y-3">
          <div className="flex items-center justify-between">
            <span className="p-2 rounded-xl bg-slate-800/80 text-yellow-400 border border-slate-700/60">
              <IconKey className="w-4 h-4" />
            </span>
            <span className={`px-2 py-0.5 rounded-full text-[10px] font-mono font-bold border ${
              posture?.ssh_keys_secure 
                ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' 
                : 'bg-amber-500/10 text-amber-400 border-amber-500/20'
            }`}>
              {posture?.ssh_keys_secure ? 'PROTECTED' : 'UNENCRYPTED'}
            </span>
          </div>
          <div className="space-y-1">
            <h4 className="text-xs font-bold text-white uppercase tracking-wider font-mono">
              SSH Key Protection
            </h4>
            <p className="text-xs text-slate-300 font-semibold">
              Passphrase Protected Keys
            </p>
            <p className="text-[11px] text-slate-400 font-medium">
              Audits local `~/.ssh/` keys for required passphrases.
            </p>
          </div>
        </div>

      </div>
    </div>
  );
}

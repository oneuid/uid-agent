'use client';

import React, { useState, useEffect, useCallback } from 'react';
import {
  IconDeviceUsb,
  IconShieldCheck,
  IconServer,
  IconCpu,
  IconSettings
} from '@tabler/icons-react';
import { Header } from '../components/header';
import { UsbTokensTab } from '../components/tabs/tokens';
import { PostureTab } from '../components/tabs/posture';
import { DaemonBridgeTab } from '../components/tabs/bridge';
import { EnclaveTab } from '../components/tabs/enclave';
import { SettingsTab } from '../components/tabs/settings';
import {
  UserProfile,
  Certificate,
  Posture,
  TpmInfo,
  DaemonStatus,
  SignatureLog,
  invokeCommand
} from '../lib/ipc';
import { useI18n } from '../lib/i18n';
import { toast } from 'sonner';

type TabKey = 'tokens' | 'posture' | 'bridge' | 'enclave' | 'settings';

export default function AgentDashboard() {
  const { t } = useI18n();
  const [activeTab, setActiveTab] = useState<TabKey>('tokens');
  const [user, setUser] = useState<UserProfile | null>(null);
  const [appVersion, setAppVersion] = useState<string>('3.0.16');
  const [certs, setCerts] = useState<Certificate[]>([]);
  const [posture, setPosture] = useState<Posture | null>(null);
  const [tpm, setTpm] = useState<TpmInfo | null>(null);
  const [daemonStatus, setDaemonStatus] = useState<DaemonStatus | null>(null);
  const [signatureLogs, setSignatureLogs] = useState<SignatureLog[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);

  const loadData = useCallback(async () => {
    setIsLoading(true);
    try {
      const [u, v, c, p, tp, ds, logs] = await Promise.all([
        invokeCommand<UserProfile | null>('get_user_profile').catch(() => null),
        invokeCommand<string>('get_app_version').catch(() => '3.0.16'),
        invokeCommand<Certificate[]>('get_certificates').catch(() => []),
        invokeCommand<Posture | null>('get_posture').catch(() => null),
        invokeCommand<TpmInfo | null>('get_tpm_info').catch(() => null),
        invokeCommand<DaemonStatus | null>('check_daemon_status').catch(() => null),
        invokeCommand<SignatureLog[]>('get_signature_history').catch(() => []),
      ]);

      setUser(u);
      if (v) setAppVersion(v);
      setCerts(c || []);
      setPosture(p);
      setTpm(tp);
      setDaemonStatus(ds);
      setSignatureLogs(logs || []);
    } catch (err) {
      console.warn('Failed to load agent telemetry:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    let ignore = false;

    const runInitialLoad = async () => {
      try {
        const [u, v, c, p, tp, ds, logs] = await Promise.all([
          invokeCommand<UserProfile | null>('get_user_profile').catch(() => null),
          invokeCommand<string>('get_app_version').catch(() => '3.0.16'),
          invokeCommand<Certificate[]>('get_certificates').catch(() => []),
          invokeCommand<Posture | null>('get_posture').catch(() => null),
          invokeCommand<TpmInfo | null>('get_tpm_info').catch(() => null),
          invokeCommand<DaemonStatus | null>('check_daemon_status').catch(() => null),
          invokeCommand<SignatureLog[]>('get_signature_history').catch(() => []),
        ]);

        if (!ignore) {
          setUser(u);
          if (v) setAppVersion(v);
          setCerts(c || []);
          setPosture(p);
          setTpm(tp);
          setDaemonStatus(ds);
          setSignatureLogs(logs || []);
        }
      } catch (err) {
        console.warn('Failed to load agent telemetry:', err);
      }
    };

    runInitialLoad();

    const interval = setInterval(async () => {
      try {
        const [c, ds] = await Promise.all([
          invokeCommand<Certificate[]>('get_certificates').catch(() => []),
          invokeCommand<DaemonStatus | null>('check_daemon_status').catch(() => null),
        ]);
        if (!ignore) {
          setCerts(c || []);
          if (ds) setDaemonStatus(ds);
        }
      } catch {}
    }, 10000);

    return () => {
      ignore = true;
      clearInterval(interval);
    };
  }, []);

  const handleLogout = async () => {
    try {
      await invokeCommand('logout_user');
      setUser(null);
      toast.success(t('logoutSuccess', 'Signed out from local session.'));
    } catch (e) {
      toast.error(`${t('logoutFailed', 'Sign out failed')}: ${String(e)}`);
    }
  };

  const navTabs = [
    {
      id: 'tokens' as TabKey,
      label: t('tokensTab', 'USB certificates'),
      icon: IconDeviceUsb,
      badge: certs.length > 0 ? `${certs.length}` : undefined,
    },
    {
      id: 'posture' as TabKey,
      label: t('postureTab', 'Posture check'),
      icon: IconShieldCheck,
    },
    {
      id: 'bridge' as TabKey,
      label: t('bridgeTab', 'Extension bridge'),
      icon: IconServer,
    },
    {
      id: 'enclave' as TabKey,
      label: t('enclaveTab', 'Hardware enclave'),
      icon: IconCpu,
    },
    {
      id: 'settings' as TabKey,
      label: t('settingsTab', 'Settings'),
      icon: IconSettings,
    },
  ];

  const tpmSummary = !tpm
    ? '—'
    : tpm.present
      ? t('active', 'Active')
      : t('notDetected', 'Not detected');

  return (
    <div className="flex flex-col min-h-dvh bg-slate-950 text-slate-100">
      {/* Top Universal Header */}
      <Header
        user={user}
        appVersion={appVersion}
        onRefresh={loadData}
        onLogout={handleLogout}
      />

      {/* Main Layout aligned horizontally (w-full px-5) with Header */}
      <div className="flex-1 w-full px-5 py-5 grid grid-cols-1 md:grid-cols-12 gap-6">
        {/* Navigation Sidebar */}
        <nav className="md:col-span-3 space-y-1.5 font-sans">
          <p className="text-xs font-semibold text-slate-400 px-3 py-1 font-sans">
            {t('navTitle', 'Navigation')}
          </p>
          {navTabs.map((tab) => {
            const isActive = activeTab === tab.id;
            const IconComp = tab.icon;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => setActiveTab(tab.id)}
                className={`w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-xs font-sans font-semibold transition-colors duration-150 text-start cursor-pointer border ${
                  isActive
                    ? 'bg-blue-600/15 text-blue-400 border-blue-500/30 font-bold shadow-2xs'
                    : 'text-slate-300 hover:text-white hover:bg-slate-900/80 border-transparent hover:border-slate-800'
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <IconComp className={`w-4 h-4 shrink-0 ${isActive ? 'text-blue-400' : 'text-slate-400'}`} />
                  <span>{tab.label}</span>
                </div>
                {tab.badge && (
                  <span className={`text-[10px] px-2 py-0.5 rounded-full font-mono font-bold tabular-nums ${
                    isActive ? 'bg-blue-500/20 text-blue-300' : 'bg-emerald-500/20 text-emerald-400'
                  }`}>
                    {tab.badge}
                  </span>
                )}
              </button>
            );
          })}

          {/* Quick Hardware Summary in Sidebar */}
          <div className="mt-6 p-3.5 rounded-xl bg-slate-900/80 border border-slate-800 space-y-2 text-xs font-sans">
            <span className="text-xs font-semibold text-slate-300 block">
              {t('nodeTelemetry', 'Node telemetry')}
            </span>
            <div className="flex items-center justify-between gap-2 text-[11px]">
              <span className="text-slate-400">{t('daemonLabel', 'Daemon')}</span>
              <span className={`font-mono tabular-nums ${daemonStatus?.running ? 'text-emerald-400 font-bold' : 'text-slate-400'}`}>
                {daemonStatus ? `127.0.0.1:${daemonStatus.port}` : '—'}
              </span>
            </div>
            <div className="flex items-center justify-between gap-2 text-[11px]">
              <span className="text-slate-400">{t('tpmLabel', 'TPM 2.0')}</span>
              <span className={tpm?.present ? 'text-blue-400 font-semibold' : 'text-slate-400'}>
                {tpmSummary}
              </span>
            </div>
            <div className="flex items-center justify-between gap-2 text-[11px]">
              <span className="text-slate-400">{t('tokensLabel', 'Tokens')}</span>
              <span className={`font-mono tabular-nums ${certs.length > 0 ? 'text-emerald-400 font-bold' : 'text-slate-400'}`}>
                {certs.length} {t('plugged', 'plugged')}
              </span>
            </div>
          </div>
        </nav>

        {/* Tab Content Display Area */}
        <main className="md:col-span-9 min-w-0">
          {activeTab === 'tokens' && (
            <UsbTokensTab certs={certs} onRefresh={loadData} isLoading={isLoading} />
          )}

          {activeTab === 'posture' && (
            <PostureTab posture={posture} onRefresh={loadData} />
          )}

          {activeTab === 'bridge' && (
            <DaemonBridgeTab status={daemonStatus} logs={signatureLogs} onRefresh={loadData} />
          )}

          {activeTab === 'enclave' && (
            <EnclaveTab tpm={tpm} />
          )}

          {activeTab === 'settings' && (
            <SettingsTab appVersion={appVersion} />
          )}
        </main>
      </div>
    </div>
  );
}

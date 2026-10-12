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
import { UsbTokensTab } from '../components/tabs/usb-tokens-tab';
import { PostureTab } from '../components/tabs/posture-tab';
import { DaemonBridgeTab } from '../components/tabs/daemon-bridge-tab';
import { EnclaveTab } from '../components/tabs/enclave-tab';
import { SettingsTab } from '../components/tabs/settings-tab';
import {
  UserProfile,
  Certificate,
  Posture,
  TpmInfo,
  DaemonStatus,
  SignatureLog,
  invokeCommand
} from '../lib/tauri-ipc';
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

    // Periodic bus polling (detect USB token plug/unplug every 10 seconds)
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
      toast.error('Logout failed: ' + String(e));
    }
  };

  const navTabs = [
    {
      id: 'tokens' as TabKey,
      label: t('tokensTab', 'USB Certificates'),
      icon: IconDeviceUsb,
      badge: certs.length > 0 ? `${certs.length}` : undefined,
    },
    {
      id: 'posture' as TabKey,
      label: t('postureTab', 'Posture Check'),
      icon: IconShieldCheck,
    },
    {
      id: 'bridge' as TabKey,
      label: 'Extension Bridge',
      icon: IconServer,
    },
    {
      id: 'enclave' as TabKey,
      label: 'Hardware Enclave',
      icon: IconCpu,
    },
    {
      id: 'settings' as TabKey,
      label: t('settingsTab', 'Settings'),
      icon: IconSettings,
    },
  ];

  return (
    <div className="flex flex-col min-h-screen bg-slate-950 text-slate-100">
      {/* Top Universal Header */}
      <Header
        user={user}
        appVersion={appVersion}
        onRefresh={loadData}
        onLogout={handleLogout}
      />

      {/* Main Kiosk Layout (Left Navigation + Content Area) */}
      <div className="flex-1 max-w-7xl w-full mx-auto p-4 sm:p-6 grid grid-cols-1 md:grid-cols-12 gap-6">
        
        {/* Navigation Sidebar */}
        <nav className="md:col-span-3 space-y-1.5">
          <p className="text-[11px] font-bold uppercase tracking-wider text-slate-400 px-3 py-1 font-mono">
            Navigation
          </p>
          {navTabs.map((tab) => {
            const isActive = activeTab === tab.id;
            const IconComp = tab.icon;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => setActiveTab(tab.id)}
                className={`w-full flex items-center justify-between px-3.5 py-3 rounded-xl text-xs font-semibold transition-all text-left cursor-pointer ${
                  isActive
                    ? 'bg-blue-600 text-white font-bold shadow-[0_2px_12px_rgba(37,99,235,0.35)]'
                    : 'text-slate-300 hover:text-white hover:bg-slate-900/80 border border-transparent hover:border-slate-800'
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <IconComp className={`w-4 h-4 shrink-0 ${isActive ? 'text-white' : 'text-slate-400'}`} />
                  <span>{tab.label}</span>
                </div>
                {tab.badge && (
                  <span className={`text-[10px] px-2 py-0.5 rounded-full font-mono font-bold ${
                    isActive ? 'bg-white/20 text-white' : 'bg-emerald-500/20 text-emerald-400'
                  }`}>
                    {tab.badge}
                  </span>
                )}
              </button>
            );
          })}

          {/* Quick Hardware Summary in Sidebar */}
          <div className="mt-8 p-3.5 rounded-xl bg-slate-900/60 border border-slate-800/80 space-y-2 text-xs font-mono">
            <span className="text-[10px] uppercase font-bold text-slate-400 tracking-wider block">
              Node Telemetry
            </span>
            <div className="flex justify-between text-[11px]">
              <span className="text-slate-400">Daemon</span>
              <span className="text-emerald-400 font-bold">127.0.0.1:13013</span>
            </div>
            <div className="flex justify-between text-[11px]">
              <span className="text-slate-400">TPM 2.0</span>
              <span className="text-blue-400 font-bold">{tpm?.present ? 'Active' : 'Detected'}</span>
            </div>
            <div className="flex justify-between text-[11px]">
              <span className="text-slate-400">Tokens</span>
              <span className={certs.length > 0 ? 'text-emerald-400 font-bold' : 'text-slate-400'}>
                {certs.length} Plugged
              </span>
            </div>
          </div>
        </nav>

        {/* Tab Content Display Area */}
        <main className="md:col-span-9">
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

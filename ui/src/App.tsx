import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { 
  IconShieldCheck, 
  IconAlertCircle, 
  IconAlertTriangle, 
  IconCpu, 
  IconDeviceUsb, 
  IconInfoCircle,
  IconPin,
  IconX,
  IconSettings,
  IconUser,
  IconLogout,
  IconBox,
  IconFingerprint,
  IconTerminal,
  IconPlayerPlay,
  IconPlayerStop,
  IconPlus,
  IconLoader2,
  IconSearch,
  IconCheck,
  IconTrash,
  IconDownload,
  IconUpload,
  IconHistory
} from '@tabler/icons-react';

import en from '../messages/en.json';
import vi from '../messages/vi.json';
import zh from '../messages/zh.json';
import fr from '../messages/fr.json';
import es from '../messages/es.json';
import ar from '../messages/ar.json';
import ja from '../messages/ja.json';
import ko from '../messages/ko.json';
import th from '../messages/th.json';
import ru from '../messages/ru.json';

const TRANSLATIONS: Record<string, any> = {
  en,
  vi,
  zh,
  fr,
  es,
  ar,
  ja,
  ko,
  th,
  ru
};

const SUPPORTED_LANGUAGES = [
  { code: 'en', name: 'English (US)' },
  { code: 'vi', name: 'Tiếng Việt' },
  { code: 'zh', name: '简体中文' },
  { code: 'fr', name: 'Français' },
  { code: 'es', name: 'Español' },
  { code: 'ar', name: 'العربية (RTL)' },
  { code: 'th', name: 'ไทย' },
  { code: 'ru', name: 'Русский' },
  { code: 'ja', name: '日本語' },
  { code: 'ko', name: '한국어' }
];

const getValidityPercentage = (validFrom?: string, validTo?: string): number => {
  if (!validFrom || !validTo) return 100;
  try {
    const start = new Date(validFrom).getTime();
    const end = new Date(validTo).getTime();
    const now = Date.now();
    if (now < start) return 0;
    if (now > end) return 100;
    const total = end - start;
    const elapsed = now - start;
    return Math.round((elapsed / total) * 100);
  } catch {
    return 100;
  }
};

interface Certificate {
  label?: string;
  id?: string;
  issuer?: string;
  subject?: string;
  valid_from?: string;
  valid_to?: string;
  serial?: string;
}

interface Posture {
  os_family?: string;
  os_release?: string;
  hostname?: string;
  firewall_status?: string;
  disk_encrypted?: boolean;
  secure_boot?: boolean;
  screen_lock_active?: boolean;
  ssh_keys_secure?: boolean;
  vpn_active?: boolean;
}

interface UserProfile {
  token: string;
  name: string;
  email: string;
  avatar?: string;
}

interface SignatureHistoryEntry {
  timestamp: string;
  cert_id: string;
  subject: string;
  hash: string;
  status: string;
  origin: string;
  referer: string;
}

export const AGENT_VERSION = '3.0.15';

export default function App() {
  const [activeTab, setActiveTab] = useState<'dashboard' | 'tokens' | 'apps' | 'approvals' | 'settings'>('dashboard');
  const [userProfile, setUserProfile] = useState<UserProfile | null>(null);
  const [language, setLanguage] = useState<string>(() => {
    return localStorage.getItem('uid-agent-lang') || 'en';
  });
  const t = (key: keyof typeof en): string => {
    const dict = TRANSLATIONS[language] || TRANSLATIONS.en;
    return dict[key] || TRANSLATIONS.en[key] || String(key);
  };
  const [posture, setPosture] = useState<Posture | null>(null);
  const [certs, setCerts] = useState<Certificate[]>([]);
  const [sigHistory, setSigHistory] = useState<SignatureHistoryEntry[]>([]);
  const loading = false;
  const [log, setLog] = useState<string>('');
  const [checkingUpdate, setCheckingUpdate] = useState<boolean>(false);
  const [verifyingConnection, setVerifyingConnection] = useState<boolean>(false);
  const [appVersion, setAppVersion] = useState<string>(AGENT_VERSION);
  const renderVersionInfo = () => {
    let v = t('versionInfo');
    v = v.replace('3.0.0', appVersion);
    if (posture?.os_family) {
      v = v.replace('(Linux)', `(${posture.os_family})`);
    }
    return v;
  };
  const [customExtId, setCustomExtId] = useState<string>('');
  const [showExtSettings, setShowExtSettings] = useState<boolean>(false);
  const [installingExt, setInstallingExt] = useState<boolean>(false);
  const [showLUKSWizard, setShowLUKSWizard] = useState<boolean>(false);
  const [showFirewallWizard, setShowFirewallWizard] = useState<boolean>(false);
  const [showSecureBootWizard, setShowSecureBootWizard] = useState<boolean>(false);
  const [showSSHKeysWizard, setShowSSHKeysWizard] = useState<boolean>(false);
  const [showVPNWizard, setShowVPNWizard] = useState<boolean>(false);
  const [remediatingMap, setRemediatingMap] = useState<Record<string, boolean>>({});
  const [isSyncing, setIsSyncing] = useState<Record<string, boolean>>({});
  const [syncStatus, setSyncStatus] = useState<Record<string, string>>(() => {
    try {
      const saved = localStorage.getItem('uid-sandbox-sync-status');
      return saved ? JSON.parse(saved) : {};
    } catch {
      return {};
    }
  });

  // View 1 (Compliance log) state
  interface ComplianceEvent {
    timestamp: string;
    control: string;
    status: string;
    details: string;
  }
  const [complianceEvents, setComplianceEvents] = useState<ComplianceEvent[]>(() => {
    try {
      const saved = localStorage.getItem('uid-compliance-events');
      return saved ? JSON.parse(saved) : [];
    } catch {
      return [];
    }
  });

  // View 2 (Sandbox storage size) state
  const [appStorageSizes, setAppStorageSizes] = useState<Record<string, string>>({});

  // View 3 (Daemon & IPC status) state
  const [daemonActive, setDaemonActive] = useState<'loading' | 'active' | 'inactive'>('loading');
  const [ipcActive, setIpcActive] = useState<'loading' | 'active' | 'inactive'>('loading');

  // View 4 (TPM info) state
  interface TpmInfo {
    present: boolean;
    version: string;
    vendor: string;
    type: string;
    attestation_support: string;
  }
  const [tpmInfo, setTpmInfo] = useState<TpmInfo | null>(null);

  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' | 'info' } | null>(null);
  
  const showToast = (message: string, type: 'success' | 'error' | 'info' = 'info') => {
    setToast({ message, type });
    setTimeout(() => {
      setToast(current => current?.message === message ? null : current);
    }, 5000);
  };
  // App Sandbox State (Isolated Web Containers)
  interface SandboxApp {
    id: string;
    name: string;
    description: string;
    status: 'running' | 'stopped' | 'not_configured';
    url: string;
    logs: string[];
    isInstalling?: boolean;
    installProgress?: number;
  }

  const [sandboxApps, setSandboxApps] = useState<SandboxApp[]>([
    {
      id: 'zalo',
      name: 'Zalo Messenger',
      description: 'Run Zalo Web in a secure, isolated desktop container. Preserves all chat history, local cache, and user profile data permanently.',
      status: 'stopped',
      url: 'https://chat.zalo.me/',
      logs: ['[sandbox] Container partition initialized at ~/.local/share/uid/apps/zalo', '[sandbox] Isolated cookies/localStorage storage active.']
    },
    {
      id: 'misa',
      name: 'MISA Accounting',
      description: 'Isolated accounting workspace. Prevents invoice session sniffing and keeps ledger tokens secure.',
      status: 'not_configured',
      url: 'https://amisapp.misa.vn/',
      logs: []
    },
    {
      id: 'pdf-signer',
      name: 'Acrobat PDF Signer',
      description: 'Secure browser workspace for document signing and verification.',
      status: 'stopped',
      url: 'https://www.adobe.com/acrobat/online/sign-pdf.html',
      logs: ['[sandbox] Secure signature tunnel initialized.']
    }
  ]);
  const [selectedApp, setSelectedApp] = useState<SandboxApp | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [showAddAppModal, setShowAddAppModal] = useState<boolean>(false);
  const [newAppName, setNewAppName] = useState<string>('');
  const [newAppUrl, setNewAppUrl] = useState<string>('');
  const [showDevOptions, setShowDevOptions] = useState<boolean>(false);
  const [isCardHovered, setIsCardHovered] = useState<boolean>(false);

  // Pending Approvals State
  interface ApprovalRequest {
    id: string;
    origin: string;
    type: 'login' | 'sign_document';
    title: string;
    description: string;
    payload: string;
    timestamp: string;
    isSigning?: boolean;
    isSuccess?: boolean;
  }

  const [pendingApprovals, setPendingApprovals] = useState<ApprovalRequest[]>([
    {
      id: 'req_1',
      origin: 'http://localhost:3000',
      type: 'login',
      title: 'Identity Login Approval',
      description: 'Authenticate connection request from Trip.Express Workspace.',
      payload: 'challenge_token_9x12a87c12f0088b72e12e',
      timestamp: new Date().toLocaleTimeString()
    },
    {
      id: 'req_2',
      origin: 'http://localhost:3000',
      type: 'sign_document',
      title: 'Contract PDF Sign Request',
      description: 'Cryptographically sign "trip_express_contract_2026.pdf" with local token.',
      payload: '8f0a2c918bb1389cb1f1c991e12db59846bf493aa9a8b1399f92e92c2876611e',
      timestamp: new Date(Date.now() - 60000).toLocaleTimeString()
    }
  ]);
  const handleLaunchApp = (appId: string) => {
    setSandboxApps(prev => prev.map(app => {
      if (app.id === appId) {
        const newLogs = [
          ...app.logs,
          `[sandbox] [${new Date().toLocaleTimeString()}] Spawning isolated WebView container...`,
          `[sandbox] [${new Date().toLocaleTimeString()}] Local user profile directory: ~/.local/share/uid/apps/${appId}`,
          `[sandbox] [${new Date().toLocaleTimeString()}] Target URL: ${app.url}`,
          `[sandbox] [${new Date().toLocaleTimeString()}] WebView process launched successfully.`
        ];
        const updated = { ...app, status: 'running' as const, logs: newLogs };
        if (selectedApp?.id === appId) {
          setSelectedApp(updated);
        }
        
        // Invoke native Tauri command to launch WebView with persistent local data
        invoke('launch_sandbox_app', { appId, appName: app.name, url: app.url }).catch(err => {
          console.error('Failed to launch sandbox app window:', err);
        });
        
        return updated;
      }
      return app;
    }));
  };

  const handleStopApp = (appId: string) => {
    setSandboxApps(prev => prev.map(app => {
      if (app.id === appId) {
        const newLogs = [
          ...app.logs,
          `[sandbox] [${new Date().toLocaleTimeString()}] Terminating isolated WebView...`,
          `[sandbox] [${new Date().toLocaleTimeString()}] Isolated session saved successfully.`
        ];
        const updated = { ...app, status: 'stopped' as const, logs: newLogs };
        if (selectedApp?.id === appId) {
          setSelectedApp(updated);
        }
        return updated;
      }
      return app;
    }));
  };

  const handleInstallApp = (appId: string) => {
    setSandboxApps(prev => prev.map(app => {
      if (app.id === appId) {
        return { ...app, isInstalling: true, installProgress: 0 };
      }
      return app;
    }));

    let progress = 0;
    const interval = setInterval(() => {
      progress += 10;
      setSandboxApps(prev => prev.map(app => {
        if (app.id === appId) {
          const finished = progress >= 100;
          if (finished) {
            clearInterval(interval);
            const newLogs = [
              `[sandbox] [${new Date().toLocaleTimeString()}] Creating isolated storage directories...`,
              `[sandbox] [${new Date().toLocaleTimeString()}] Persisting custom workspace profiles...`,
              `[sandbox] [${new Date().toLocaleTimeString()}] Desktop Launcher created successfully.`
            ];
            const updated = { 
              ...app, 
              status: 'stopped' as const, 
              isInstalling: false, 
              installProgress: 100,
              logs: newLogs
            };
            if (selectedApp?.id === appId) {
              setSelectedApp(updated);
            }
            return updated;
          }
          return { ...app, installProgress: progress };
        }
        return app;
      }));
    }, 200);
  };

  const handleAddCustomApp = () => {
    if (!newAppName.trim() || !newAppUrl.trim()) return;
    const id = newAppName.toLowerCase().replace(/\s+/g, '-');
    const newApp: SandboxApp = {
      id,
      name: newAppName,
      description: `Isolated workspace for ${newAppName}.`,
      status: 'not_configured',
      url: newAppUrl,
      logs: []
    };
    setSandboxApps(prev => [...prev, newApp]);
    setShowAddAppModal(false);
    setNewAppName('');
    setNewAppUrl('');
    setTimeout(() => {
      handleInstallApp(id);
    }, 500);
  };

  const handleSyncProfile = async (appId: string, url: string, direction: 'import' | 'export' | 'restore') => {
    setIsSyncing(prev => ({ ...prev, [appId]: true }));
    try {
      const response = await invoke<string>('sync_sandbox_profile', {
        appId,
        targetUrl: url,
        direction
      });
      
      const newLogs = [
        `[sync] [${new Date().toLocaleTimeString()}] Starting synchronization (${direction})...`,
        `[sync] [${new Date().toLocaleTimeString()}] ${response}`
      ];
      
      // Update app logs
      setSandboxApps(prev => prev.map(app => {
        if (app.id === appId) {
          const updatedLogs = [...app.logs, ...newLogs];
          const updated = { ...app, logs: updatedLogs };
          if (selectedApp?.id === appId) {
            setSelectedApp(updated);
          }
          return updated;
        }
        return app;
      }));

      // Update sync status timestamp
      const nowStr = new Date().toLocaleString();
      setSyncStatus(prev => {
        const next = { ...prev, [appId]: nowStr };
        localStorage.setItem('uid-sandbox-sync-status', JSON.stringify(next));
        return next;
      });

      let toastMsg = '';
      if (direction === 'import') {
        toastMsg = t('syncSuccessImport');
      } else if (direction === 'export') {
        toastMsg = t('syncSuccessExport');
      } else {
        toastMsg = t('syncSuccessRestore');
      }
      showToast(toastMsg, 'success');
    } catch (err) {
      const newLogs = [
        `[sync] [${new Date().toLocaleTimeString()}] Error: ${err}`
      ];
      setSandboxApps(prev => prev.map(app => {
        if (app.id === appId) {
          const updatedLogs = [...app.logs, ...newLogs];
          const updated = { ...app, logs: updatedLogs };
          if (selectedApp?.id === appId) {
            setSelectedApp(updated);
          }
          return updated;
        }
        return app;
      }));
      showToast(t('syncFailed').replace('{error}', String(err)), 'error');
    } finally {
      setIsSyncing(prev => ({ ...prev, [appId]: false }));
    }
  };

  const handleApproveRequest = (reqId: string) => {
    setPendingApprovals(prev => prev.map(req => {
      if (req.id === reqId) {
        return { ...req, isSigning: true };
      }
      return req;
    }));

    setTimeout(async () => {
      const req = pendingApprovals.find(r => r.id === reqId);
      if (req && req.type === 'sign_document') {
        const historyEntry: SignatureHistoryEntry = {
          timestamp: new Date().toISOString(),
          cert_id: 'HSM-ATTEST-01',
          subject: 'CN=Admin UID, O=UID.one, C=VN',
          hash: req.payload,
          status: 'Success',
          origin: req.origin,
          referer: req.origin
        };
        setSigHistory(prev => [historyEntry, ...prev]);
      }

      setPendingApprovals(prev => prev.map(r => {
        if (r.id === reqId) {
          return { ...r, isSigning: false, isSuccess: true };
        }
        return r;
      }));

      setTimeout(() => {
        setPendingApprovals(prev => prev.filter(r => r.id !== reqId));
      }, 1000);
    }, 1500);
  };

  const handleRejectRequest = (reqId: string) => {
    setPendingApprovals(prev => prev.filter(req => req.id !== reqId));
  };

  // Load initial info once on mount
  useEffect(() => {
    const loadInitialData = async () => {
      try {
        const postureData = await invoke<Posture>('get_posture');
        setPosture(postureData);

        const profile = await invoke<UserProfile | null>('get_user_profile');
        setUserProfile(profile);

        try {
          const version = await invoke<string>('get_app_version');
          setAppVersion(version);
        } catch (err) {
          console.warn('Failed to get app version:', err);
        }

        try {
          const certsData = await invoke<Certificate[]>('get_certificates');
          setCerts(certsData);
        } catch (err) {
          console.warn('Failed to load certificates on mount:', err);
        }

        try {
          const tpm = await invoke<TpmInfo>('get_tpm_info');
          setTpmInfo(tpm);
        } catch (err) {
          console.warn('Failed to get TPM info:', err);
          setTpmInfo({
            present: true,
            version: '2.0',
            vendor: 'INTC (Intel)',
            type: 'fTPM (Intel PTT)',
            attestation_support: 'Intel Attestation Service (IAS)'
          });
        }
      } catch (e) {
        console.error('Failed to load initial data:', e);
        setLog(`Failed to load initial data: ${e}`);
      }
    };
    loadInitialData();
  }, []);

  // Poll daemon and IPC status
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const daemon = await invoke<boolean>('check_daemon_status');
        setDaemonActive(daemon ? 'active' : 'inactive');
      } catch {
        setDaemonActive('inactive');
      }
      setIpcActive('active');
    };
    checkStatus();
    const interval = setInterval(checkStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  // Monitor posture changes for compliance event log
  const [prevPosture, setPrevPosture] = useState<Posture | null>(null);
  useEffect(() => {
    if (!posture) return;
    if (!prevPosture) {
      setPrevPosture(posture);
      return;
    }

    const newEvents: ComplianceEvent[] = [];
    const timestamp = new Date().toISOString();

    const checkChange = (
      controlName: string,
      oldVal: any,
      newVal: any,
      descEnabled: string,
      descDisabled: string
    ) => {
      if (oldVal !== undefined && oldVal !== newVal) {
        newEvents.push({
          timestamp,
          control: controlName,
          status: newVal === 'active' || newVal === true ? 'active' : 'inactive',
          details: newVal === 'active' || newVal === true ? descEnabled : descDisabled,
        });
      }
    };

    checkChange(
      'firewall',
      prevPosture.firewall_status,
      posture.firewall_status,
      'Firewall has been enabled and is actively blocking unauthorized traffic.',
      'Firewall was disabled, exposing system ports to external network requests.'
    );
    checkChange(
      'diskEncryption',
      prevPosture.disk_encrypted,
      posture.disk_encrypted,
      'LUKS/Disk Encryption detected. Secure storage partition is locked and protected.',
      'LUKS/Disk Encryption is not detected. Data at rest is vulnerable to physical theft.'
    );
    checkChange(
      'secureBoot',
      prevPosture.secure_boot,
      posture.secure_boot,
      'Secure Boot is active, preventing unsigned kernels and firmware rootkits.',
      'Secure Boot is inactive, allowing unsigned or boot-level rootkits to execute.'
    );
    checkChange(
      'screenLock',
      prevPosture.screen_lock_active,
      posture.screen_lock_active,
      'Screen auto-lock timeout is active and secured.',
      'Screen auto-lock timeout is inactive, allowing physical workstation tampering.'
    );
    checkChange(
      'sshKeys',
      prevPosture.ssh_keys_secure,
      posture.ssh_keys_secure,
      'All local SSH private keys are secured with strong passphrases.',
      'Unsecured SSH private key found without a passphrase.'
    );
    checkChange(
      'vpn',
      prevPosture.vpn_active,
      posture.vpn_active,
      'VPN connection established. Remote network traffic is secured.',
      'VPN disconnected. Remote network access is exposed to public routing.'
    );

    if (newEvents.length > 0) {
      setComplianceEvents(prev => {
        const updated = [...newEvents, ...prev].slice(0, 100);
        localStorage.setItem('uid-compliance-events', JSON.stringify(updated));
        return updated;
      });
    }
    setPrevPosture(posture);
  }, [posture, prevPosture]);

  // Export audit report
  const handleExportAuditReport = () => {
    if (!posture) return;
    const report = {
      header: t('auditReportHeader'),
      timestamp: new Date().toISOString(),
      agent_version: appVersion,
      device_info: {
        hostname: posture.hostname || 'Unknown',
        os_family: posture.os_family || 'Linux',
        os_release: posture.os_release || 'Unknown',
      },
      security_posture_compliance: {
        firewall: posture.firewall_status === 'active' ? 'COMPLIANT' : 'NON-COMPLIANT',
        disk_encryption: posture.disk_encrypted ? 'COMPLIANT' : 'NON-COMPLIANT',
        secure_boot: posture.secure_boot ? 'COMPLIANT' : 'NON-COMPLIANT',
        screen_lock: posture.screen_lock_active ? 'COMPLIANT' : 'NON-COMPLIANT',
        ssh_keys: posture.ssh_keys_secure ? 'COMPLIANT' : 'NON-COMPLIANT',
        vpn: posture.vpn_active ? 'SECURED' : 'UNSECURED',
      },
      audit_events_history: complianceEvents,
    };

    const blob = new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `uid-security-audit-${posture.hostname || 'device'}-${new Date().toISOString().slice(0, 10)}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    showToast('Successfully exported compliance audit report!', 'success');
  };

  // Load storage size for selected app
  useEffect(() => {
    if (activeTab !== 'apps' || !selectedApp) return;
    const fetchStorageSize = async () => {
      try {
        const sizeBytes = await invoke<number>('get_sandbox_storage_size', { appId: selectedApp.id });
        let sizeStr = '';
        if (sizeBytes === 0) {
          sizeStr = '0 B';
        } else if (sizeBytes < 1024) {
          sizeStr = `${sizeBytes} B`;
        } else if (sizeBytes < 1024 * 1024) {
          sizeStr = `${(sizeBytes / 1024).toFixed(1)} KB`;
        } else {
          sizeStr = `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`;
        }
        setAppStorageSizes(prev => ({ ...prev, [selectedApp.id]: sizeStr }));
      } catch (err) {
        console.warn('Failed to fetch storage size:', err);
      }
    };
    fetchStorageSize();
  }, [selectedApp, activeTab]);

  // Poll user profile status periodically to keep sync state updated dynamically
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const profile = await invoke<UserProfile | null>('get_user_profile');
        setUserProfile(prev => {
          // Compare objects to avoid trigger state updates/re-renders if same
          if (JSON.stringify(prev) !== JSON.stringify(profile)) {
            if (profile && !prev) {
              invoke('close_login_window').catch(() => {});
            }
            return profile;
          }
          return prev;
        });
      } catch (err) {
        console.warn('Failed to poll user profile:', err);
      }
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  // Poll USB certificates only when tokens tab is active
  useEffect(() => {
    if (activeTab !== 'tokens') {
      return;
    }

    const pollData = async () => {
      try {
        const certsData = await invoke<Certificate[]>('get_certificates');
        setCerts(certsData);
      } catch (e) {
        console.error('Error polling certificates:', e);
      }
      try {
        const historyData = await invoke<SignatureHistoryEntry[]>('get_signature_history');
        setSigHistory(historyData);
      } catch (e) {
        console.error('Error fetching signature history:', e);
      }
    };

    pollData();
    const interval = setInterval(pollData, 5000); // 5-second poll interval
    return () => clearInterval(interval);
  }, [activeTab]);

  const refreshAllData = async () => {
    try {
      const postureData = await invoke<Posture>('get_posture');
      setPosture(postureData);

      const profile = await invoke<UserProfile | null>('get_user_profile');
      setUserProfile(profile);
    } catch (e) {
      console.error('Error refreshing data:', e);
    }
  };

  const handleCheckUpdate = async () => {
    setCheckingUpdate(true);
    setLog('Checking for software updates...');
    try {
      const res = await invoke<string>('check_for_updates');
      setLog(`${t('updateSuccess').replace('3.0.0', appVersion)} (Status: ${res})`);
      showToast(res, 'info');
    } catch (e: any) {
      setLog(`Update check failed: ${e}`);
      showToast(`Update check failed: ${e}`, 'error');
    } finally {
      setCheckingUpdate(false);
    }
  };

  const handleVerifyConnection = async () => {
    setVerifyingConnection(true);
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 6000);
      await fetch('https://api.uid.one/', {
        method: 'GET',
        mode: 'no-cors',
        signal: controller.signal
      });
      clearTimeout(timeoutId);
      showToast(t('verifySuccess'), 'success');
    } catch (e: any) {
      console.warn('Connection check failed:', e);
      showToast(t('verifyFailed'), 'error');
    } finally {
      setVerifyingConnection(false);
    }
  };

  const handleRemediateFirewall = async () => {
    setRemediatingMap(prev => ({ ...prev, firewall: true }));
    try {
      await invoke('remediate_firewall');
      setLog(t('remediationSuccess'));
      await refreshAllData();
    } catch (e: any) {
      console.error(e);
      setLog(t('remediationFailed').replace('{error}', e.toString()));
      setShowFirewallWizard(true);
    } finally {
      setRemediatingMap(prev => ({ ...prev, firewall: false }));
    }
  };

  const handleRemediateScreenLock = async () => {
    setRemediatingMap(prev => ({ ...prev, screenLock: true }));
    try {
      await invoke('remediate_screen_lock');
      setLog(t('remediationSuccess'));
      await refreshAllData();
    } catch (e: any) {
      console.error(e);
      setLog(t('remediationFailed').replace('{error}', e.toString()));
      showToast(t('remediationFailed').replace('{error}', e.toString()), 'error');
    } finally {
      setRemediatingMap(prev => ({ ...prev, screenLock: false }));
    }
  };



  const handleLogout = async () => {
    try {
      await invoke('logout_user');
      setUserProfile(null);
      setLog("Logged out successfully.");
    } catch (e: any) {
      setLog(`Failed to logout: ${e}`);
    }
  };

  const handleConnectAccount = async () => {
    try {
      await invoke('open_login_window');
      setLog("Opened secure login window. Once logged in, your account will automatically sync here.");
    } catch (e: any) {
      setLog(`Failed to open login window, falling back to browser: ${e}`);
      try {
        await invoke('open_browser_url', { url: 'https://uid.one' });
      } catch (err) {
        console.error('Browser fallback failed:', err);
      }
    }
  };



  const handlePinApp = async (appId: string) => {
    try {
      const res = await invoke<string>('pin_to_dock', { appId });
      setLog(res);
      showToast(t('pinSuccessMsg'), 'success');
    } catch (e: any) {
      setLog(`Error pinning app to GNOME Dock: ${e}`);
      showToast(`${t('pinFailedMsg')}: ${e}`, 'error');
    }
  };

  const handleInstallExtension = async () => {
    setInstallingExt(true);
    setLog('Initializing UID Link Browser Extension installation...');
    try {
      const res = await invoke<string>('install_browser_extension', { 
        customChromeId: customExtId || null 
      });
      
      const match = res.match(/unpacked successfully at:\s*(.+)/i);
      const extPath = match ? match[1].trim() : '/home/s/.config/uid/extensions/coobgfinhhjocjlhjiaegcfolhdgiinb';
      
      try {
        await navigator.clipboard.writeText(extPath);
      } catch (clipboardErr) {
        console.warn('Clipboard write failed:', clipboardErr);
      }

      setLog(`${t('extensionInstallSuccess')}\n\nDetails:\n${res}\n\n` + 
             `[ACTION REQUIRED] Since the extension is not yet published to the Chrome Web Store, you must load it manually:\n` +
             `1. Open chrome://extensions/ in your browser.\n` +
             `2. Turn ON "Developer mode" in the top-right.\n` +
             `3. Click "Load unpacked" in the top-left.\n` +
             `4. Paste the extension path (already copied to your clipboard):\n   ${extPath}\n\n` +
             `Once loaded, the browser extension will connect automatically.`);
      showToast("Extension unpacked. Path copied to clipboard!", "success");
    } catch (e: any) {
      setLog(`${t('extensionInstallFailed').replace('{error}', e)}`);
      showToast("Extension installation failed", "error");
    } finally {
      setInstallingExt(false);
    }
  };

  return (
    <div className="app-container" dir={language === 'ar' ? 'rtl' : 'ltr'}>
      {toast && (
        <div style={{
          position: 'fixed',
          bottom: '24px',
          right: '24px',
          background: toast.type === 'error' ? 'rgba(239, 68, 68, 0.15)' : 'rgba(16, 185, 129, 0.15)',
          border: toast.type === 'error' ? '1px solid rgba(239, 68, 68, 0.3)' : '1px solid rgba(16, 185, 129, 0.3)',
          color: toast.type === 'error' ? 'var(--danger-color)' : 'var(--success-color)',
          borderRadius: '12px',
          padding: '14px 20px',
          boxShadow: '0 8px 32px rgba(0,0,0,0.5)',
          zIndex: 9999,
          backdropFilter: 'blur(20px)',
          display: 'flex',
          alignItems: 'center',
          gap: '10px',
          animation: 'slideIn 0.3s ease-out'
        }}>
          {toast.type === 'error' ? <IconAlertTriangle size={18} /> : <IconShieldCheck size={18} />}
          <span style={{ fontSize: '13px', fontWeight: '500' }}>{toast.message}</span>
          <button 
            onClick={() => setToast(null)}
            style={{ 
              background: 'none', 
              border: 'none', 
              color: 'inherit', 
              cursor: 'pointer', 
              opacity: 0.7, 
              marginLeft: '10px',
              padding: 0,
              display: 'flex',
              alignItems: 'center'
            }}
          >
            <IconX size={14} />
          </button>
        </div>
      )}
      {/* Header */}
      <header className="app-header">
        <div className="header-logo">
          <div className="logo-uid-one">
            <svg viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg" width="40" height="40" style={{ borderRadius: '10px' }}>
              <defs>
                <linearGradient id="uidGradient" x1="0" y1="0" x2="1" y2="1">
                  <stop offset="0%" stop-color="#2563FF"/>
                  <stop offset="100%" stop-color="#7C3AED"/>
                </linearGradient>
              </defs>
              <rect width="200" height="200" rx="46" fill="#081120"/>
              <g transform="translate(35, 10)">
                <text x="0" y="140" font-family="system-ui, -apple-system, sans-serif" font-size="130" font-weight="800" fill="#FFFFFF">U</text>
                <circle cx="105" cy="124" r="16" fill="url(#uidGradient)"/>
              </g>
            </svg>
          </div>
          <div>
            <h1>{t('agentTitle')}</h1>
            <p>{t('agentSubtitle')}</p>
          </div>
        </div>
        <div className="header-status">
          <span className="status-badge online">
            <span className="pulse-dot"></span> {t('active')}
          </span>
        </div>
      </header>

      {/* Main Area */}
      <div className="app-main">
        {/* Sidebar */}
        <aside className="app-sidebar">
          <div className="sidebar-top-wrapper">
            {/* User Profile Card */}
            <div className="sidebar-profile">
              {userProfile ? (
                <div className="profile-card">
                  <div className="profile-avatar">
                    {userProfile.avatar ? (
                      <img src={userProfile.avatar} alt={userProfile.name} className="avatar-img" />
                    ) : (
                      <div className="avatar-placeholder">
                        {/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(userProfile.name.trim()) ? (
                          <IconShieldCheck size={16} className="gold-icon" />
                        ) : (
                          userProfile.name.charAt(0).toUpperCase()
                        )}
                      </div>
                    )}
                  </div>
                  <div className="profile-info">
                    <span className="profile-name" title={userProfile.name}>
                      {/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(userProfile.name.trim()) 
                        ? t('securedDevice') 
                        : userProfile.name}
                    </span>
                    <span className="profile-email" title={userProfile.email}>
                      {/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(userProfile.email.trim())
                        ? `${userProfile.email.slice(0, 8)}...${userProfile.email.slice(-8)}`
                        : userProfile.email}
                    </span>
                  </div>
                  <button className="profile-logout-btn" onClick={handleLogout} title={t('userLogout')}>
                    <IconLogout size={16} />
                  </button>
                </div>
              ) : (
                <div className="profile-card offline">
                  <div className="profile-avatar placeholder">
                    <IconUser size={18} className="gold-icon" />
                  </div>
                  <div className="profile-info">
                    <span className="profile-name">{t('userNotSynced')}</span>
                    <span className="profile-email">{t('userSyncDesc')}</span>
                  </div>
                  <button className="profile-login-btn" onClick={handleConnectAccount} title={t('userConnectAccount')}>
                    <IconShieldCheck size={16} className="gold-icon" />
                  </button>
                </div>
              )}
            </div>

            <nav className="sidebar-nav">
              <button 
                className={`nav-btn ${activeTab === 'dashboard' ? 'active' : ''}`}
                onClick={() => setActiveTab('dashboard')}
              >
                <IconCpu size={18} />
                <span>{t('postureTab')}</span>
              </button>
              <button 
                className={`nav-btn ${activeTab === 'tokens' ? 'active' : ''}`}
                onClick={() => setActiveTab('tokens')}
              >
                <IconDeviceUsb size={18} />
                <span>{t('tokensTab')}</span>
              </button>

              <button 
                className={`nav-btn ${activeTab === 'apps' ? 'active' : ''}`}
                onClick={() => setActiveTab('apps')}
              >
                <IconBox size={18} />
                <span>{t('appsTab')}</span>
              </button>

              <button 
                className={`nav-btn ${activeTab === 'approvals' ? 'active' : ''}`}
                onClick={() => setActiveTab('approvals')}
                style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', width: '100%' }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '14px' }}>
                  <IconFingerprint size={18} />
                  <span>{t('approvalsTab')}</span>
                </div>
                {pendingApprovals.length > 0 && (
                  <span style={{ 
                    background: 'var(--danger-color)', 
                    color: 'white', 
                    borderRadius: '50%', 
                    padding: '2px 6px', 
                    fontSize: '10px', 
                    fontWeight: 'bold',
                    lineHeight: 1
                  }}>
                    {pendingApprovals.length}
                  </span>
                )}
              </button>

              <button 
                className={`nav-btn ${activeTab === 'settings' ? 'active' : ''}`}
                onClick={() => setActiveTab('settings')}
              >
                <IconSettings size={18} />
                <span>{t('settingsTab')}</span>
              </button>
            </nav>
          </div>

          <div className="sidebar-footer">
            <div className="agent-info">
              <IconInfoCircle size={14} />
              <span>{renderVersionInfo()}</span>
            </div>
            <button 
              className="btn btn-secondary pin-sidebar-btn" 
              onClick={() => handlePinApp('agent')}
              style={{ marginTop: '8px', width: '100%', fontSize: '11px', padding: '6px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '4px' }}
            >
              <IconPin size={12} className="gold-icon" />
              <span>{t('pinAgentBtn')}</span>
            </button>
          </div>
        </aside>

        {/* Content Panel */}
        <main className="app-content">
          {activeTab === 'dashboard' && (
            <div className="tab-pane">
              <h2 className="section-title">{t('postureTitle')}</h2>
              <p className="section-subtitle">{t('postureSubtitle')}</p>

              {/* Security Status Summary Guard */}
              {(() => {
                const totalChecks = 6;
                const compliantCount = (posture?.disk_encrypted ? 1 : 0) +
                                       (posture?.firewall_status === 'active' ? 1 : 0) +
                                       (posture?.secure_boot ? 1 : 0) +
                                       (posture?.screen_lock_active ? 1 : 0) +
                                       (posture?.ssh_keys_secure ? 1 : 0) +
                                       (posture?.vpn_active ? 1 : 0);
                const isAllCompliant = compliantCount === totalChecks;
                const activeToken = certs.length > 0 ? certs[0] : null;

                return (
                  <div style={{
                    background: isAllCompliant ? 'rgba(16, 185, 129, 0.08)' : 'rgba(245, 158, 11, 0.08)',
                    border: `1px solid ${isAllCompliant ? 'rgba(16, 185, 129, 0.25)' : 'rgba(245, 158, 11, 0.25)'}`,
                    borderRadius: '16px',
                    padding: '20px 24px',
                    margin: '24px 0 28px 0',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    gap: '24px',
                    boxShadow: '0 4px 20px rgba(0, 0, 0, 0.15)',
                    backdropFilter: 'blur(20px)',
                    transition: 'all 0.3s ease'
                  }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '20px', flex: 1 }}>
                      <div style={{
                        width: '56px',
                        height: '56px',
                        borderRadius: '50%',
                        background: isAllCompliant ? 'rgba(16, 185, 129, 0.15)' : 'rgba(245, 158, 11, 0.15)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: isAllCompliant ? 'var(--success-color)' : 'var(--warning-color)',
                        boxShadow: `0 0 20px ${isAllCompliant ? 'rgba(16, 185, 129, 0.2)' : 'rgba(245, 158, 11, 0.2)'}`,
                        flexShrink: 0
                      }}>
                        {isAllCompliant ? <IconShieldCheck size={32} /> : <IconAlertTriangle size={32} />}
                      </div>
                      <div>
                        <h3 style={{ margin: 0, fontSize: '16px', fontWeight: '700', color: 'white' }}>
                          {isAllCompliant ? t('dashboardStatusSecure') : t('dashboardStatusRisk')}
                        </h3>
                        <p style={{ margin: '4px 0 8px 0', fontSize: '13px', color: 'var(--text-secondary)' }}>
                          {isAllCompliant 
                            ? t('dashboardStatusDescAll').replace('{count}', String(totalChecks))
                            : t('dashboardStatusDescSome').replace('{compliant}', String(compliantCount)).replace('{total}', String(totalChecks))
                          }
                        </p>
                        
                        {/* Active Hardware State */}
                        <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '12px', color: 'var(--text-muted)' }}>
                          <IconDeviceUsb size={14} className={activeToken ? 'gold-icon' : 'gray-icon'} />
                          <span>
                            {activeToken 
                              ? t('activeUsbToken').replace('{name}', activeToken.issuer || activeToken.subject || 'PKCS#11 Token') 
                              : t('noUsbToken')
                            }
                          </span>
                        </div>
                      </div>
                    </div>
                    
                    <div style={{ display: 'flex', gap: '12px', flexShrink: 0 }}>
                      <button 
                        onClick={handleVerifyConnection}
                        disabled={verifyingConnection}
                        className="btn btn-secondary"
                        style={{ fontSize: '12px', padding: '10px 18px', height: '40px', display: 'flex', alignItems: 'center', gap: '6px' }}
                      >
                        {verifyingConnection ? (
                          <>
                            <IconLoader2 size={14} className="animate-spin" />
                            <span>{t('verifyingConnection')}</span>
                          </>
                        ) : (
                          <span>{t('btnVerifyConnection')}</span>
                        )}
                      </button>
                      
                      <button 
                        onClick={handleCheckUpdate}
                        disabled={checkingUpdate}
                        className="btn btn-secondary"
                        style={{ fontSize: '12px', padding: '10px 18px', height: '40px', display: 'flex', alignItems: 'center', gap: '6px' }}
                      >
                        {checkingUpdate ? (
                          <>
                            <IconLoader2 size={14} className="animate-spin" />
                            <span>{t('btnUpdating')}</span>
                          </>
                        ) : (
                          <span>{t('btnCheckUpdates')}</span>
                        )}
                      </button>
                    </div>
                  </div>
                );
              })()}

              <h4 className="posture-group-title" style={{ margin: '32px 0 16px 0', fontSize: '13px', fontWeight: '700', color: 'var(--accent-gold)', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                {t('basicChecksGroup')}
              </h4>
              <div className="posture-grid">
                <div className="posture-card">
                  <div className="card-header">
                    <h3>{t('osHeader')}</h3>
                    <IconInfoCircle size={18} className="gray-icon" />
                  </div>
                  <div className="card-body">
                    <p className="value">{posture?.os_family || 'Linux'} ({posture?.os_release || 'Unknown'})</p>
                    <p className="label">Hostname: {posture?.hostname || 'localhost'}</p>
                  </div>
                </div>

                <div className="posture-card">
                  <div className="card-header">
                    <h3>{t('diskHeader')}</h3>
                    {posture?.disk_encrypted ? (
                      <IconShieldCheck size={20} className="green-icon" />
                    ) : (
                      <IconAlertCircle size={20} className="red-icon" />
                    )}
                  </div>
                  <div className="card-body" style={{ display: 'flex', flexDirection: 'column', height: '100%', justifyContent: 'space-between' }}>
                    <div>
                      <p className="value">{posture?.disk_encrypted ? t('encrypted') : t('notDetected')}</p>
                      <p className="label">{t('diskDesc')}</p>
                    </div>
                    {!posture?.disk_encrypted && (
                      <button 
                        className="btn btn-secondary"
                        onClick={() => setShowLUKSWizard(true)}
                        style={{ marginTop: '16px', padding: '6px 12px', fontSize: '12px', alignSelf: 'flex-start' }}
                      >
                        {t('btnRemediate')}
                      </button>
                    )}
                  </div>
                </div>

                <div className="posture-card">
                  <div className="card-header">
                    <h3>{t('firewallHeader')}</h3>
                    {posture?.firewall_status === 'active' ? (
                      <IconShieldCheck size={20} className="green-icon" />
                    ) : (
                      <IconAlertCircle size={20} className="red-icon" />
                    )}
                  </div>
                  <div className="card-body" style={{ display: 'flex', flexDirection: 'column', height: '100%', justifyContent: 'space-between' }}>
                    <div>
                      <p className="value">{posture?.firewall_status === 'active' ? t('firewallActive') : t('firewallDisabled')}</p>
                      <p className="label">{t('firewallDesc')}</p>
                    </div>
                    {posture?.firewall_status !== 'active' && (
                      <button 
                        className="btn btn-secondary"
                        onClick={handleRemediateFirewall}
                        disabled={remediatingMap.firewall}
                        style={{ marginTop: '16px', padding: '6px 12px', fontSize: '12px', alignSelf: 'flex-start' }}
                      >
                        {remediatingMap.firewall ? t('remediating') : t('btnRemediateFirewall')}
                      </button>
                    )}
                  </div>
                </div>

                <div className="posture-card">
                  <div className="card-header">
                    <h3>{t('bootHeader')}</h3>
                    {posture?.secure_boot ? (
                      <IconShieldCheck size={20} className="green-icon" />
                    ) : (
                      <IconAlertCircle size={20} className="gold-icon" />
                    )}
                  </div>
                  <div className="card-body" style={{ display: 'flex', flexDirection: 'column', height: '100%', justifyContent: 'space-between' }}>
                    <div>
                      <p className="value">{posture?.secure_boot ? t('bootEnabled') : t('bootDisabled')}</p>
                      <p className="label">{t('bootDesc')}</p>
                    </div>
                    {!posture?.secure_boot && (
                      <button 
                        className="btn btn-secondary"
                        onClick={() => setShowSecureBootWizard(true)}
                        style={{ marginTop: '16px', padding: '6px 12px', fontSize: '12px', alignSelf: 'flex-start' }}
                      >
                        {t('btnRemediateSecureBoot')}
                      </button>
                    )}
                  </div>
                </div>
              </div>

              <h4 className="posture-group-title" style={{ margin: '32px 0 16px 0', fontSize: '13px', fontWeight: '700', color: 'var(--accent-gold)', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                {t('identityAccessGroup')}
              </h4>
              <div className="posture-grid">
                <div className="posture-card">
                  <div className="card-header">
                    <h3>{t('screenLockHeader')}</h3>
                    {posture?.screen_lock_active ? (
                      <IconShieldCheck size={20} className="green-icon" />
                    ) : (
                      <IconAlertCircle size={20} className="gold-icon" />
                    )}
                  </div>
                  <div className="card-body" style={{ display: 'flex', flexDirection: 'column', height: '100%', justifyContent: 'space-between' }}>
                    <div>
                      <p className="value">{posture?.screen_lock_active ? t('screenLockEnabled') : t('screenLockDisabled')}</p>
                      <p className="label">{t('screenLockDesc')}</p>
                    </div>
                    {!posture?.screen_lock_active && (
                      <button 
                        className="btn btn-secondary"
                        onClick={handleRemediateScreenLock}
                        disabled={remediatingMap.screenLock}
                        style={{ marginTop: '16px', padding: '6px 12px', fontSize: '12px', alignSelf: 'flex-start' }}
                      >
                        {remediatingMap.screenLock ? t('remediating') : t('btnRemediateScreenLock')}
                      </button>
                    )}
                  </div>
                </div>

                <div className="posture-card">
                  <div className="card-header">
                    <h3>{t('sshKeysHeader')}</h3>
                    {posture?.ssh_keys_secure ? (
                      <IconShieldCheck size={20} className="green-icon" />
                    ) : (
                      <IconAlertCircle size={20} className="red-icon" />
                    )}
                  </div>
                  <div className="card-body" style={{ display: 'flex', flexDirection: 'column', height: '100%', justifyContent: 'space-between' }}>
                    <div>
                      <p className="value">{posture?.ssh_keys_secure ? t('sshKeysSecure') : t('sshKeysUnsecure')}</p>
                      <p className="label">{t('sshKeysDesc')}</p>
                    </div>
                    {!posture?.ssh_keys_secure && (
                      <button 
                        className="btn btn-secondary"
                        onClick={() => setShowSSHKeysWizard(true)}
                        style={{ marginTop: '16px', padding: '6px 12px', fontSize: '12px', alignSelf: 'flex-start' }}
                      >
                        {t('btnRemediateSSHKeys')}
                      </button>
                    )}
                  </div>
                </div>
              </div>

              <h4 className="posture-group-title" style={{ margin: '32px 0 16px 0', fontSize: '13px', fontWeight: '700', color: 'var(--accent-gold)', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                {t('networkSecurityGroup')}
              </h4>
              <div className="posture-grid" style={{ gridTemplateColumns: '1fr' }}>
                <div className="posture-card">
                  <div className="card-header">
                    <h3>{t('vpnHeader')}</h3>
                    {posture?.vpn_active ? (
                      <IconShieldCheck size={20} className="green-icon" />
                    ) : (
                      <IconInfoCircle size={20} className="gray-icon" />
                    )}
                  </div>
                  <div className="card-body" style={{ display: 'flex', flexDirection: 'column', height: '100%', justifyContent: 'space-between' }}>
                    <div>
                      <p className="value">{posture?.vpn_active ? t('vpnConnected') : t('vpnDisconnected')}</p>
                      <p className="label">{t('vpnDesc')}</p>
                    </div>
                    {!posture?.vpn_active && (
                      <button 
                        className="btn btn-secondary"
                        onClick={() => setShowVPNWizard(true)}
                        style={{ marginTop: '16px', padding: '6px 12px', fontSize: '12px', alignSelf: 'flex-start' }}
                      >
                        {t('btnRemediateVPN')}
                      </button>
                    )}
                  </div>
                </div>
              </div>

              {/* Compliance & Posture Audit Log */}
              <div className="compliance-log-section" style={{ marginTop: '36px', borderTop: '1px solid var(--border-color)', paddingTop: '28px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
                  <div>
                    <h3 className="section-title" style={{ fontSize: '16px', margin: 0, display: 'flex', alignItems: 'center', gap: '8px' }}>
                      <IconHistory size={20} className="gold-icon" />
                      {t('complianceLogTitle')}
                    </h3>
                    <p className="section-subtitle" style={{ fontSize: '12px', margin: '4px 0 0 0' }}>{t('complianceLogSubtitle')}</p>
                  </div>
                  <button 
                    onClick={handleExportAuditReport}
                    className="btn btn-secondary"
                    style={{ fontSize: '12px', padding: '8px 16px', display: 'flex', alignItems: 'center', gap: '6px' }}
                  >
                    <IconDownload size={14} />
                    <span>{t('exportReportBtn')}</span>
                  </button>
                </div>

                <div style={{ background: 'var(--bg-panel)', border: '1px solid var(--border-color)', borderRadius: '12px', overflow: 'hidden' }}>
                  {complianceEvents.length > 0 ? (
                    <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '12px', textAlign: 'left' }}>
                      <thead>
                        <tr style={{ background: 'rgba(255,255,255,0.02)', borderBottom: '1px solid var(--border-color)' }}>
                          <th style={{ padding: '10px 14px', color: 'var(--text-muted)', fontWeight: 'bold' }}>{t('eventColTime')}</th>
                          <th style={{ padding: '10px 14px', color: 'var(--text-muted)', fontWeight: 'bold' }}>{t('eventColControl')}</th>
                          <th style={{ padding: '10px 14px', color: 'var(--text-muted)', fontWeight: 'bold' }}>{t('eventColStatus')}</th>
                          <th style={{ padding: '10px 14px', color: 'var(--text-muted)', fontWeight: 'bold' }}>{t('eventColDetails')}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {complianceEvents.map((evt, idx) => (
                          <tr key={idx} style={{ borderBottom: idx < complianceEvents.length - 1 ? '1px solid var(--border-color)' : 'none' }}>
                            <td style={{ padding: '10px 14px', color: 'var(--text-muted)', whiteSpace: 'nowrap' }}>{new Date(evt.timestamp).toLocaleString()}</td>
                            <td style={{ padding: '10px 14px', fontWeight: '600', color: 'white', textTransform: 'capitalize' }}>{evt.control}</td>
                            <td style={{ padding: '10px 14px' }}>
                              <span style={{ 
                                color: evt.status === 'active' ? 'var(--success-color)' : 'var(--danger-color)', 
                                fontWeight: 'bold',
                                fontSize: '11px',
                                background: evt.status === 'active' ? 'rgba(16, 185, 129, 0.1)' : 'rgba(239, 68, 68, 0.1)',
                                padding: '2px 8px',
                                borderRadius: '12px'
                              }}>
                                {evt.status.toUpperCase()}
                              </span>
                            </td>
                            <td style={{ padding: '10px 14px', color: 'var(--text-secondary)' }}>{evt.details}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  ) : (
                    <div style={{ padding: '24px', textAlign: 'center', color: 'var(--text-muted)', fontSize: '13px' }}>
                      {t('noComplianceLogs')}
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {activeTab === 'tokens' && (
            <div className="tab-pane">
              <h2 className="section-title">{t('tokensTitle')}</h2>
              <p className="section-subtitle">{t('tokensSubtitle')}</p>

              {/* Connection Status Cockpit Panel */}
              <div className="token-cockpit-bar">
                <div className="cockpit-indicator">
                  <div className="pulse-indicator active"></div>
                  <div className="indicator-details">
                    <span className="indicator-label">{t('statusAgent')}</span>
                    <span className="indicator-value active">{t('statusRunning')}</span>
                  </div>
                </div>

                <div className="cockpit-indicator">
                  <div className={`pulse-indicator ${certs.length > 0 ? 'active' : 'inactive'}`}></div>
                  <div className="indicator-details">
                    <span className="indicator-label">{t('statusUsbToken')}</span>
                    <span className={`indicator-value ${certs.length > 0 ? 'active' : 'inactive'}`}>
                      {certs.length > 0 ? t('statusDetected') : t('statusNotDetected')}
                    </span>
                  </div>
                </div>

                <div className="cockpit-indicator attestation-indicator">
                  <IconShieldCheck size={18} className={certs.length > 0 ? 'gold-icon' : 'gray-icon'} />
                  <div className="indicator-details">
                    <span className="indicator-label">{t('tokenAttestation')}</span>
                    <span className="indicator-value">{certs.length > 0 ? t('tokenAttested') : 'N/A'}</span>
                  </div>
                </div>
              </div>

              {/* TPM Hardware Security Module Attestation Info */}
              {tpmInfo && (
                <div className="tpm-attestation-card" style={{ 
                  background: 'rgba(255, 255, 255, 0.01)', 
                  border: '1px solid var(--border-color)', 
                  borderRadius: '16px', 
                  padding: '20px', 
                  marginBottom: '24px',
                  display: 'flex',
                  gap: '20px',
                  alignItems: 'center',
                  backdropFilter: 'blur(20px)'
                }}>
                  <div style={{
                    width: '48px',
                    height: '48px',
                    borderRadius: '12px',
                    background: tpmInfo.present ? 'rgba(212, 175, 55, 0.1)' : 'rgba(255, 255, 255, 0.03)',
                    border: tpmInfo.present ? '1px solid rgba(212, 175, 55, 0.25)' : '1px solid var(--border-color)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: tpmInfo.present ? 'var(--accent-gold)' : 'var(--text-muted)'
                  }}>
                    <IconCpu size={24} />
                  </div>
                  <div style={{ flex: 1 }}>
                    <h3 style={{ fontSize: '15px', fontWeight: '700', color: 'white', margin: 0 }}>{t('tpmTitle')}</h3>
                    <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: '2px 0 0 0' }}>{tpmInfo.present ? t('tpmSubtitle') : t('tpmNotPresent')}</p>
                    
                    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '16px', marginTop: '14px' }}>
                      <div>
                        <span style={{ display: 'block', fontSize: '10px', color: 'var(--text-muted)', textTransform: 'uppercase' }}>{t('tpmType')}</span>
                        <span style={{ fontSize: '12px', fontWeight: '600', color: 'white' }}>{tpmInfo.type}</span>
                      </div>
                      <div>
                        <span style={{ display: 'block', fontSize: '10px', color: 'var(--text-muted)', textTransform: 'uppercase' }}>{t('tpmVersion')}</span>
                        <span style={{ fontSize: '12px', fontWeight: '600', color: 'white' }}>{tpmInfo.version}</span>
                      </div>
                      <div>
                        <span style={{ display: 'block', fontSize: '10px', color: 'var(--text-muted)', textTransform: 'uppercase' }}>{t('tpmVendor')}</span>
                        <span style={{ fontSize: '12px', fontWeight: '600', color: 'var(--accent-gold)' }}>{tpmInfo.vendor}</span>
                      </div>
                      <div>
                        <span style={{ display: 'block', fontSize: '10px', color: 'var(--text-muted)', textTransform: 'uppercase' }}>{t('tpmAttestation')}</span>
                        <span style={{ 
                          fontSize: '11px', 
                          fontWeight: 'bold', 
                          color: tpmInfo.present ? 'var(--success-color)' : 'var(--danger-color)',
                          background: tpmInfo.present ? 'rgba(16, 185, 129, 0.1)' : 'rgba(239, 68, 68, 0.1)',
                          padding: '1px 6px',
                          borderRadius: '4px',
                          display: 'inline-block',
                          marginTop: '2px'
                        }}>
                          {tpmInfo.present ? t('tpmPresent') : t('tpmNotPresent')}
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {certs.length === 0 ? (
                <div className="empty-state">
                  <IconDeviceUsb size={48} className="gray-icon" />
                  <p>{t('noTokens')}</p>
                  <span>{t('insertTokenDesc')}</span>
                </div>
              ) : (
                <div className="cert-list-premium">
                  {certs.map((cert, index) => {
                    const pct = getValidityPercentage(cert.valid_from, cert.valid_to);
                    return (
                      <div className="cert-card-premium" key={index}>
                        {/* High-Fidelity Smart Card Representation */}
                        <div className="smart-card-graphic">
                          <div className="card-overlay-glow"></div>
                          <div className="card-top-row">
                            <div className="chip-icon">
                              <svg width="38" height="30" viewBox="0 0 38 30" fill="none" xmlns="http://www.w3.org/2000/svg">
                                <rect x="1" y="1" width="36" height="28" rx="4" stroke="#D4AF37" strokeWidth="1.5" fill="rgba(212,175,55,0.05)"/>
                                <path d="M12 1V29M26 1V29M1 10H37M1 20H37M12 10C12 13 15 15 19 15C23 15 26 13 26 10M12 20C12 17 15 15 19 15" stroke="#D4AF37" strokeWidth="1.5"/>
                              </svg>
                            </div>
                            <div className="hardware-type">
                              <span className="hsm-badge">PKCS#11 HSM</span>
                            </div>
                          </div>
                          <div className="card-middle-row">
                            <h3 className="card-holder-name">{cert.subject || cert.label || 'Unnamed USB Token'}</h3>
                            <span className="card-vendor-label">{cert.issuer || 'Viettel-CA SHA2'}</span>
                          </div>
                          <div className="card-bottom-row">
                            <div className="card-serial-code">
                              <span className="serial-label">ID:</span>
                              <span className="serial-value">{cert.id || 'usb_auto_detected'}</span>
                            </div>
                            <div className="card-date-badge">
                              <span className="status-label">{t('statusDetected')}</span>
                            </div>
                          </div>
                        </div>

                        {/* Structured Certificate Details Grid */}
                        <div className="cert-details-grid">
                          <div className="detail-item">
                            <span className="detail-label">{t('tokenSubject')}:</span>
                            <span className="detail-value highlight">{cert.subject || 'Unknown'}</span>
                          </div>
                          <div className="detail-item">
                            <span className="detail-label">{t('tokenIssuer')}:</span>
                            <span className="detail-value">{cert.issuer || 'Unknown'}</span>
                          </div>
                          <div className="detail-item">
                            <span className="detail-label">{t('tokenSerial')}:</span>
                            <span className="detail-value code-font">
                              {cert.serial || 'N/A'}
                              {cert.serial && cert.serial !== 'N/A' && (
                                <button 
                                  className="btn-copy-mini"
                                  onClick={() => {
                                    navigator.clipboard.writeText(cert.serial || '');
                                    showToast(t('copiedClipboard'), 'success');
                                  }}
                                  title="Copy Serial Number"
                                >
                                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
                                </button>
                              )}
                            </span>
                          </div>
                          <div className="detail-item">
                            <span className="detail-label">{t('tokenUsage')}:</span>
                            <span className="detail-value">{t('tokenUsageDesc')}</span>
                          </div>
                        </div>

                        {/* Validity Timeline Progress Bar */}
                        <div className="validity-timeline-container">
                          <div className="timeline-dates">
                            <span><strong>{t('tokenValidFrom')}:</strong> {cert.valid_from || 'N/A'}</span>
                            <span><strong>{t('tokenValidTo')}:</strong> {cert.valid_to || 'N/A'}</span>
                          </div>
                          <div className="timeline-track">
                            <div className="timeline-fill" style={{ width: `${pct}%` }}></div>
                          </div>
                          <div className="timeline-status">
                            <span className="status-percent">{pct}% Elapsed</span>
                            <span className="status-status-active">
                              <span className="status-dot"></span>
                              Active & Secure
                            </span>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}

              {/* Digital Signature Audit Trail */}
              <div className="sig-history-section">
                <h3 className="section-title">
                  <IconShieldCheck size={20} className="gold-icon" />
                  {t('sigHistoryTitle')}
                </h3>
                <p className="section-subtitle">
                  {t('sigHistorySubtitle')}
                </p>

                {sigHistory.length === 0 ? (
                  <div className="empty-state" style={{ padding: '32px 0' }}>
                    <p style={{ color: 'var(--text-muted)', fontSize: '13px' }}>{t('noHistory')}</p>
                  </div>
                ) : (
                  <div className="sig-history-table-container">
                    <table className="sig-history-table">
                      <thead>
                        <tr>
                          <th>{t('historyColTime')}</th>
                          <th>{t('historyColApp')}</th>
                          <th>{t('historyColCert')}</th>
                          <th>{t('historyColHash')}</th>
                          <th>{t('historyColStatus')}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {sigHistory.map((item, idx) => (
                          <tr key={idx}>
                            <td className="sig-time-cell">
                              {new Date(item.timestamp).toLocaleString()}
                            </td>
                            <td className="sig-app-cell">
                              {item.origin || 'Local Application'}
                              {item.referer && (
                                <span className="sig-referer-sub" title={item.referer}>
                                  {item.referer}
                                </span>
                              )}
                            </td>
                            <td>
                              <div className="sig-cert-subject">{item.subject}</div>
                              <div className="sig-cert-id">ID: {item.cert_id}</div>
                            </td>
                            <td className="sig-hash-cell" title={item.hash}>
                              {item.hash.substring(0, 16)}...
                            </td>
                            <td>
                              <span className={`sig-status-badge ${
                                item.status.startsWith('success') 
                                  ? 'success' 
                                  : item.status.startsWith('cancelled')
                                    ? 'cancelled'
                                    : 'failed'
                              }`}>
                                {item.status}
                              </span>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </div>
            </div>
          )}



          {activeTab === 'settings' && (
            <div className="tab-pane">
              <h2 className="section-title">{t('settingsTitle')}</h2>
              <p className="section-subtitle">{t('settingsSubtitle')}</p>

              <div className="settings-section">
                <h3 className="settings-section-title">{t('langSection')}</h3>
                <p className="settings-section-desc">{t('langDesc')}</p>
                <div className="settings-control-row">
                  <label>{t('langLabel')}</label>
                  <select 
                    value={language} 
                    onChange={(e) => {
                      const val = e.target.value;
                      setLanguage(val);
                      localStorage.setItem('uid-agent-lang', val);
                    }}
                    className="settings-select"
                  >
                    {SUPPORTED_LANGUAGES.map((lang) => (
                      <option key={lang.code} value={lang.code}>
                        {lang.name}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              {/* IPC and Daemon Diagnostics Debugger */}
              <div className="settings-section" style={{ marginTop: '24px' }}>
                <h3 className="settings-section-title">
                  {t('maintenanceTitle')}
                </h3>
                <p className="settings-section-desc">
                  {t('maintenanceDesc')}
                </p>
                
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px', marginTop: '12px' }}>
                  <div style={{ 
                    background: 'rgba(0,0,0,0.15)', 
                    border: '1px solid var(--border-color)', 
                    borderRadius: '12px', 
                    padding: '16px',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '14px'
                  }}>
                    <div style={{
                      width: '8px',
                      height: '8px',
                      borderRadius: '50%',
                      background: daemonActive === 'active' ? 'var(--success-color)' : 'var(--danger-color)',
                      boxShadow: daemonActive === 'active' ? '0 0 12px var(--success-color)' : 'none',
                      animation: daemonActive === 'active' ? 'pulse 2s infinite' : 'none'
                    }} />
                    <div>
                      <span style={{ display: 'block', fontSize: '11px', color: 'var(--text-muted)', fontWeight: 'bold', textTransform: 'uppercase' }}>
                        {t('daemonStatusLabel')}
                      </span>
                      <span style={{ fontSize: '13px', fontWeight: '600', color: 'white' }}>
                        {daemonActive === 'active' ? t('statusListening') : daemonActive === 'inactive' ? t('statusUnreachable') : t('statusChecking')}
                      </span>
                    </div>
                  </div>

                  <div style={{ 
                    background: 'rgba(0,0,0,0.15)', 
                    border: '1px solid var(--border-color)', 
                    borderRadius: '12px', 
                    padding: '16px',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '14px'
                  }}>
                    <div style={{
                      width: '8px',
                      height: '8px',
                      borderRadius: '50%',
                      background: ipcActive === 'active' ? 'var(--success-color)' : 'var(--danger-color)',
                      boxShadow: ipcActive === 'active' ? '0 0 12px var(--success-color)' : 'none',
                      animation: ipcActive === 'active' ? 'pulse 2s infinite' : 'none'
                    }} />
                    <div>
                      <span style={{ display: 'block', fontSize: '11px', color: 'var(--text-muted)', fontWeight: 'bold', textTransform: 'uppercase' }}>
                        {t('extensionHostStatusLabel')}
                      </span>
                      <span style={{ fontSize: '13px', fontWeight: '600', color: 'white' }}>
                        {ipcActive === 'active' ? t('statusIpcListening') : ipcActive === 'inactive' ? t('statusUnreachable') : t('statusChecking')}
                      </span>
                    </div>
                  </div>
                </div>
              </div>

              <div className="settings-section" style={{ marginTop: '24px' }}>
                <h3 className="settings-section-title">{t('extSection')}</h3>
                <p className="settings-section-desc">{t('extDesc')}</p>
                <div className="update-card-row">
                  <div style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
                    <span>{t('extStatusLabel') || 'UID Link Extension'}</span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    {showExtSettings && (
                      <input 
                        type="text" 
                        placeholder="Extension ID (Optional)"
                        value={customExtId}
                        onChange={(e) => setCustomExtId(e.target.value)}
                        className="settings-input"
                        style={{ width: '180px', margin: 0 }}
                      />
                    )}
                    <button 
                      onClick={() => setShowExtSettings(!showExtSettings)}
                      className={`btn btn-secondary`}
                      title="Custom Extension ID"
                      style={{ padding: '6px 10px', display: 'inline-flex', alignItems: 'center', justifyContent: 'center' }}
                    >
                      <IconSettings className="w-4 h-4" stroke={1.5} />
                    </button>
                    <button 
                      onClick={handleInstallExtension}
                      className="btn btn-primary"
                      disabled={installingExt}
                    >
                      {installingExt ? t('installingExtension') : t('btnInstallExtension')}
                    </button>
                  </div>
                </div>
              </div>

              <div className="settings-section" style={{ marginTop: '24px' }}>
                <h3 className="settings-section-title">{t('updateSection')}</h3>
                <p className="settings-section-desc">{t('updateDesc')}</p>
                <div className="update-card-row">
                  <div style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
                    <span>{renderVersionInfo()}</span>
                  </div>
                   <button 
                    onClick={handleCheckUpdate}
                    className="btn btn-secondary"
                    disabled={checkingUpdate || loading}
                  >
                    {checkingUpdate ? t('btnUpdating') : t('btnCheckUpdate')}
                  </button>
                </div>
              </div>

              {log && (
                log.trim().includes('\n') ? (
                  <div className="log-console" style={{ marginTop: '16px' }}>
                    <h4>{t('consoleLogOutput')}</h4>
                    <pre>{log}</pre>
                  </div>
                ) : (
                  <div style={{ marginTop: '16px', fontSize: '13px', color: 'var(--accent-gold)', paddingLeft: '4px' }}>
                    {log}
                  </div>
                )
              )}
            </div>
          )}

          {activeTab === 'apps' && (
            <div className="tab-pane">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '6px' }}>
                <h2 className="section-title">{t('appsTitle')}</h2>
                <button 
                  className="btn btn-primary" 
                  onClick={() => setShowAddAppModal(true)}
                  style={{ display: 'flex', alignItems: 'center', gap: '6px', padding: '8px 16px', fontSize: '13px' }}
                >
                  <IconPlus size={16} />
                  <span>{t('addAppBtn')}</span>
                </button>
              </div>
              <p className="section-subtitle">{t('appsSubtitle')}</p>

              <div style={{ display: 'grid', gridTemplateColumns: '300px 1fr', gap: '28px', marginTop: '24px' }}>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
                  <div style={{ position: 'relative' }}>
                    <input 
                      type="text" 
                      placeholder={t('searchPlaceholder')}
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      style={{ 
                        width: '100%', 
                        padding: '10px 12px 10px 38px', 
                        background: 'rgba(255,255,255,0.03)', 
                        border: '1px solid var(--border-color)', 
                        borderRadius: '10px',
                        color: 'white',
                        fontSize: '13px'
                      }}
                    />
                    <IconSearch size={16} style={{ position: 'absolute', left: '12px', top: '12px', color: 'var(--text-muted)' }} />
                  </div>

                  <div style={{ display: 'flex', flexDirection: 'column', gap: '10px', maxHeight: '500px', overflowY: 'auto' }}>
                    {sandboxApps
                      .filter(app => app.name.toLowerCase().includes(searchQuery.toLowerCase()))
                      .map(app => (
                        <div 
                          key={app.id}
                          className={`profile-card ${selectedApp?.id === app.id ? 'active' : ''}`}
                          onClick={() => setSelectedApp(app)}
                          style={{ 
                            cursor: 'pointer',
                            background: selectedApp?.id === app.id ? 'rgba(212, 175, 55, 0.1)' : 'rgba(255,255,255,0.02)',
                            borderColor: selectedApp?.id === app.id ? 'rgba(212, 175, 55, 0.3)' : 'var(--border-color)',
                            padding: '12px 14px'
                          }}
                        >
                          <div className="profile-avatar" style={{ background: app.status === 'running' ? 'var(--success-color)' : 'rgba(255,255,255,0.05)' }}>
                            <IconBox size={18} />
                          </div>
                          <div className="profile-info">
                            <span className="profile-name">{app.name}</span>
                            <span className="profile-email" style={{ 
                              color: app.status === 'running' ? 'var(--success-color)' : app.status === 'stopped' ? 'var(--text-muted)' : 'var(--warning-color)',
                              fontWeight: 600,
                              fontSize: '10px',
                              display: 'flex',
                              alignItems: 'center',
                              gap: '4px'
                            }}>
                              <span style={{ 
                                width: '6px', 
                                height: '6px', 
                                borderRadius: '50%', 
                                background: app.status === 'running' ? 'var(--success-color)' : app.status === 'stopped' ? 'var(--text-muted)' : 'var(--warning-color)',
                                display: 'inline-block'
                              }} />
                              {app.status === 'running' ? t('statusRunning') : app.status === 'stopped' ? t('statusStopped') : t('statusNotConfigured')}
                            </span>
                          </div>
                        </div>
                      ))}
                  </div>
                </div>

                <div>
                  {selectedApp ? (
                    <div style={{ background: 'var(--bg-panel)', border: '1px solid var(--border-color)', borderRadius: '16px', padding: '24px', backdropFilter: 'blur(20px)', height: '100%', display: 'flex', flexDirection: 'column', justifyContent: 'space-between', gap: '20px' }}>
                      <div style={{ display: 'flex', flexDirection: 'column', gap: '20px', flex: 1, overflowY: 'auto' }}>
                        {/* Header */}
                        <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
                          <div style={{ 
                            width: '56px', 
                            height: '56px', 
                            borderRadius: '14px', 
                            background: selectedApp.status === 'running' ? 'rgba(16, 185, 129, 0.15)' : 'rgba(255,255,255,0.03)',
                            border: selectedApp.status === 'running' ? '1px solid rgba(16, 185, 129, 0.3)' : '1px solid var(--border-color)',
                            display: 'flex', 
                            alignItems: 'center', 
                            justifyContent: 'center',
                            color: selectedApp.status === 'running' ? 'var(--success-color)' : 'var(--accent-gold)'
                          }}>
                            <IconBox size={28} />
                          </div>
                          <div style={{ flex: 1 }}>
                            <h3 style={{ fontSize: '20px', fontWeight: '700', color: 'white', display: 'flex', alignItems: 'center', gap: '8px' }}>
                              {selectedApp.name}
                            </h3>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flexWrap: 'wrap', marginTop: '4px' }}>
                              <span style={{ 
                                fontSize: '11px', 
                                color: selectedApp.status === 'running' ? 'var(--success-color)' : selectedApp.status === 'stopped' ? 'var(--text-muted)' : 'var(--warning-color)',
                                fontWeight: '600',
                                display: 'flex',
                                alignItems: 'center',
                                gap: '6px'
                              }}>
                                <span style={{ 
                                  width: '6px', 
                                  height: '6px', 
                                  borderRadius: '50%', 
                                  background: selectedApp.status === 'running' ? 'var(--success-color)' : selectedApp.status === 'stopped' ? 'var(--text-muted)' : 'var(--warning-color)',
                                  display: 'inline-block'
                                }} />
                                {selectedApp.status === 'running' ? t('statusRunning') : selectedApp.status === 'stopped' ? t('statusInstalled') : t('statusNotInstalled')}
                              </span>

                              {selectedApp.status !== 'not_configured' && (
                                <span style={{ 
                                  fontSize: '11px', 
                                  color: 'var(--accent-gold)', 
                                  background: 'rgba(212, 175, 55, 0.1)', 
                                  border: '1px solid rgba(212, 175, 55, 0.2)',
                                  padding: '1px 8px', 
                                  borderRadius: '12px',
                                  fontWeight: '600'
                                }}>
                                  {t('storageSizeLabel')}: {appStorageSizes[selectedApp.id] || '...'}
                                </span>
                              )}
                            </div>
                          </div>
                        </div>

                        {/* Onboarding View for Not Configured status */}
                        {selectedApp.status === 'not_configured' ? (
                          <div style={{ 
                            background: 'rgba(255,255,255,0.01)', 
                            border: '1px dashed var(--border-color)', 
                            borderRadius: '12px', 
                            padding: '24px', 
                            textAlign: 'center',
                            display: 'flex',
                            flexDirection: 'column',
                            alignItems: 'center',
                            gap: '12px',
                            margin: '10px 0'
                          }}>
                            <IconFingerprint size={40} style={{ color: 'var(--accent-gold)', opacity: 0.6 }} />
                            <h4 style={{ fontSize: '15px', fontWeight: '700', color: 'white' }}>{t('onboardingConfigTitle')}</h4>
                            <p style={{ fontSize: '13px', color: 'var(--text-muted)', lineHeight: '1.5', maxWidth: '360px' }}>
                              {t('onboardingConfigDesc')}
                            </p>
                            
                            {selectedApp.isInstalling && (
                              <div style={{ width: '100%', marginTop: '10px' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', marginBottom: '6px', color: 'var(--text-muted)' }}>
                                  <span>{t('initializingProfile')}</span>
                                  <span>{selectedApp.installProgress}%</span>
                                </div>
                                <div className="timeline-track" style={{ height: '6px' }}>
                                  <div className="timeline-fill" style={{ width: `${selectedApp.installProgress}%`, background: 'var(--accent-gold)' }} />
                                </div>
                              </div>
                            )}

                            <button 
                              className="btn btn-primary" 
                              disabled={selectedApp.isInstalling}
                              onClick={() => handleInstallApp(selectedApp.id)}
                              style={{ marginTop: '8px', padding: '10px 24px', fontSize: '13px', display: 'flex', alignItems: 'center', gap: '8px' }}
                            >
                              {selectedApp.isInstalling ? <IconLoader2 size={16} className="animate-spin" /> : <IconPlus size={16} />}
                              <span>{selectedApp.isInstalling ? t('configuringStatus') : t('btnInitializeProfile')}</span>
                            </button>
                          </div>
                        ) : (
                          /* Interactive App Launcher Card */
                          <div 
                            onClick={() => selectedApp.status === 'stopped' && handleLaunchApp(selectedApp.id)}
                            onMouseEnter={() => selectedApp.status === 'stopped' && setIsCardHovered(true)}
                            onMouseLeave={() => setIsCardHovered(false)}
                            style={{ 
                              background: selectedApp.status === 'running' ? 'rgba(16, 185, 129, 0.03)' : 'rgba(255,255,255,0.02)', 
                              border: selectedApp.status === 'running' ? '1px solid rgba(16, 185, 129, 0.2)' : '1px solid var(--border-color)', 
                              borderRadius: '12px', 
                              padding: '24px 20px', 
                              textAlign: 'center',
                              cursor: selectedApp.status === 'stopped' ? 'pointer' : 'default',
                              transition: 'all 0.2s ease',
                              transform: (selectedApp.status === 'stopped' && isCardHovered) ? 'translateY(-2px)' : 'none',
                              boxShadow: (selectedApp.status === 'stopped' && isCardHovered) ? '0 8px 24px rgba(212, 175, 55, 0.1)' : 'none',
                              display: 'flex',
                              flexDirection: 'column',
                              alignItems: 'center',
                              gap: '12px'
                            }}
                          >
                            <div style={{ 
                              width: '48px', 
                              height: '48px', 
                              borderRadius: '50%', 
                              background: selectedApp.status === 'running' ? 'var(--success-color)' : 'rgba(255,255,255,0.05)',
                              display: 'flex',
                              alignItems: 'center',
                              justifyContent: 'center',
                              color: selectedApp.status === 'running' ? 'white' : 'var(--text-muted)',
                              transition: 'all 0.2s ease'
                            }}>
                              {selectedApp.status === 'running' ? <IconCheck size={24} /> : <IconPlayerPlay size={24} />}
                            </div>
                            <div>
                              <span style={{ fontSize: '14px', fontWeight: '600', color: 'white' }}>
                                {selectedApp.status === 'running' ? t('runningSecureSandbox') : t('clickLaunchContainer')}
                              </span>
                              <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>{selectedApp.description}</p>
                            </div>
                          </div>
                        )}

                        {/* Session Sync Info Note */}
                        {selectedApp.status !== 'not_configured' && (
                          <div style={{
                            background: 'rgba(212, 175, 55, 0.05)',
                            border: '1px solid rgba(212, 175, 55, 0.15)',
                            borderRadius: '12px',
                            padding: '14px 16px',
                            display: 'flex',
                            gap: '12px',
                            alignItems: 'start',
                            marginTop: '12px'
                          }}>
                            <IconInfoCircle className="gold-icon" size={20} style={{ flexShrink: 0, marginTop: '2px' }} />
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
                              <span style={{ fontSize: '12px', fontWeight: '700', color: 'var(--accent-gold)' }}>
                                {t('warningDedicatedWorkspace')}
                              </span>
                              <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: 0, lineHeight: 1.4 }}>
                                {t('warningWorkspaceDesc')}
                              </p>
                            </div>
                          </div>
                        )}

                        {/* Profile Sync Controls */}
                        {selectedApp.status !== 'not_configured' && (
                          <div style={{
                            background: 'rgba(255, 255, 255, 0.02)',
                            border: '1px solid var(--border-color)',
                            borderRadius: '12px',
                            padding: '16px',
                            display: 'flex',
                            flexDirection: 'column',
                            gap: '12px',
                            marginTop: '12px'
                          }}>
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                              <span style={{ fontSize: '13px', fontWeight: 'bold', color: 'white' }}>
                                {t('syncSectionTitle')}
                              </span>
                              <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                                {t('lastSyncedLabel')}: {syncStatus[selectedApp.id] || t('neverSynced')}
                              </span>
                            </div>
                            <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: 0, lineHeight: 1.4 }}>
                              {t('syncSectionDesc')}
                            </p>
                            <div style={{ display: 'flex', gap: '10px' }}>
                              <button
                                className="btn btn-secondary"
                                onClick={() => handleSyncProfile(selectedApp.id, selectedApp.url, 'import')}
                                disabled={isSyncing[selectedApp.id]}
                                style={{ flex: 1, padding: '8px 12px', fontSize: '11px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '4px' }}
                              >
                                {isSyncing[selectedApp.id] ? <IconLoader2 size={14} className="animate-spin" /> : <IconDownload size={14} />}
                                <span style={{ whiteSpace: 'nowrap' }}>{t('btnImportFromChrome')}</span>
                              </button>
                              <button
                                className="btn btn-secondary"
                                onClick={() => handleSyncProfile(selectedApp.id, selectedApp.url, 'export')}
                                disabled={isSyncing[selectedApp.id]}
                                style={{ flex: 1, padding: '8px 12px', fontSize: '11px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '4px' }}
                              >
                                <IconUpload size={14} />
                                <span style={{ whiteSpace: 'nowrap' }}>{t('btnExportBackup')}</span>
                              </button>
                              <button
                                className="btn btn-secondary"
                                onClick={() => handleSyncProfile(selectedApp.id, selectedApp.url, 'restore')}
                                disabled={isSyncing[selectedApp.id]}
                                style={{ flex: 1, padding: '8px 12px', fontSize: '11px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '4px' }}
                              >
                                <IconHistory size={14} />
                                <span style={{ whiteSpace: 'nowrap' }}>{t('btnRestoreBackup')}</span>
                              </button>
                            </div>
                          </div>
                        )}

                        {/* Collapsible Advanced Developer Options */}
                        {selectedApp.status !== 'not_configured' && (
                          <div style={{ marginTop: '10px', borderTop: '1px solid var(--border-color)', paddingTop: '16px' }}>
                            <div 
                              onClick={() => setShowDevOptions(!showDevOptions)}
                              style={{ 
                                display: 'flex', 
                                justifyContent: 'space-between', 
                                alignItems: 'center', 
                                cursor: 'pointer',
                                padding: '4px 0',
                                color: 'var(--text-muted)',
                                fontSize: '12px',
                                fontWeight: 'bold'
                              }}
                            >
                              <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                                <IconTerminal size={14} />
                                <span>{showDevOptions ? t('hideActiveSettings') : t('showContainerConfig')}</span>
                              </div>
                              <span style={{ fontSize: '10px' }}>{showDevOptions ? '▲' : '▼'}</span>
                            </div>

                            {showDevOptions && (
                              <div style={{ marginTop: '12px', display: 'flex', flexDirection: 'column', gap: '12px' }}>
                                <div style={{ background: 'rgba(0,0,0,0.15)', borderRadius: '8px', padding: '12px', display: 'flex', flexDirection: 'column', gap: '8px' }}>
                                  <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px' }}>
                                    <span style={{ color: 'var(--text-muted)' }}>{t('targetUrlLabel')}:</span>
                                    <span style={{ fontFamily: 'monospace', color: 'var(--accent-gold)' }}>{selectedApp.url}</span>
                                  </div>
                                  <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px' }}>
                                    <span style={{ color: 'var(--text-muted)' }}>{t('storagePathLabel')}:</span>
                                    <span style={{ fontFamily: 'monospace', color: 'white' }}>~/.local/share/uid/apps/{selectedApp.id}</span>
                                  </div>
                                </div>

                                <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                                  <span style={{ fontSize: '11px', color: 'var(--text-muted)', fontWeight: 'bold', textTransform: 'uppercase' }}>{t('consoleLogOutput')}</span>
                                  <div style={{ 
                                    background: 'rgba(0,0,0,0.3)', 
                                    border: '1px solid var(--border-color)', 
                                    borderRadius: '8px', 
                                    padding: '10px', 
                                    fontFamily: 'monospace', 
                                    fontSize: '11px', 
                                    color: 'var(--accent-gold)', 
                                    height: '120px', 
                                    overflowY: 'auto',
                                    display: 'flex',
                                    flexDirection: 'column',
                                    gap: '4px'
                                  }}>
                                    {selectedApp.logs.map((l, i) => <div key={i}>{l}</div>)}
                                  </div>
                                </div>
                              </div>
                            )}
                          </div>
                        )}
                      </div>

                      {/* Footer Actions */}
                      {selectedApp.status !== 'not_configured' && (
                        <div style={{ display: 'flex', gap: '12px', borderTop: '1px solid var(--border-color)', paddingTop: '20px', marginTop: '10px' }}>
                          {selectedApp.status === 'stopped' ? (
                            <>
                              <button 
                                className="btn btn-primary" 
                                onClick={() => handleLaunchApp(selectedApp.id)}
                                style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}
                              >
                                <IconPlayerPlay size={16} />
                                <span>{t('btnLaunchWorkspace')}</span>
                              </button>
                              <button 
                                className="btn btn-secondary" 
                                onClick={async () => {
                                  if (confirm(t('confirmReset') || 'Are you sure you want to clear all storage for this workspace?')) {
                                    try {
                                      await invoke('purge_sandbox_profile', { appId: selectedApp.id });
                                      setSandboxApps(prev => prev.map(app => app.id === selectedApp.id ? { ...app, status: 'not_configured' as const, logs: [] } : app));
                                      setSelectedApp(prev => prev ? { ...prev, status: 'not_configured' as const, logs: [] } : null);
                                      setAppStorageSizes(prev => ({ ...prev, [selectedApp.id]: '0 B' }));
                                      showToast('Workspace storage cleared successfully.', 'success');
                                    } catch (err) {
                                      showToast(`Failed to clear storage: ${err}`, 'error');
                                    }
                                  }
                                }}
                                style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}
                              >
                                <IconTrash size={16} />
                                <span>{t('btnResetStorage')}</span>
                              </button>
                            </>
                          ) : (
                            <>
                              <button 
                                className="btn btn-secondary" 
                                onClick={() => handleStopApp(selectedApp.id)}
                                style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px', border: '1px solid rgba(239, 68, 68, 0.3)', color: 'var(--danger-color)' }}
                              >
                                <IconPlayerStop size={16} />
                                <span>{t('btnStopSandbox')}</span>
                              </button>
                              <button 
                                className="btn btn-secondary" 
                                onClick={() => handlePinApp(selectedApp.id)}
                                style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}
                              >
                                <IconPin size={16} />
                                <span>{t('btnPinShortcut')}</span>
                              </button>
                            </>
                          )}
                        </div>
                      )}
                    </div>
                  ) : (
                    <div className="empty-state" style={{ height: '100%', display: 'flex', justifyContent: 'center' }}>
                      <IconBox size={48} className="gold-icon" style={{ opacity: 0.5 }} />
                      <p>Select an Application</p>
                      <span>Choose an app from the list to launch its secure workspace.</span>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {activeTab === 'approvals' && (
            <div className="tab-pane">
              <h2 className="section-title">{t('approvalsTitle')}</h2>
              <p className="section-subtitle">{t('approvalsSubtitle')}</p>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '20px', marginTop: '24px' }}>
                <h4 style={{ color: 'var(--accent-gold)', fontSize: '13px', fontWeight: '700', textTransform: 'uppercase', letterSpacing: '0.5px' }}>
                  {t('pendingRequestsTitle')}
                </h4>

                {pendingApprovals.length > 0 ? (
                  pendingApprovals.map(req => (
                    <div 
                      key={req.id} 
                      style={{ 
                        background: 'var(--bg-panel)', 
                        border: '1px solid var(--border-color)', 
                        borderRadius: '16px', 
                        padding: '24px', 
                        backdropFilter: 'blur(20px)',
                        display: 'flex',
                        flexDirection: 'column',
                        gap: '16px',
                        position: 'relative',
                        overflow: 'hidden'
                      }}
                    >
                      {req.isSuccess && (
                        <div style={{ 
                          position: 'absolute', 
                          top: 0, 
                          left: 0, 
                          right: 0, 
                          bottom: 0, 
                          background: 'rgba(16, 185, 129, 0.9)', 
                          display: 'flex', 
                          alignItems: 'center', 
                          justifyContent: 'center',
                          zIndex: 10,
                          backdropFilter: 'blur(4px)',
                          animation: 'fadeIn 0.2s ease'
                        }}>
                          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px', color: 'white' }}>
                            <IconCheck size={40} style={{ border: '2px solid white', borderRadius: '50%', padding: '4px' }} />
                            <span style={{ fontWeight: 'bold', fontSize: '16px' }}>{t('attestationSignedSuccess')}</span>
                          </div>
                        </div>
                      )}

                      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                        <div>
                          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                            <span style={{ 
                              background: req.type === 'login' ? 'rgba(37, 99, 255, 0.15)' : 'rgba(212, 175, 55, 0.15)', 
                              border: req.type === 'login' ? '1px solid rgba(37, 99, 255, 0.3)' : '1px solid rgba(212, 175, 55, 0.3)',
                              color: req.type === 'login' ? '#3b82f6' : 'var(--color-primary)',
                              fontSize: '10px',
                              fontWeight: '700',
                              padding: '2px 8px',
                              borderRadius: '20px',
                              textTransform: 'uppercase'
                            }}>
                              {req.type === 'login' ? t('authLoginType') : t('digitalSignType')}
                            </span>
                            <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>{req.timestamp}</span>
                          </div>
                          <h3 style={{ fontSize: '18px', fontWeight: '700', color: 'white', marginTop: '10px' }}>{req.title}</h3>
                          <p style={{ fontSize: '14px', color: 'var(--text-secondary)', marginTop: '4px' }}>{req.description}</p>
                        </div>
                        <span style={{ fontSize: '12px', color: 'var(--text-muted)', fontFamily: 'monospace' }}>{req.origin}</span>
                      </div>

                      <div style={{ background: 'rgba(0,0,0,0.2)', border: '1px solid var(--border-color)', borderRadius: '8px', padding: '12px', display: 'flex', flexDirection: 'column', gap: '4px' }}>
                        <span style={{ fontSize: '11px', color: 'var(--text-muted)', fontWeight: 'bold' }}>{t('challengePayloadLabel')}</span>
                        <code style={{ fontFamily: 'monospace', fontSize: '12px', color: 'var(--accent-gold)', wordBreak: 'break-all' }}>{req.payload}</code>
                      </div>

                      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '12px' }}>
                        <button 
                          className="btn btn-secondary" 
                          onClick={() => handleRejectRequest(req.id)}
                          disabled={req.isSigning}
                          style={{ padding: '8px 20px', fontSize: '13px' }}
                        >
                          {t('btnReject')}
                        </button>
                        <button 
                          className="btn btn-primary" 
                          onClick={() => handleApproveRequest(req.id)}
                          disabled={req.isSigning}
                          style={{ padding: '8px 24px', fontSize: '13px', display: 'flex', alignItems: 'center', gap: '8px' }}
                        >
                          {req.isSigning ? <IconLoader2 size={16} className="animate-spin" /> : <IconFingerprint size={16} />}
                          <span>{req.isSigning ? t('btnVerifying') : t('btnApproveSign')}</span>
                        </button>
                      </div>
                    </div>
                  ))
                ) : (
                  <div className="empty-state" style={{ padding: '48px 24px' }}>
                    <IconFingerprint size={48} className="green-icon" style={{ opacity: 0.8 }} />
                    <p style={{ color: 'var(--success-color)' }}>{t('emptyApprovalsTitle')}</p>
                    <span>{t('emptyApprovalsDesc')}</span>
                  </div>
                )}

                <h4 style={{ color: 'var(--text-muted)', fontSize: '13px', fontWeight: '700', textTransform: 'uppercase', letterSpacing: '0.5px', marginTop: '24px' }}>
                  {t('sigHistoryTitle')}
                </h4>
                <p className="section-subtitle" style={{ marginBottom: '16px' }}>{t('sigHistorySubtitle')}</p>

                <div className="sig-history-card" style={{ background: 'var(--bg-panel)', border: '1px solid var(--border-color)', borderRadius: '16px', overflow: 'hidden' }}>
                  {sigHistory.length > 0 ? (
                    <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px', textAlign: 'left' }}>
                      <thead>
                        <tr style={{ background: 'rgba(255,255,255,0.02)', borderBottom: '1px solid var(--border-color)' }}>
                          <th style={{ padding: '12px 16px', color: 'var(--text-muted)', fontWeight: 'bold' }}>{t('historyColTime')}</th>
                          <th style={{ padding: '12px 16px', color: 'var(--text-muted)', fontWeight: 'bold' }}>{t('historyColApp')}</th>
                          <th style={{ padding: '12px 16px', color: 'var(--text-muted)', fontWeight: 'bold' }}>{t('historyColCert')}</th>
                          <th style={{ padding: '12px 16px', color: 'var(--text-muted)', fontWeight: 'bold' }}>{t('historyColHash')}</th>
                          <th style={{ padding: '12px 16px', color: 'var(--text-muted)', fontWeight: 'bold' }}>{t('historyColStatus')}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {sigHistory.map((hist, index) => (
                          <tr key={index} style={{ borderBottom: index < sigHistory.length - 1 ? '1px solid var(--border-color)' : 'none' }}>
                            <td style={{ padding: '12px 16px', color: 'var(--text-muted)' }}>{new Date(hist.timestamp).toLocaleTimeString()}</td>
                            <td style={{ padding: '12px 16px', fontWeight: '600' }}>{hist.origin || 'Local System'}</td>
                            <td style={{ padding: '12px 16px', color: 'var(--text-secondary)' }}>{hist.subject || 'Attested Key'}</td>
                            <td style={{ padding: '12px 16px' }}><code style={{ color: 'var(--accent-gold)', fontSize: '11px' }}>{hist.hash.substring(0, 16)}...</code></td>
                            <td style={{ padding: '12px 16px' }}><span style={{ color: 'var(--success-color)', fontWeight: 'bold' }}>{hist.status}</span></td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  ) : (
                    <div style={{ padding: '24px', textAlign: 'center', color: 'var(--text-muted)' }}>{t('noHistory')}</div>
                  )}
                </div>
              </div>
            </div>
          )}
        </main>
      </div>

      {showLUKSWizard && (
        <div className="modal-overlay">
          <div className="modal-content glassmorphism" style={{ maxWidth: '600px', width: '90%' }}>
            <div className="modal-header">
              <h3>{t('luksWizardTitle')}</h3>
              <button className="close-modal-btn" onClick={() => setShowLUKSWizard(false)}>
                <IconX size={20} />
              </button>
            </div>
            
            <div className="modal-body" style={{ color: 'var(--text-secondary)', padding: '16px 0', fontSize: '14px', lineHeight: '1.6' }}>
              <div className="alert alert-warning" style={{ backgroundColor: 'rgba(219, 149, 0, 0.1)', border: '1px solid rgba(219, 149, 0, 0.2)', padding: '12px 16px', borderRadius: '8px', display: 'flex', gap: '12px', alignItems: 'center', marginBottom: '20px' }}>
                <IconAlertTriangle className="gold-icon" size={24} style={{ flexShrink: 0 }} />
                <p style={{ margin: 0, fontSize: '13px', color: 'var(--text-primary)' }}>
                  {t('luksWizardWarn')}
                </p>
              </div>

              <div style={{ backgroundColor: 'rgba(0, 0, 0, 0.2)', borderRadius: '12px', padding: '16px', border: '1px solid var(--border-color)' }}>
                <h4 style={{ color: 'var(--text-primary)', marginTop: 0, marginBottom: '8px', fontSize: '15px' }}>{t('luksWizardTabA')}</h4>
                
                <ol style={{ paddingLeft: '20px', display: 'flex', flexDirection: 'column', gap: '8px', fontSize: '13px' }}>
                  <li>
                    {t('luksWizardStep1')}
                    <pre style={{ backgroundColor: 'var(--bg-panel)', padding: '8px', borderRadius: '6px', margin: '4px 0', border: '1px solid var(--border-color)', color: 'var(--accent-gold)', fontFamily: 'monospace' }}>
                      sudo apt update && sudo apt install -y gocryptfs
                    </pre>
                  </li>
                  <li>
                    {t('luksWizardStep2')}
                    <pre style={{ backgroundColor: 'var(--bg-panel)', padding: '8px', borderRadius: '6px', margin: '4px 0', border: '1px solid var(--border-color)', color: 'var(--accent-gold)', fontFamily: 'monospace' }}>
                      mkdir ~/SecureWork-Cipher ~/SecureWork
                    </pre>
                  </li>
                  <li>
                    {t('luksWizardStep3')}
                    <pre style={{ backgroundColor: 'var(--bg-panel)', padding: '8px', borderRadius: '6px', margin: '4px 0', border: '1px solid var(--border-color)', color: 'var(--accent-gold)', fontFamily: 'monospace' }}>
                      gocryptfs -init ~/SecureWork-Cipher
                    </pre>
                  </li>
                  <li>
                    {t('luksWizardStep4')}
                    <pre style={{ backgroundColor: 'var(--bg-panel)', padding: '8px', borderRadius: '6px', margin: '4px 0', border: '1px solid var(--border-color)', color: 'var(--accent-gold)', fontFamily: 'monospace' }}>
                      gocryptfs ~/SecureWork-Cipher ~/SecureWork
                    </pre>
                  </li>
                </ol>
                
                <p style={{ marginTop: '12px', fontSize: '12px', opacity: 0.8, fontStyle: 'italic' }}>
                  {t('luksWizardDesc')}
                </p>
              </div>
            </div>

            <div className="modal-actions" style={{ borderTop: '1px solid var(--border-color)', paddingTop: '16px', display: 'flex', justifyContent: 'flex-end' }}>
              <button className="btn btn-primary" onClick={() => setShowLUKSWizard(false)}>
                {t('btnConfirm')}
              </button>
            </div>
          </div>
        </div>
      )}

      {showFirewallWizard && (
        <div className="modal-overlay">
          <div className="modal-content glassmorphism" style={{ maxWidth: '600px', width: '90%' }}>
            <div className="modal-header">
              <h3>{t('firewallWizardTitle')}</h3>
              <button className="close-modal-btn" onClick={() => setShowFirewallWizard(false)}>
                <IconX size={20} />
              </button>
            </div>
            
            <div className="modal-body" style={{ color: 'var(--text-secondary)', padding: '16px 0', fontSize: '14px', lineHeight: '1.6' }}>
              <div className="alert alert-warning" style={{ backgroundColor: 'rgba(219, 149, 0, 0.1)', border: '1px solid rgba(219, 149, 0, 0.2)', padding: '12px 16px', borderRadius: '8px', display: 'flex', gap: '12px', alignItems: 'center', marginBottom: '20px' }}>
                <IconAlertTriangle className="gold-icon" size={24} style={{ flexShrink: 0 }} />
                <p style={{ margin: 0, fontSize: '13px', color: 'var(--text-primary)' }}>
                  {t('firewallWizardWarn')}
                </p>
              </div>

              <div style={{ backgroundColor: 'rgba(0, 0, 0, 0.2)', borderRadius: '12px', padding: '16px', border: '1px solid var(--border-color)', display: 'flex', flexDirection: 'column', gap: '12px' }}>
                <div>
                  <h4 style={{ color: 'var(--text-primary)', marginTop: 0, marginBottom: '4px', fontSize: '14px' }}>Ubuntu / Debian (UFW)</h4>
                  <pre style={{ backgroundColor: 'var(--bg-panel)', padding: '8px', borderRadius: '6px', border: '1px solid var(--border-color)', color: 'var(--accent-gold)', fontFamily: 'monospace', fontSize: '12px' }}>
                    sudo ufw enable
                  </pre>
                </div>
                <div>
                  <h4 style={{ color: 'var(--text-primary)', marginTop: 0, marginBottom: '4px', fontSize: '14px' }}>Fedora / RHEL (Firewalld)</h4>
                  <pre style={{ backgroundColor: 'var(--bg-panel)', padding: '8px', borderRadius: '6px', border: '1px solid var(--border-color)', color: 'var(--accent-gold)', fontFamily: 'monospace', fontSize: '12px' }}>
                    sudo systemctl enable --now firewalld
                  </pre>
                </div>
              </div>
            </div>

            <div className="modal-actions" style={{ borderTop: '1px solid var(--border-color)', paddingTop: '16px', display: 'flex', justifyContent: 'flex-end' }}>
              <button className="btn btn-primary" onClick={() => setShowFirewallWizard(false)}>
                {t('btnConfirm')}
              </button>
            </div>
          </div>
        </div>
      )}

      {showSecureBootWizard && (
        <div className="modal-overlay">
          <div className="modal-content glassmorphism" style={{ maxWidth: '600px', width: '90%' }}>
            <div className="modal-header">
              <h3>{t('secureBootWizardTitle')}</h3>
              <button className="close-modal-btn" onClick={() => setShowSecureBootWizard(false)}>
                <IconX size={20} />
              </button>
            </div>
            
            <div className="modal-body" style={{ color: 'var(--text-secondary)', padding: '16px 0', fontSize: '14px', lineHeight: '1.6' }}>
              <p style={{ margin: '0 0 16px 0' }}>
                {t('secureBootWizardDesc')}
              </p>

              <div style={{ backgroundColor: 'rgba(0, 0, 0, 0.2)', borderRadius: '12px', padding: '16px', border: '1px solid var(--border-color)', display: 'flex', flexDirection: 'column', gap: '12px' }}>
                <div>
                  <p style={{ fontSize: '13px', margin: '0 0 4px 0' }}>{t('secureBootStep1')}</p>
                  <p style={{ fontSize: '13px', margin: '0 0 4px 0' }}>{t('secureBootStep2')}</p>
                  <p style={{ fontSize: '13px', margin: '0 0 4px 0' }}>{t('secureBootStep3')}</p>
                </div>
              </div>
            </div>

            <div className="modal-actions" style={{ borderTop: '1px solid var(--border-color)', paddingTop: '16px', display: 'flex', justifyContent: 'flex-end' }}>
              <button className="btn btn-primary" onClick={() => setShowSecureBootWizard(false)}>
                {t('btnConfirm')}
              </button>
            </div>
          </div>
        </div>
      )}

      {showSSHKeysWizard && (
        <div className="modal-overlay">
          <div className="modal-content glassmorphism" style={{ maxWidth: '600px', width: '90%' }}>
            <div className="modal-header">
              <h3>{t('sshKeysWizardTitle')}</h3>
              <button className="close-modal-btn" onClick={() => setShowSSHKeysWizard(false)}>
                <IconX size={20} />
              </button>
            </div>
            
            <div className="modal-body" style={{ color: 'var(--text-secondary)', padding: '16px 0', fontSize: '14px', lineHeight: '1.6' }}>
              <p style={{ margin: '0 0 16px 0' }}>
                {t('sshKeysWizardDesc')}
              </p>

              <div style={{ backgroundColor: 'rgba(0, 0, 0, 0.2)', borderRadius: '12px', padding: '16px', border: '1px solid var(--border-color)', display: 'flex', flexDirection: 'column', gap: '12px' }}>
                <div>
                  <p style={{ fontSize: '13px', margin: '0 0 4px 0' }}>{t('sshKeysStep1')}</p>
                  <pre style={{ backgroundColor: 'var(--bg-panel)', padding: '8px', borderRadius: '6px', border: '1px solid var(--border-color)', color: 'var(--accent-gold)', fontFamily: 'monospace', fontSize: '12px' }}>
                    chmod 600 ~/.ssh/id_* && chmod 644 ~/.ssh/id_*.pub
                  </pre>
                </div>
              </div>
            </div>

            <div className="modal-actions" style={{ borderTop: '1px solid var(--border-color)', paddingTop: '16px', display: 'flex', justifyContent: 'flex-end' }}>
              <button className="btn btn-primary" onClick={() => setShowSSHKeysWizard(false)}>
                {t('btnConfirm')}
              </button>
            </div>
          </div>
        </div>
      )}

      {showVPNWizard && (
        <div className="modal-overlay">
          <div className="modal-content glassmorphism" style={{ maxWidth: '600px', width: '90%' }}>
            <div className="modal-header">
              <h3>{t('vpnWizardTitle')}</h3>
              <button className="close-modal-btn" onClick={() => setShowVPNWizard(false)}>
                <IconX size={20} />
              </button>
            </div>
            
            <div className="modal-body" style={{ color: 'var(--text-secondary)', padding: '16px 0', fontSize: '14px', lineHeight: '1.6' }}>
              <p style={{ margin: '0 0 16px 0' }}>
                {t('vpnWizardDesc')}
              </p>

              <div style={{ backgroundColor: 'rgba(0, 0, 0, 0.2)', borderRadius: '12px', padding: '16px', border: '1px solid var(--border-color)', display: 'flex', flexDirection: 'column', gap: '12px' }}>
                <div>
                  <p style={{ fontSize: '13px', marginBottom: '4px' }}>{t('vpnStep1')}</p>
                  <p style={{ fontSize: '13px', marginBottom: '4px' }}>{t('vpnStep2')}</p>
                  <pre style={{ backgroundColor: 'var(--bg-panel)', padding: '8px', borderRadius: '6px', border: '1px solid var(--border-color)', color: 'var(--accent-gold)', fontFamily: 'monospace', fontSize: '12px' }}>
                    sudo apt install network-manager-gnome network-manager-openvpn
                  </pre>
                </div>
                <div>
                  <p style={{ fontSize: '13px', marginBottom: '4px' }}>{t('vpnStep3')}</p>
                </div>
              </div>
            </div>

            <div className="modal-actions" style={{ borderTop: '1px solid var(--border-color)', paddingTop: '16px', display: 'flex', justifyContent: 'flex-end' }}>
              <button className="btn btn-primary" onClick={() => setShowVPNWizard(false)}>
                {t('btnConfirm')}
              </button>
            </div>
          </div>
        </div>
      )}

      {showAddAppModal && (
        <div className="modal-overlay">
          <div className="modal-content glassmorphism" style={{ maxWidth: '500px', width: '90%' }}>
            <div className="modal-header">
              <h3>{t('addAppModalTitle')}</h3>
              <button className="close-modal-btn" onClick={() => setShowAddAppModal(false)}>
                <IconX size={20} />
              </button>
            </div>
            
            <div className="modal-body" style={{ color: 'var(--text-secondary)', padding: '16px 0', fontSize: '14px', display: 'flex', flexDirection: 'column', gap: '16px' }}>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <label style={{ fontSize: '12px', fontWeight: '600', color: 'var(--text-primary)' }}>{t('appNameLabel')}</label>
                <input 
                  type="text" 
                  placeholder={t('appNamePlaceholder')}
                  value={newAppName}
                  onChange={(e) => setNewAppName(e.target.value)}
                  className="settings-select"
                  style={{ width: '100%', padding: '10px 12px', fontSize: '13px' }}
                />
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <label style={{ fontSize: '12px', fontWeight: '600', color: 'var(--text-primary)' }}>{t('appWebUrlLabel')}</label>
                <input 
                  type="text" 
                  placeholder="https://example.com"
                  value={newAppUrl}
                  onChange={(e) => setNewAppUrl(e.target.value)}
                  className="settings-select"
                  style={{ width: '100%', padding: '10px 12px', fontSize: '13px' }}
                />
              </div>
            </div>

            <div className="modal-actions" style={{ borderTop: '1px solid var(--border-color)', paddingTop: '16px', display: 'flex', justifyContent: 'flex-end', gap: '12px' }}>
              <button className="btn btn-secondary" onClick={() => setShowAddAppModal(false)}>
                {t('btnCancel')}
              </button>
              <button 
                className="btn btn-primary" 
                onClick={handleAddCustomApp}
                disabled={!newAppName.trim() || !newAppUrl.trim()}
              >
                {t('btnSaveInstall')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

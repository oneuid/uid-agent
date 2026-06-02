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
  IconLogout
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

export default function App() {
  const [activeTab, setActiveTab] = useState<'dashboard' | 'tokens' | 'settings'>('dashboard');
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
  const [showLUKSWizard, setShowLUKSWizard] = useState<boolean>(false);
  const [showFirewallWizard, setShowFirewallWizard] = useState<boolean>(false);
  const [showSecureBootWizard, setShowSecureBootWizard] = useState<boolean>(false);
  const [showSSHKeysWizard, setShowSSHKeysWizard] = useState<boolean>(false);
  const [showVPNWizard, setShowVPNWizard] = useState<boolean>(false);
  const [remediatingMap, setRemediatingMap] = useState<Record<string, boolean>>({});



  // Load initial info once on mount
  useEffect(() => {
    const loadInitialData = async () => {
      try {
        const postureData = await invoke<Posture>('get_posture');
        setPosture(postureData);

        const profile = await invoke<UserProfile | null>('get_user_profile');
        setUserProfile(profile);
      } catch (e) {
        console.error('Failed to load initial data:', e);
        setLog(`Failed to load initial data: ${e}`);
      }
    };
    loadInitialData();
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
      alert(t('remediationFailed').replace('{error}', e.toString()));
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
      await invoke('open_browser_url', { url: 'https://uid.one' });
      setLog("Opened login page in browser. Once logged in, your account will automatically sync here.");
    } catch (e: any) {
      setLog(`Failed to open login page: ${e}`);
    }
  };



  const handlePinApp = async (appId: string) => {
    try {
      const res = await invoke<string>('pin_to_dock', { appId });
      setLog(res);
    } catch (e: any) {
      setLog(`Error pinning app to GNOME Dock: ${e}`);
    }
  };

  return (
    <div className="app-container" dir={language === 'ar' ? 'rtl' : 'ltr'}>
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
                        {userProfile.name.charAt(0).toUpperCase()}
                      </div>
                    )}
                  </div>
                  <div className="profile-info">
                    <span className="profile-name" title={userProfile.name}>{userProfile.name}</span>
                    <span className="profile-email" title={userProfile.email}>{userProfile.email}</span>
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
              <span>{t('versionInfo')}</span>
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
                                    alert('Copied certificate serial to clipboard!');
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



              <div className="settings-section" style={{ marginTop: '24px' }}>
                <h3 className="settings-section-title">{t('updateSection')}</h3>
                <p className="settings-section-desc">{t('updateDesc')}</p>
                <div className="update-card-row">
                  <div style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
                    <span>{t('versionInfo')}</span>
                  </div>
                  <button 
                    onClick={async () => {
                      setCheckingUpdate(true);
                      setLog('Checking for software updates...');
                      try {
                        const res = await invoke<string>('check_for_updates');
                        setLog(`${t('updateSuccess')} (Status: ${res})`);
                      } catch (e: any) {
                        setLog(`Update check failed: ${e}`);
                      } finally {
                        setCheckingUpdate(false);
                      }
                    }}
                    className="btn btn-secondary"
                    disabled={checkingUpdate || loading}
                  >
                    {checkingUpdate ? t('btnUpdating') : t('btnCheckUpdate')}
                  </button>
                </div>
              </div>

              {log && (
                <div className="log-console" style={{ marginTop: '16px' }}>
                  <pre style={{ margin: 0, padding: '10px 14px', fontSize: '12px', background: 'rgba(0,0,0,0.2)', border: '1px solid var(--border-color)', borderRadius: '8px', color: 'var(--accent-gold)', fontFamily: 'monospace', overflowX: 'auto' }}>{log}</pre>
                </div>
              )}
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

    </div>
  );
}

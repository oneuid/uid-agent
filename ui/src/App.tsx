import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { 
  IconShieldCheck, 
  IconAlertCircle, 
  IconCpu, 
  IconDeviceUsb, 
  IconKey, 
  IconServer, 
  IconInfoCircle,
  IconPlayerPlay,
  IconPlayerStop,
  IconCloudDownload
} from '@tabler/icons-react';

interface Certificate {
  label?: string;
  id?: string;
  issuer?: string;
  subject?: string;
  valid_from?: string;
  valid_to?: string;
}

interface Posture {
  os_family?: string;
  os_release?: string;
  hostname?: string;
  firewall_status?: string;
  disk_encrypted?: boolean;
  secure_boot?: boolean;
}

export default function App() {
  const [activeTab, setActiveTab] = useState<'dashboard' | 'tokens' | 'apps'>('dashboard');
  const [posture, setPosture] = useState<Posture | null>(null);
  const [certs, setCerts] = useState<Certificate[]>([]);
  const [dockerInstalled, setDockerInstalled] = useState<boolean>(false);
  const [zaloStatus, setZaloStatus] = useState<'not_installed' | 'stopped' | 'running' | 'unknown'>('unknown');
  const [loading, setLoading] = useState<boolean>(false);
  const [log, setLog] = useState<string>('');

  // Fetch initial info
  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, 3000);
    return () => clearInterval(interval);
  }, []);

  const fetchStatus = async () => {
    try {
      const postureData = await invoke<Posture>('get_posture');
      setPosture(postureData);

      const certsData = await invoke<Certificate[]>('get_certificates');
      setCerts(certsData);

      const docker = await invoke<boolean>('check_docker_installed');
      setDockerInstalled(docker);

      if (docker) {
        const appStatus = await invoke<{ status: 'not_installed' | 'stopped' | 'running' | 'unknown' }>('check_app_status', { appId: 'zalo' });
        setZaloStatus(appStatus.status);
      } else {
        setZaloStatus('not_installed');
      }
    } catch (e) {
      console.error('Error fetching state from Tauri backend:', e);
    }
  };

  const handleInstallZalo = async () => {
    setLoading(true);
    setLog('Initializing download and setting up Wine container sandbox (this might take a few minutes)...');
    try {
      const res = await invoke<string>('install_app', { appId: 'zalo' });
      setLog(res);
      fetchStatus();
    } catch (e: any) {
      setLog(`Error during installation: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleLaunchZalo = async () => {
    setLoading(true);
    setLog('Starting Zalo in sandbox...');
    try {
      await invoke('launch_app', { appId: 'zalo' });
      setLog('Zalo launched successfully.');
      fetchStatus();
    } catch (e: any) {
      setLog(`Error launching app: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleStopZalo = async () => {
    setLoading(true);
    setLog('Stopping Zalo container...');
    try {
      await invoke('stop_app', { appId: 'zalo' });
      setLog('Zalo container stopped.');
      fetchStatus();
    } catch (e: any) {
      setLog(`Error stopping app: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app-container">
      {/* Header */}
      <header className="app-header">
        <div className="header-logo">
          <div className="logo-shield">
            <IconShieldCheck size={24} className="gold-icon" />
          </div>
          <div>
            <h1>UID.one</h1>
            <p>Endpoint OS Security Agent</p>
          </div>
        </div>
        <div className="header-status">
          <span className="status-badge online">
            <span className="pulse-dot"></span> Active
          </span>
        </div>
      </header>

      {/* Main Area */}
      <div className="app-main">
        {/* Sidebar */}
        <aside className="app-sidebar">
          <nav className="sidebar-nav">
            <button 
              className={`nav-btn ${activeTab === 'dashboard' ? 'active' : ''}`}
              onClick={() => setActiveTab('dashboard')}
            >
              <IconCpu size={18} />
              <span>Posture Check</span>
            </button>
            <button 
              className={`nav-btn ${activeTab === 'tokens' ? 'active' : ''}`}
              onClick={() => setActiveTab('tokens')}
            >
              <IconDeviceUsb size={18} />
              <span>USB Certificates</span>
            </button>
            <button 
              className={`nav-btn ${activeTab === 'apps' ? 'active' : ''}`}
              onClick={() => setActiveTab('apps')}
            >
              <IconServer size={18} />
              <span>App Sandbox</span>
            </button>
          </nav>

          <div className="sidebar-footer">
            <div className="agent-info">
              <IconInfoCircle size={14} />
              <span>Version 3.0.0 (Linux)</span>
            </div>
          </div>
        </aside>

        {/* Content Panel */}
        <main className="app-content">
          {activeTab === 'dashboard' && (
            <div className="tab-pane">
              <h2 className="section-title">System Posture & SOC 2 Compliance</h2>
              <p className="section-subtitle">Real-time validation of device security controls.</p>

              <div className="posture-grid">
                <div className="posture-card">
                  <div className="card-header">
                    <h3>Operating System</h3>
                    <IconInfoCircle size={18} className="gray-icon" />
                  </div>
                  <div className="card-body">
                    <p className="value">{posture?.os_family || 'Linux'} ({posture?.os_release || 'Unknown'})</p>
                    <p className="label">Hostname: {posture?.hostname || 'localhost'}</p>
                  </div>
                </div>

                <div className="posture-card">
                  <div className="card-header">
                    <h3>Disk Encryption</h3>
                    {posture?.disk_encrypted ? (
                      <IconShieldCheck size={20} className="green-icon" />
                    ) : (
                      <IconAlertCircle size={20} className="red-icon" />
                    )}
                  </div>
                  <div className="card-body">
                    <p className="value">{posture?.disk_encrypted ? 'Encrypted' : 'Not Detected'}</p>
                    <p className="label">Protects local data at rest</p>
                  </div>
                </div>

                <div className="posture-card">
                  <div className="card-header">
                    <h3>System Firewall</h3>
                    {posture?.firewall_status === 'active' ? (
                      <IconShieldCheck size={20} className="green-icon" />
                    ) : (
                      <IconAlertCircle size={20} className="red-icon" />
                    )}
                  </div>
                  <div className="card-body">
                    <p className="value">{posture?.firewall_status === 'active' ? 'Active' : 'Disabled'}</p>
                    <p className="label">Blocks unauthorized network connections</p>
                  </div>
                </div>

                <div className="posture-card">
                  <div className="card-header">
                    <h3>Secure Boot</h3>
                    {posture?.secure_boot ? (
                      <IconShieldCheck size={20} className="green-icon" />
                    ) : (
                      <IconAlertCircle size={20} className="gold-icon" />
                    )}
                  </div>
                  <div className="card-body">
                    <p className="value">{posture?.secure_boot ? 'Enabled' : 'Disabled'}</p>
                    <p className="label">Prevents malicious rootkits on startup</p>
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'tokens' && (
            <div className="tab-pane">
              <h2 className="section-title">Connected Smart Cards & USB Tokens</h2>
              <p className="section-subtitle">Attested PKCS#11 hardware tokens plugged into this machine.</p>

              {certs.length === 0 ? (
                <div className="empty-state">
                  <IconDeviceUsb size={48} className="gray-icon" />
                  <p>No USB security tokens detected.</p>
                  <span>Insert your token to sign tax, customs, or auth payloads.</span>
                </div>
              ) : (
                <div className="cert-list">
                  {certs.map((cert, index) => (
                    <div className="cert-card" key={index}>
                      <div className="cert-card-header">
                        <IconKey size={20} className="gold-icon" />
                        <h3>{cert.label || 'Unnamed USB Token'}</h3>
                      </div>
                      <div className="cert-card-body">
                        <p><strong>Subject:</strong> {cert.subject || 'Unknown'}</p>
                        <p><strong>Issuer:</strong> {cert.issuer || 'Unknown'}</p>
                        <div className="cert-dates">
                          <span><strong>Valid From:</strong> {cert.valid_from || 'N/A'}</span>
                          <span><strong>Valid To:</strong> {cert.valid_to || 'N/A'}</span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {activeTab === 'apps' && (
            <div className="tab-pane">
              <h2 className="section-title">Enterprise Linux Sandbox</h2>
              <p className="section-subtitle">Run Windows business applications (like Zalo) inside a secure containerized sandbox.</p>

              {!dockerInstalled && (
                <div className="warning-banner">
                  <IconAlertCircle size={20} className="red-icon" />
                  <span>Docker daemon is not running. Please install and start Docker to run sandboxed applications.</span>
                </div>
              )}

              <div className="apps-container">
                <div className="app-store-card">
                  <div className="app-card-top">
                    <div className="app-icon zalo-bg">Z</div>
                    <div className="app-details">
                      <h3>Zalo Messenger</h3>
                      <p>Windows Desktop Edition (Wine Sandbox)</p>
                      <div className="app-tags">
                        <span className="app-tag">Secure Folder</span>
                        <span className="app-tag">Wine 9.0</span>
                      </div>
                    </div>
                    <div className="app-action-status">
                      {zaloStatus === 'running' && <span className="status-indicator running">Running</span>}
                      {zaloStatus === 'stopped' && <span className="status-indicator stopped">Stopped</span>}
                      {zaloStatus === 'not_installed' && <span className="status-indicator not-installed">Not Configured</span>}
                    </div>
                  </div>

                  <div className="app-card-controls">
                    {zaloStatus === 'not_installed' && (
                      <button 
                        className="btn btn-primary"
                        onClick={handleInstallZalo}
                        disabled={loading || !dockerInstalled}
                      >
                        <IconCloudDownload size={18} />
                        <span>Install Sandbox App</span>
                      </button>
                    )}

                    {zaloStatus === 'stopped' && (
                      <div className="btn-group">
                        <button 
                          className="btn btn-success"
                          onClick={handleLaunchZalo}
                          disabled={loading}
                        >
                          <IconPlayerPlay size={18} />
                          <span>Launch App</span>
                        </button>
                        <button 
                          className="btn btn-secondary"
                          onClick={handleInstallZalo}
                          disabled={loading}
                        >
                          Reconfigure
                        </button>
                      </div>
                    )}

                    {zaloStatus === 'running' && (
                      <button 
                        className="btn btn-danger"
                        onClick={handleStopZalo}
                        disabled={loading}
                      >
                        <IconPlayerStop size={18} />
                        <span>Stop Sandbox</span>
                      </button>
                    )}
                  </div>
                </div>

                {log && (
                  <div className="log-console">
                    <h4>Operation Console Logs</h4>
                    <pre>{log}</pre>
                  </div>
                )}
              </div>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}

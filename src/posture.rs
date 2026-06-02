use std::fs;
use std::process::Command;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevicePosture {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub disk_encrypted: bool,
    pub firewall_active: bool,
    pub uptime_seconds: u64,
    pub hostname: String,
    pub secure_boot: bool,
    pub screen_lock_active: bool,
    pub ssh_keys_secure: bool,
    pub vpn_active: bool,
}

fn get_hostname() -> String {
    Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "localhost".to_string())
}

pub fn get_posture() -> DevicePosture {
    #[cfg(target_os = "linux")]
    return linux::get_posture();

    #[cfg(target_os = "macos")]
    return macos::get_posture();

    #[cfg(target_os = "windows")]
    return windows::get_posture();

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return DevicePosture {
        os_name: "Unknown".to_string(),
        os_version: "Unknown".to_string(),
        kernel_version: "Unknown".to_string(),
        disk_encrypted: false,
        firewall_active: false,
        uptime_seconds: 0,
        hostname: "localhost".to_string(),
        secure_boot: false,
        screen_lock_active: false,
        ssh_keys_secure: true,
        vpn_active: false,
    };
}

// ─────────────────────────────────────────────────────────────────────────────
// Linux Implementation
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    pub fn get_posture() -> DevicePosture {
        let (os_name, os_version) = get_os_info();
        let kernel_version = get_kernel_version();
        let disk_encrypted = check_disk_encryption();
        let firewall_active = check_firewall_status();
        let uptime_seconds = get_uptime();
        let hostname = get_hostname();
        let secure_boot = check_secure_boot();
        let screen_lock_active = check_screen_lock();
        let ssh_keys_secure = check_ssh_keys_secure();
        let vpn_active = check_vpn_status();

        DevicePosture {
            os_name,
            os_version,
            kernel_version,
            disk_encrypted,
            firewall_active,
            uptime_seconds,
            hostname,
            secure_boot,
            screen_lock_active,
            ssh_keys_secure,
            vpn_active,
        }
    }

    fn get_os_info() -> (String, String) {
        let mut os_name = "Linux".to_string();
        let mut os_version = "Unknown".to_string();
        if let Ok(content) = fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("NAME=") {
                    os_name = line.trim_start_matches("NAME=").trim_matches('"').to_string();
                } else if line.starts_with("VERSION_ID=") {
                    os_version = line.trim_start_matches("VERSION_ID=").trim_matches('"').to_string();
                }
            }
        }
        (os_name, os_version)
    }

    fn get_kernel_version() -> String {
        fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    fn get_uptime() -> u64 {
        if let Ok(content) = fs::read_to_string("/proc/uptime") {
            if let Some(s) = content.split_whitespace().next() {
                if let Ok(v) = s.parse::<f64>() { return v as u64; }
            }
        }
        0
    }

    fn check_disk_encryption() -> bool {
        // Check dm-crypt (LUKS) in /sys/class/block
        if let Ok(entries) = fs::read_dir("/sys/class/block") {
            for entry in entries.flatten() {
                let path = entry.path().join("dm").join("uuid");
                if path.exists() {
                    if let Ok(uuid) = fs::read_to_string(path) {
                        if uuid.contains("CRYPT-LUKS") || uuid.contains("CRYPT-") {
                            return true;
                        }
                    }
                }
            }
        }
        // Fallback: /proc/mounts
        if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
            if mounts.contains("/dev/mapper/") || mounts.contains("gocryptfs") || mounts.contains("ecryptfs") { 
                return true; 
            }
        }
        false
    }

    fn check_firewall_status() -> bool {
        // 1. systemctl check for ufw (works without root)
        if let Ok(out) = Command::new("systemctl").args(["is-active", "ufw"]).output() {
            if String::from_utf8_lossy(&out.stdout).trim() == "active" {
                return true;
            }
        }
        // 2. systemctl check for firewalld (works without root)
        if let Ok(out) = Command::new("systemctl").args(["is-active", "firewalld"]).output() {
            if String::from_utf8_lossy(&out.stdout).trim() == "active" {
                return true;
            }
        }
        // 3. ufw command fallback (requires root)
        if let Ok(out) = Command::new("ufw").arg("status").output() {
            if String::from_utf8_lossy(&out.stdout).contains("Status: active") {
                return true;
            }
        }
        // 4. iptables fallback (requires root)
        if let Ok(out) = Command::new("iptables").args(["-L", "-n"]).output() {
            if String::from_utf8_lossy(&out.stdout).contains("Chain INPUT") {
                return true;
            }
        }
        false
    }

    fn check_secure_boot() -> bool {
        // Try reading efivars secure boot
        if let Ok(data) = fs::read("/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c") {
            if data.len() >= 5 && data[4] == 1 {
                return true;
            }
        }
        // Fallback: check mokutil
        if let Ok(out) = Command::new("mokutil").arg("--sb-state").output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.contains("SecureBoot enabled") {
                return true;
            }
        }
        false
    }

    fn check_screen_lock() -> bool {
        // GNOME desktop setting
        if let Ok(out) = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.screensaver", "lock-enabled"])
            .output() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
            if s == "true" {
                return true;
            }
        }
        // KDE desktop setting fallback
        if let Ok(out) = Command::new("kreadconfig5")
            .args(["--group", "ScreenSaver", "--key", "Lock"])
            .output() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
            if s == "true" || s == "1" {
                return true;
            }
        }
        false
    }

    fn check_ssh_keys_secure() -> bool {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| "/home/s".to_string());
        let ssh_dir = std::path::Path::new(&home).join(".ssh");
        if !ssh_dir.exists() {
            return true;
        }
        if let Ok(entries) = fs::read_dir(ssh_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.ends_with(".pub") || name == "known_hosts" || name == "known_hosts.old" || name == "config" || name == "authorized_keys" {
                            continue;
                        }
                        if let Ok(content) = fs::read_to_string(&path) {
                            if content.contains("BEGIN") && content.contains("PRIVATE KEY") {
                                if let Ok(out) = Command::new("ssh-keygen")
                                    .args(["-y", "-P", "", "-f", path.to_str().unwrap_or("")])
                                    .output() {
                                    if out.status.success() {
                                        return false; 
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn check_vpn_status() -> bool {
        if let Ok(out) = Command::new("nmcli").args(["connection", "show", "--active"]).output() {
            let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if s.contains("vpn") || s.contains("wireguard") || s.contains("tun") {
                return true;
            }
        }
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    let name_lower = name.to_lowercase();
                    if name_lower.starts_with("tun") || name_lower.starts_with("tap") || name_lower.starts_with("wg") || name_lower.contains("vpn") {
                        return true;
                    }
                }
            }
        }
        false
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// macOS Implementation
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    pub fn get_posture() -> DevicePosture {
        let os_name = "macOS".to_string();
        let os_version = get_os_version();
        let kernel_version = get_kernel_version();
        let disk_encrypted = check_filevault();
        let firewall_active = check_firewall();
        let uptime_seconds = get_uptime();
        let hostname = get_hostname();
        let secure_boot = check_secure_boot();

        DevicePosture {
            os_name,
            os_version,
            kernel_version,
            disk_encrypted,
            firewall_active,
            uptime_seconds,
            hostname,
            secure_boot,
            screen_lock_active: true,
            ssh_keys_secure: true,
            vpn_active: false,
        }
    }

    fn get_os_version() -> String {
        Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    fn get_kernel_version() -> String {
        Command::new("uname")
            .arg("-r")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    fn get_uptime() -> u64 {
        if let Ok(out) = Command::new("sysctl").arg("kern.boottime").output() {
            let s = String::from_utf8_lossy(&out.stdout);
            if let Some(pos) = s.find("sec = ") {
                let after = &s[pos + 6..];
                if let Some(comma) = after.find(',') {
                    if let Ok(boot_sec) = after[..comma].trim().parse::<u64>() {
                        use std::time::{SystemTime, UNIX_EPOCH};
                        if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
                            return now.as_secs().saturating_sub(boot_sec);
                        }
                    }
                }
            }
        }
        0
    }

    fn check_filevault() -> bool {
        Command::new("fdesetup")
            .arg("status")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("FileVault is On"))
            .unwrap_or(false)
    }

    fn check_firewall() -> bool {
        Command::new("/usr/libexec/ApplicationFirewall/socketfilterfw")
            .arg("--getglobalstate")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains("enabled"))
            .unwrap_or(false)
    }

    fn check_secure_boot() -> bool {
        if let Ok(out) = Command::new("csrutil").arg("status").output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.contains("enabled") {
                return true;
            }
        }
        false
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Windows Implementation
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    pub fn get_posture() -> DevicePosture {
        let os_name = "Windows".to_string();
        let os_version = get_os_version();
        let kernel_version = get_kernel_version();
        let disk_encrypted = check_bitlocker();
        let firewall_active = check_firewall();
        let uptime_seconds = get_uptime();
        let hostname = get_hostname();
        let secure_boot = check_secure_boot();

        DevicePosture {
            os_name,
            os_version,
            kernel_version,
            disk_encrypted,
            firewall_active,
            uptime_seconds,
            hostname,
            secure_boot,
            screen_lock_active: true,
            ssh_keys_secure: true,
            vpn_active: false,
        }
    }

    fn get_os_version() -> String {
        use winreg::RegKey;
        use winreg::enums::HKEY_LOCAL_MACHINE;
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
            .and_then(|key| {
                let product: String = key.get_value("ProductName")?;
                let display: String = key.get_value("DisplayVersion").unwrap_or_else(|_| "".to_string());
                if !display.is_empty() {
                    Ok(format!("{} ({})", product, display))
                } else {
                    Ok(product)
                }
            })
            .unwrap_or_else(|_| "Windows".to_string())
    }

    fn get_kernel_version() -> String {
        use winreg::RegKey;
        use winreg::enums::HKEY_LOCAL_MACHINE;
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
            .and_then(|key| {
                let build: String = key.get_value("CurrentBuild")?;
                Ok(build)
            })
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    fn get_uptime() -> u64 {
        Command::new("powershell")
            .args(["-NoProfile", "-Command",
                "((Get-Date) - (gcim Win32_OperatingSystem).LastBootUpTime).TotalSeconds"])
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(0.0) as u64
            })
            .unwrap_or(0)
    }

    fn check_bitlocker() -> bool {
        Command::new("powershell")
            .args(["-NoProfile", "-Command",
                "(Get-BitLockerVolume -MountPoint C:).ProtectionStatus"])
            .output()
            .map(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_lowercase();
                s == "on" || s == "1"
            })
            .unwrap_or(false)
    }

    fn check_firewall() -> bool {
        Command::new("powershell")
            .args(["-NoProfile", "-Command",
                "(Get-NetFirewallProfile -Profile Domain,Public,Private | Where-Object { $_.Enabled -eq $true }).Count -gt 0"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "True")
            .unwrap_or(false)
    }

    fn check_secure_boot() -> bool {
        use winreg::RegKey;
        use winreg::enums::HKEY_LOCAL_MACHINE;
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("SYSTEM\\CurrentControlSet\\Control\\SecureBoot\\State")
            .and_then(|key| key.get_value::<u32, _>("UEFISecureBootEnabled"))
            .map(|val| val == 1)
            .unwrap_or(false)
    }
}

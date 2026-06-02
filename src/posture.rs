use std::fs;
use std::process::Command;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DevicePosture {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub disk_encrypted: bool,
    pub firewall_active: bool,
    pub uptime_seconds: u64,
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

        DevicePosture { os_name, os_version, kernel_version, disk_encrypted, firewall_active, uptime_seconds }
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
            if mounts.contains("/dev/mapper/") { return true; }
        }
        false
    }

    fn check_firewall_status() -> bool {
        // ufw
        if let Ok(out) = Command::new("ufw").arg("status").output() {
            if String::from_utf8_lossy(&out.stdout).contains("Status: active") {
                return true;
            }
        }
        // iptables fallback
        if let Ok(out) = Command::new("iptables").args(["-L", "-n"]).output() {
            if String::from_utf8_lossy(&out.stdout).contains("Chain INPUT") {
                return true;
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
        DevicePosture {
            os_name: "macOS".to_string(),
            os_version: get_os_version(),
            kernel_version: get_kernel_version(),
            disk_encrypted: check_filevault(),
            firewall_active: check_firewall(),
            uptime_seconds: get_uptime(),
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
        // sysctl kern.boottime returns struct timeval; parse seconds from output
        if let Ok(out) = Command::new("sysctl").arg("kern.boottime").output() {
            let s = String::from_utf8_lossy(&out.stdout);
            // Format: "kern.boottime: { sec = 1700000000, usec = 0 } ..."
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
        // fdesetup status returns "FileVault is On." or "FileVault is Off."
        Command::new("fdesetup")
            .arg("status")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("FileVault is On"))
            .unwrap_or(false)
    }

    fn check_firewall() -> bool {
        // socketfilterfw --getglobalstate returns "Firewall is enabled."
        Command::new("/usr/libexec/ApplicationFirewall/socketfilterfw")
            .arg("--getglobalstate")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains("enabled"))
            .unwrap_or(false)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Windows Implementation
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    pub fn get_posture() -> DevicePosture {
        DevicePosture {
            os_name: "Windows".to_string(),
            os_version: get_os_version(),
            kernel_version: get_kernel_version(),
            disk_encrypted: check_bitlocker(),
            firewall_active: check_firewall(),
            uptime_seconds: get_uptime(),
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
        // (Get-Date) - (gcim Win32_OperatingSystem).LastBootUpTime | select -ExpandProperty TotalSeconds
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
        // manage-bde -status returns protection status per volume
        Command::new("powershell")
            .args(["-NoProfile", "-Command",
                "(Get-BitLockerVolume -MountPoint C:).ProtectionStatus"])
            .output()
            .map(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_lowercase();
                // ProtectionStatus: 1 = On, 0 = Off
                s == "on" || s == "1"
            })
            .unwrap_or(false)
    }

    fn check_firewall() -> bool {
        // Windows Firewall profile status
        Command::new("powershell")
            .args(["-NoProfile", "-Command",
                "(Get-NetFirewallProfile -Profile Domain,Public,Private | Where-Object { $_.Enabled -eq $true }).Count -gt 0"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "True")
            .unwrap_or(false)
    }
}

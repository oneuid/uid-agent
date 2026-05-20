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
    let (os_name, os_version) = get_os_info();
    let kernel_version = get_kernel_version();
    let disk_encrypted = check_disk_encryption();
    let firewall_active = check_firewall_status();
    let uptime_seconds = get_uptime();

    DevicePosture {
        os_name,
        os_version,
        kernel_version,
        disk_encrypted,
        firewall_active,
        uptime_seconds,
    }
}

fn get_os_info() -> (String, String) {
    let mut os_name = "Linux".to_string();
    let mut os_version = "Unknown".to_string();

    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("NAME=") {
                os_name = line.trim_start_matches("NAME=")
                    .trim_matches('"')
                    .to_string();
            } else if line.starts_with("VERSION_ID=") {
                os_version = line.trim_start_matches("VERSION_ID=")
                    .trim_matches('"')
                    .to_string();
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
        if let Some(uptime_str) = content.split_whitespace().next() {
            if let Ok(uptime) = uptime_str.parse::<f64>() {
                return uptime as u64;
            }
        }
    }
    0
}

fn check_disk_encryption() -> bool {
    // Check for active dm-crypt devices in /sys/class/block
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

    // Fallback: check /proc/mounts for common encrypted flags
    if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
        if mounts.contains("/dev/mapper/") {
            return true;
        }
    }

    false
}

fn check_firewall_status() -> bool {
    // Check if ufw service is active or iptables rules are present
    let ufw_status = Command::new("ufw")
        .arg("status")
        .output();

    if let Ok(output) = ufw_status {
        let status_str = String::from_utf8_lossy(&output.stdout);
        if status_str.contains("Status: active") {
            return true;
        }
    }

    // Alternatively, verify iptables loaded status or check systemctl directly
    let iptables_status = Command::new("iptables")
        .arg("-L")
        .arg("-n")
        .output();

    if let Ok(output) = iptables_status {
        // If iptables rules are successfully listed and contain standard filtering chains, count as active
        let rules_str = String::from_utf8_lossy(&output.stdout);
        if rules_str.contains("Chain INPUT") {
            return true;
        }
    }

    false
}

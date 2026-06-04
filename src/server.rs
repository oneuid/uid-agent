use std::sync::{Arc, Mutex, OnceLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::crypto::AgentKeys;
use serde_json::json;
use std::process::Command;
use std::fs;

fn new_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

static DRIVER_CACHE: OnceLock<Mutex<Option<(String, String, bool)>>> = OnceLock::new();
static USB_SIG_CACHE: OnceLock<Mutex<String>> = OnceLock::new();
static CERTS_CACHE: OnceLock<Mutex<Vec<serde_json::Value>>> = OnceLock::new();
static DRIVER_SIG_CACHE: OnceLock<Mutex<String>> = OnceLock::new();



fn get_cached_driver() -> Option<(String, String, bool)> {
    let cache = DRIVER_CACHE.get_or_init(|| Mutex::new(None));
    let guard = cache.lock().ok()?;
    guard.clone()
}

fn set_cached_driver(driver: String, label: String, login_required: bool) {
    let cache = DRIVER_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = cache.lock() {
        *guard = Some((driver, label, login_required));
    }
}

fn clear_cached_driver() {
    let cache = DRIVER_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = cache.lock() {
        *guard = None;
    }
}

fn get_cached_certs(sig: &str) -> Option<Vec<serde_json::Value>> {
    let sig_cache = USB_SIG_CACHE.get_or_init(|| Mutex::new(String::new()));
    let certs_cache = CERTS_CACHE.get_or_init(|| Mutex::new(Vec::new()));
    
    if let Ok(sig_guard) = sig_cache.lock() {
        if *sig_guard == sig && !sig.is_empty() {
            if let Ok(certs_guard) = certs_cache.lock() {
                return Some(certs_guard.clone());
            }
        }
    }
    None
}

fn set_cached_certs(sig: String, certs: Vec<serde_json::Value>) {
    let sig_cache = USB_SIG_CACHE.get_or_init(|| Mutex::new(String::new()));
    let certs_cache = CERTS_CACHE.get_or_init(|| Mutex::new(Vec::new()));
    
    if let Ok(mut sig_guard) = sig_cache.lock() {
        if let Ok(mut certs_guard) = certs_cache.lock() {
            *sig_guard = sig;
            *certs_guard = certs;
        }
    }
}

fn read_persistent_certs(sig: &str) -> Option<Vec<serde_json::Value>> {
    let base_dir = crate::get_uid_data_dir();
    let path = format!("{}/certs_cache.json", base_dir);
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
            if json_val["usb_signature"].as_str() == Some(sig) {
                if let (Some(drv), Some(lbl), Some(login_req)) = (
                    json_val["active_driver"].as_str(),
                    json_val["active_label"].as_str(),
                    json_val["login_required"].as_bool()
                ) {
                    set_cached_driver(drv.to_string(), lbl.to_string(), login_req);
                    let sig_cache = DRIVER_SIG_CACHE.get_or_init(|| Mutex::new(String::new()));
                    if let Ok(mut sig_guard) = sig_cache.lock() {
                        *sig_guard = sig.to_string();
                    }
                }
                
                if let Some(arr) = json_val["certificates"].as_array() {
                    return Some(arr.clone());
                }
            }
        }
    }
    None
}

fn clean_dn(dn: &str) -> String {
    for pattern in &["CN = ", "CN=", "O = ", "O="] {
        if let Some(pos) = dn.find(pattern) {
            let val_part = &dn[pos + pattern.len()..];
            if let Some(comma_pos) = val_part.find(", ") {
                return val_part[..comma_pos].to_string();
            } else {
                return val_part.to_string();
            }
        }
    }
    dn.to_string()
}

fn parse_cert_info(der_bytes: &[u8]) -> Option<(String, String, String, String, String)> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = new_command("openssl")
        .args([
            "x509",
            "-inform", "der",
            "-noout",
            "-dates",
            "-subject",
            "-issuer",
            "-serial",
            "-nameopt", "oneline,-esc_msb,utf8",
            "-dateopt", "iso_8601"
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(der_bytes);
    }

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut valid_from = String::new();
    let mut valid_to = String::new();
    let mut subject = String::new();
    let mut issuer = String::new();
    let mut serial = String::new();

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("notBefore=") {
            if val.len() >= 10 {
                valid_from = val[..10].to_string();
            } else {
                valid_from = val.to_string();
            }
        } else if let Some(val) = line.strip_prefix("notAfter=") {
            if val.len() >= 10 {
                valid_to = val[..10].to_string();
            } else {
                valid_to = val.to_string();
            }
        } else if let Some(val) = line.strip_prefix("subject=") {
            subject = clean_dn(val);
        } else if let Some(val) = line.strip_prefix("issuer=") {
            issuer = clean_dn(val);
        } else if let Some(val) = line.strip_prefix("serial=") {
            serial = val.to_string();
        }
    }

    if valid_from.is_empty() || valid_to.is_empty() {
        None
    } else {
        Some((valid_from, valid_to, subject, issuer, serial))
    }
}

fn write_persistent_certs(sig: &str, certs: &[serde_json::Value]) {
    let base_dir = crate::get_uid_data_dir();
    let path = format!("{}/certs_cache.json", base_dir);
    
    let mut active_driver = String::new();
    let mut active_label = String::new();
    let mut login_req = false;
    if let Some((drv, lbl, lr)) = get_cached_driver() {
        active_driver = drv;
        active_label = lbl;
        login_req = lr;
    }
    
    let cache_obj = json!({
        "usb_signature": sig,
        "active_driver": active_driver,
        "active_label": active_label,
        "login_required": login_req,
        "certificates": certs
    });
    if let Ok(content) = serde_json::to_string_pretty(&cache_obj) {
        let _ = fs::write(&path, content);
    }
}

fn get_usb_devices_signature() -> String {
    #[cfg(target_os = "windows")]
    {
        let output = new_command("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "[System.Management.ManagementObjectSearcher]::new('SELECT PNPDeviceID FROM Win32_PnPEntity WHERE PNPDeviceID LIKE ''USB%''').Get() | ForEach-Object { $_.PNPDeviceID }"
            ])
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut devices: Vec<String> = stdout.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            devices.sort();
            devices.join(",")
        } else {
            String::new()
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = new_command("lsusb").output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut devices = Vec::new();
            for line in stdout.lines() {
                if let Some(pos) = line.find("ID ") {
                    let id_part = &line[pos + 3..];
                    if id_part.len() >= 9 {
                        devices.push(id_part[..9].to_string());
                    }
                }
            }
            devices.sort();
            devices.join(",")
        } else {
            String::new()
        }
    }
}

fn is_smartcard_reader_present(sig_string: &str) -> bool {
    // Return true if any USB device is connected, or always true as a fallback on Windows
    // to prevent blocking PKCS#11 scanning/Certificate Store listing.
    if sig_string.is_empty() {
        #[cfg(target_os = "windows")]
        return true;
        #[cfg(not(target_os = "windows"))]
        return true; // Scan standard paths anyway to check for active slots/tokens
    }
    true
}

fn get_pkcs11_tool_path(_driver: Option<&str>) -> String {
    #[cfg(target_os = "windows")]
    {
        let paths = if let Some(drv) = _driver {
            let drv_lower = drv.to_lowercase();
            if drv_lower.contains("syswow64") || drv_lower.contains("x86") {
                vec![
                    "C:\\Program Files (x86)\\OpenSC Project\\OpenSC\\bin\\pkcs11-tool.exe",
                    "C:\\Program Files\\OpenSC Project\\OpenSC\\bin\\pkcs11-tool.exe",
                ]
            } else {
                vec![
                    "C:\\Program Files\\OpenSC Project\\OpenSC\\bin\\pkcs11-tool.exe",
                    "C:\\Program Files (x86)\\OpenSC Project\\OpenSC\\bin\\pkcs11-tool.exe",
                ]
            }
        } else {
            vec![
                "C:\\Program Files\\OpenSC Project\\OpenSC\\bin\\pkcs11-tool.exe",
                "C:\\Program Files (x86)\\OpenSC Project\\OpenSC\\bin\\pkcs11-tool.exe",
            ]
        };
        for p in &paths {
            if std::path::Path::new(p).exists() {
                return p.to_string();
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mac_paths = [
            "/opt/homebrew/bin/pkcs11-tool",
            "/usr/local/bin/pkcs11-tool",
            "/usr/bin/pkcs11-tool",
        ];
        for p in &mac_paths {
            if std::path::Path::new(p).exists() {
                return p.to_string();
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let linux_paths = [
            "/usr/bin/pkcs11-tool",
            "/usr/local/bin/pkcs11-tool",
            "/bin/pkcs11-tool",
        ];
        for p in &linux_paths {
            if std::path::Path::new(p).exists() {
                return p.to_string();
            }
        }
    }

    "pkcs11-tool".to_string()
}

fn check_driver_valid(driver: &str) -> Option<(String, bool)> {
    let output = new_command(get_pkcs11_tool_path(Some(driver)))
        .args(["--module", driver, "--list-slots"])
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        if out.status.success() && !stdout.contains("token not recognized") {
            let mut label = None;
            let mut login_required = false;
            
            for line in stdout.lines() {
                let line = line.trim();
                if line.contains("token label") {
                    let lbl = line.split(':').nth(1).unwrap_or("").trim().trim_matches('"').to_string();
                    if !lbl.to_lowercase().contains("not recognized") {
                        label = Some(lbl);
                    }
                } else if line.contains("token flags") {
                    if line.to_lowercase().contains("login required") {
                        login_required = true;
                    }
                }
            }
            if let Some(lbl) = label {
                return Some((lbl, login_required));
            }
        }
    }
    None
}

#[cfg(unix)]
fn get_p11_kit_modules() -> Vec<String> {
    let mut paths = Vec::new();
    if let Ok(output) = std::process::Command::new("p11-kit")
        .arg("list-modules")
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line_trimmed = line.trim();
                if line_trimmed.starts_with("path:") {
                    let path_part = line_trimmed.splitn(2, ':').nth(1).unwrap_or("").trim();
                    if !path_part.is_empty() {
                        paths.push(path_part.to_string());
                    }
                }
            }
        }
    }
    paths
}

fn is_pkcs11_filename(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    
    // Check extension
    #[cfg(target_os = "windows")]
    let has_lib_ext = name_lower.ends_with(".dll");
    #[cfg(target_os = "macos")]
    let has_lib_ext = name_lower.ends_with(".dylib");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let has_lib_ext = name_lower.ends_with(".so") || name_lower.contains(".so.");

    if !has_lib_ext {
        return false;
    }
    
    // Purely generic/algorithmic terms - NO hardcoded vendor names!
    name_lower.contains("pkcs11")
        || name_lower.contains("pkcs")
        || name_lower.contains("token")
        || name_lower.contains("sign")
        || name_lower.contains("prov")
        || name_lower.contains("csp")
        || name_lower.contains("opensc")
        || name_lower.contains("card")
        || name_lower.contains("-ca")
        || name_lower.contains("_ca")
}

fn scan_directory_for_drivers(dir: &std::path::Path, current_depth: usize, max_depth: usize, results: &mut Vec<String>) {
    if current_depth > max_depth {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with('.') || name.starts_with('$') || name == "Windows" || name == "System Volume Information" {
                        continue;
                    }
                }
                scan_directory_for_drivers(&path, current_depth + 1, max_depth, results);
            } else if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if is_pkcs11_filename(file_name) {
                        if let Some(path_str) = path.to_str() {
                            results.push(path_str.to_string());
                        }
                    }
                }
            }
        }
    }
}

fn get_driver_paths() -> Vec<String> {
    let base_dir = crate::get_uid_data_dir();
    
    let mut custom_drivers = Vec::new();
    let config_path = format!("{}/config.json", base_dir);
    if let Ok(config_str) = fs::read_to_string(&config_path) {
        if let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&config_str) {
            if let Some(arr) = config_json["custom_drivers"].as_array() {
                for val in arr {
                    if let Some(path) = val.as_str() {
                        custom_drivers.push(path.to_string());
                    }
                }
            }
        }
    }

    let mut standard = Vec::new();

    // 1. Dynamic integration with p11-kit on Unix (macOS/Linux)
    #[cfg(unix)]
    {
        standard.extend(get_p11_kit_modules());
    }

    // 2. Specific directory scanning for each platform
    #[cfg(target_os = "macos")]
    {
        let search_dirs = [
            "/usr/local/lib",
            "/opt/homebrew/lib",
            "/usr/lib",
            "/Library/OpenSC/lib",
            &base_dir,
        ];
        
        for dir in &search_dirs {
            scan_directory_for_drivers(std::path::Path::new(dir), 1, 1, &mut standard);
        }
        
        // Scan specific pkcs11 directory structures
        let pkcs11_dirs = [
            "/usr/lib/pkcs11",
            "/usr/lib64/pkcs11",
            "/usr/local/lib/pkcs11",
            "/Library/OpenSC/lib/pkcs11",
        ];
        for dir in &pkcs11_dirs {
            scan_directory_for_drivers(std::path::Path::new(dir), 1, 1, &mut standard);
        }

        // App-specific resource bundles on macOS (scan all dylibs in resource bundles)
        if let Ok(entries) = fs::read_dir("/Applications") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.extension().map_or(false, |ext| ext == "app") {
                    let resources_dir = path.join("Contents/Resources");
                    if let Ok(res_entries) = fs::read_dir(resources_dir) {
                        for res_entry in res_entries.flatten() {
                            let res_path = res_entry.path();
                            if res_path.is_file() && res_path.extension().map_or(false, |ext| ext == "dylib") {
                                if let Some(path_str) = res_path.to_str() {
                                    standard.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        standard.push("/usr/local/lib/opensc-pkcs11.dylib".to_string());
        standard.push("/opt/homebrew/lib/opensc-pkcs11.dylib".to_string());
        standard.push("/usr/lib/opensc-pkcs11.dylib".to_string());
        standard.push("/Library/OpenSC/lib/opensc-pkcs11.dylib".to_string());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let search_dirs = [
            "/usr/lib",
            "/usr/lib64",
            "/usr/local/lib",
            "/usr/local/lib64",
            "/lib",
            "/lib64",
            "/usr/lib/x86_64-linux-gnu",
            "/usr/lib/i386-linux-gnu",
            &base_dir,
        ];
        
        for dir in &search_dirs {
            scan_directory_for_drivers(std::path::Path::new(dir), 1, 1, &mut standard);
        }

        // Scan specific pkcs11 directory structures
        let pkcs11_dirs = [
            "/usr/lib/pkcs11",
            "/usr/lib64/pkcs11",
            "/usr/local/lib/pkcs11",
            "/usr/lib/x86_64-linux-gnu/pkcs11",
            "/usr/lib/i386-linux-gnu/pkcs11",
        ];
        for dir in &pkcs11_dirs {
            scan_directory_for_drivers(std::path::Path::new(dir), 1, 1, &mut standard);
        }
        
        standard.push("/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so".to_string());
        standard.push("/usr/lib/opensc-pkcs11.so".to_string());
        standard.push("/usr/lib64/opensc-pkcs11.so".to_string());
        standard.push("/usr/local/lib/opensc-pkcs11.so".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        standard.push(format!("{}\\uid-agent\\viettel-ca_v6.dll", localappdata));
        
        scan_directory_for_drivers(std::path::Path::new("C:\\Windows\\System32"), 1, 1, &mut standard);
        scan_directory_for_drivers(std::path::Path::new("C:\\Windows\\SysWOW64"), 1, 1, &mut standard);
        
        let program_files = [
            "C:\\Program Files",
            "C:\\Program Files (x86)",
        ];
        for pf in &program_files {
            if let Ok(entries) = fs::read_dir(pf) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                            let folder_lower = folder_name.to_lowercase();
                            if folder_lower.contains("ca")
                                || folder_lower.contains("token")
                                || folder_lower.contains("manager")
                                || folder_lower.contains("card")
                                || folder_lower.contains("sign")
                                || folder_lower.contains("security")
                                || folder_lower.contains("key")
                                || folder_lower.contains("opensc")
                            {
                                scan_directory_for_drivers(&path, 1, 2, &mut standard);
                            }
                        }
                    }
                }
            }
        }
        
        standard.push("C:\\Program Files\\OpenSC Project\\OpenSC\\pkcs11\\opensc-pkcs11.dll".to_string());
        standard.push("C:\\Program Files (x86)\\OpenSC Project\\OpenSC\\pkcs11\\opensc-pkcs11.dll".to_string());
    }

    custom_drivers.append(&mut standard);
    
    custom_drivers.sort();
    custom_drivers.dedup();
    
    custom_drivers
}

#[cfg(target_os = "windows")]
fn get_windows_store_certificates() -> Option<Vec<serde_json::Value>> {
    let script = r#"
        $ErrorActionPreference = 'SilentlyContinue'
        $certs = Get-ChildItem -Path Cert:\CurrentUser\My -ErrorAction SilentlyContinue
        $result = @()
        if ($certs) {
            foreach ($cert in $certs) {
                try {
                    $hasKey = $false
                    try {
                        $hasKey = $cert.HasPrivateKey
                    } catch {}
                    if (-not $hasKey) { continue }

                    # Skip self-signed localhost / dev certificates to avoid cluttering with dev certs
                    if ($cert.Subject -eq $cert.Issuer -and ($cert.Subject -like "*localhost*" -or $cert.Subject -like "*127.0.0.1*" -or $cert.Subject -like "*IIS Express*")) {
                        continue
                    }

                    $hex = [System.BitConverter]::ToString($cert.RawData).Replace("-", "").ToLower()
                    $result += [PSCustomObject]@{
                        id = "win_" + $cert.Thumbprint
                        label = $cert.Subject
                        subject = $cert.Subject
                        issuer = $cert.Issuer
                        valid_from = $cert.NotBefore.ToString("yyyy-MM-dd")
                        valid_to = $cert.NotAfter.ToString("yyyy-MM-dd")
                        validTo = $cert.NotAfter.ToString("yyyy-MM-dd")
                        serial = $cert.SerialNumber
                        certData = $hex
                    }
                } catch {}
            }
        }
        Write-Output (@($result) | ConvertTo-Json -Compress)
    "#;

    let output = new_command("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed.is_empty() {
            return Some(Vec::new());
        }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(arr) = parsed.as_array() {
                return Some(arr.clone());
            } else if parsed.is_object() {
                return Some(vec![parsed]);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn sign_with_windows_store(thumbprint: &str, hash_hex: &str) -> Result<String, String> {
    let script = format!(r#"
        $ErrorActionPreference = 'Stop'
        $thumbprint = "{}"
        $hashHex = "{}"
        
        $cert = Get-Item "Cert:\CurrentUser\My\$thumbprint" -ErrorAction Stop
        
        $hashBytes = New-Object byte[] ($hashHex.Length / 2)
        for ($i=0; $i -lt $hashHex.Length; $i+=2) {{
            $hashBytes[$i/2] = [System.Convert]::ToByte($hashHex.Substring($i, 2), 16)
        }}
        
        $privateKey = [System.Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($cert)
        if (-not $privateKey) {{
            throw "Private key not accessible or not RSA"
        }}
        
        $sigBytes = $privateKey.SignHash($hashBytes, [System.Security.Cryptography.HashAlgorithmName]::SHA256, [System.Security.Cryptography.RSASignaturePadding]::Pkcs1)
        $sigHex = [System.BitConverter]::ToString($sigBytes).Replace("-", "").ToLower()
        Write-Output $sigHex
    "#, thumbprint, hash_hex);

    let output = new_command("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|e| format!("Failed to execute powershell: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.trim().to_string())
    }
}

pub async fn start_web_server(keys: Arc<AgentKeys>) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let mut bound_any = false;
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(TcpStream, Arc<AgentKeys>)>(100);

        // Bind to IPv4 loopback
        let addr_v4 = "127.0.0.1:13013";
        let tx_v4 = tx.clone();
        let keys_v4 = keys.clone();
        let mut listener_v4 = None;
        match TcpListener::bind(addr_v4).await {
            Ok(listener) => {
                bound_any = true;
                println!("[uid-agent] Local signing HTTP server listening on http://{}", addr_v4);
                listener_v4 = Some(listener);
            }
            Err(e) => {
                eprintln!("[uid-agent] Failed to bind to IPv4 loopback 127.0.0.1:13013: {:?}", e);
            }
        }

        // Bind to IPv6 loopback
        let addr_v6 = "[::1]:13013";
        let tx_v6 = tx.clone();
        let keys_v6 = keys.clone();
        let mut listener_v6 = None;
        match TcpListener::bind(addr_v6).await {
            Ok(listener) => {
                bound_any = true;
                println!("[uid-agent] Local signing HTTP server listening on http://{}", addr_v6);
                listener_v6 = Some(listener);
            }
            Err(e) => {
                eprintln!("[uid-agent] Note: Could not bind to IPv6 loopback [::1]:13013 (IPv6 might be disabled): {:?}", e);
            }
        }

        if bound_any {
            // Spawn listener accept tasks
            if let Some(listener) = listener_v4 {
                let tx_v4_clone = tx_v4.clone();
                let keys_v4_clone = keys_v4.clone();
                tokio::spawn(async move {
                    loop {
                        if let Ok((stream, _)) = listener.accept().await {
                            let _ = tx_v4_clone.send((stream, keys_v4_clone.clone())).await;
                        }
                    }
                });
            }

            if let Some(listener) = listener_v6 {
                let tx_v6_clone = tx_v6.clone();
                let keys_v6_clone = keys_v6.clone();
                tokio::spawn(async move {
                    loop {
                        if let Ok((stream, _)) = listener.accept().await {
                            let _ = tx_v6_clone.send((stream, keys_v6_clone.clone())).await;
                        }
                    }
                });
            }

            // Handle all incoming connections from either loopback address
            while let Some((stream, keys_clone)) = rx.recv().await {
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, keys_clone).await {
                        eprintln!("[uid-agent] Web server connection error: {:?}", e);
                    }
                });
            }
        }

        // If we reach here, it means either bound_any was false, or rx closed.
        // Wait 5 seconds before attempting to bind again.
        println!("[uid-agent] Web server port 13013 is already in use or unavailable. Retrying in 5 seconds...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

// Scans for active PKCS#11 module and parses the active token label
fn detect_active_driver_and_label() -> Option<(String, String, bool)> {
    let sig = get_usb_devices_signature();
    
    // Check if smartcard/token reader is connected at all
    if !is_smartcard_reader_present(&sig) {
        return None;
    }
    
    let sig_cache = DRIVER_SIG_CACHE.get_or_init(|| Mutex::new(String::new()));
    if let Ok(sig_guard) = sig_cache.lock() {
        if *sig_guard == sig && !sig.is_empty() {
            if let Some(cached) = get_cached_driver() {
                return Some(cached);
            }
        }
    }
    
    // If not cached or signature changed, do the real check
    let mut detected = None;
    if let Some((cached_driver, cached_label, _cached_login_required)) = get_cached_driver() {
        if let Some((_, new_login_required)) = check_driver_valid(&cached_driver) {
            detected = Some((cached_driver, cached_label, new_login_required));
        } else {
            clear_cached_driver();
        }
    }

    if detected.is_none() {
        // Real scan of existing drivers
        let drivers = get_driver_paths();
        let existing_drivers: Vec<String> = drivers.iter()
            .filter(|path| std::path::Path::new(path).exists())
            .cloned()
            .collect();
    
        for driver in existing_drivers {
            if let Some((label, login_required)) = check_driver_valid(&driver) {
                detected = Some((driver, label, login_required));
                break;
            }
        }
    }

    if detected.is_none() {
        // Default Fallback: Check if generic OpenSC is installed
        let drivers = get_driver_paths();
        for driver in drivers {
            if driver.contains("opensc") && std::path::Path::new(&driver).exists() {
                if let Some((label, login_required)) = check_driver_valid(&driver) {
                    detected = Some((driver, label, login_required));
                } else {
                    // Default fallback: assume OpenSC requires login (highly secure by default)
                    detected = Some((driver, "USB Token".to_string(), true));
                }
                break;
            }
        }
    }

    if let Some((ref driver, ref label, login_required)) = detected {
        set_cached_driver(driver.clone(), label.clone(), login_required);
        if let Ok(mut sig_guard) = sig_cache.lock() {
            *sig_guard = sig;
        }
    }

    detected
}

// Scans plugged-in USB Tokens using pkcs11-tool
pub fn get_usb_certificates() -> Vec<serde_json::Value> {
    let sig = get_usb_devices_signature();
    
    // Check if smartcard/token reader is connected at all
    if !is_smartcard_reader_present(&sig) {
        return Vec::new();
    }
    
    if let Some(cached) = get_cached_certs(&sig) {
        return cached;
    }
    
    #[cfg(target_os = "windows")]
    {
        if let Some(win_certs) = get_windows_store_certificates() {
            if !win_certs.is_empty() {
                set_cached_certs(sig.clone(), win_certs.clone());
                return win_certs;
            }
        }
    }
    
    if let Some(cached) = read_persistent_certs(&sig) {
        set_cached_certs(sig.clone(), cached.clone());
        return cached;
    }
    
    let mut certs = Vec::new();
    
    if let Some((driver, label, _login_required)) = detect_active_driver_and_label() {
        // Try listing certificates
        let output = new_command(get_pkcs11_tool_path(Some(&driver)))
            .args(["--module", &driver, "--list-objects", "--type", "cert"])
            .output();
            
        let mut found_certs = false;
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut current_label = None;
            let mut current_id = None;
            
            for line in stdout.lines() {
                let line = line.trim();
                if line.contains("label:") {
                    let lbl = line.split(':').nth(1).unwrap_or("").trim().trim_matches('"').to_string();
                    current_label = Some(lbl);
                } else if line.contains("ID:") {
                    let id = line.split(':').nth(1).unwrap_or("").trim().to_string();
                    current_id = Some(id);
                }
                
                if let (Some(lbl), Some(id)) = (&current_label, &current_id) {
                    let base_dir = crate::get_uid_data_dir();
                    let temp_cert_path = format!("{}/temp_cert_{}.der", base_dir, id);
                    
                    let read_output = new_command(get_pkcs11_tool_path(Some(&driver)))
                        .args([
                            "--module", &driver,
                            "--read-object",
                            "--type", "cert",
                            "--id", id,
                            "--output-file", &temp_cert_path
                        ])
                        .output();
                        
                    let mut cert_data = String::new();
                    let mut valid_from = "2024-01-01".to_string();
                    let mut valid_to = "2029-12-31".to_string();
                    let mut subject_name = lbl.clone();
                    let mut issuer_name = label.clone();
                    let mut serial_num = "N/A".to_string();

                    if let Ok(out) = read_output {
                        if out.status.success() {
                            if let Ok(bytes) = fs::read(&temp_cert_path) {
                                cert_data = hex::encode(&bytes);
                                if let Some((vf, vt, sub, iss, ser)) = parse_cert_info(&bytes) {
                                    valid_from = vf;
                                    valid_to = vt;
                                    subject_name = sub;
                                    issuer_name = iss;
                                    serial_num = ser;
                                }
                            }
                        }
                    }
                    let _ = fs::remove_file(&temp_cert_path);

                    certs.push(json!({
                        "id": format!("usb_{}", id),
                        "label": subject_name.clone(),
                        "subject": subject_name.clone(),
                        "issuer": issuer_name.clone(),
                        "valid_from": valid_from.clone(),
                        "valid_to": valid_to.clone(),
                        "validTo": valid_to.clone(),
                        "serial": serial_num.clone(),
                        "certData": cert_data
                    }));
                    current_label = None;
                    current_id = None;
                    found_certs = true;
                }
            }
        }
        
        // If the card hides certificates before login, or no certs found, return a placeholder using the token label
        if !found_certs {
            certs.push(json!({
                "id": "usb_auto_detected",
                "label": label.clone(),
                "subject": label.clone(),
                "issuer": label.clone(),
                "valid_from": "2024-01-01",
                "valid_to": "2029-12-31",
                "validTo": "2029-12-31"
            }));
        }
    }
    
    set_cached_certs(sig, certs.clone());
    certs
}

async fn handle_connection(mut stream: TcpStream, keys: Arc<AgentKeys>) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 4096];
    
    // Read headers first (until we find "\r\n\r\n")
    loop {
        let n = stream.read(&mut temp).await?;
        if n == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..n]);
        if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buffer.len() > 65536 {
            return Ok(());
        }
    }

    if buffer.is_empty() {
        return Ok(());
    }

    let req_str = String::from_utf8_lossy(&buffer);
    let pos = match req_str.find("\r\n\r\n") {
        Some(p) => p,
        None => return Ok(()),
    };

    let headers_part = &req_str[..pos];
    
    // Parse Content-Length
    let mut content_length = 0;
    for line in headers_part.lines() {
        if line.to_lowercase().starts_with("content-length:") {
            if let Some(val_str) = line.split(':').nth(1) {
                content_length = val_str.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }

    // Read remaining body bytes if needed
    let current_body_len = buffer.len() - (pos + 4);
    if current_body_len < content_length {
        let mut remaining = content_length - current_body_len;
        let mut body_temp = vec![0u8; remaining.min(4096)];
        while remaining > 0 {
            let n = stream.read(&mut body_temp).await?;
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&body_temp[..n]);
            if n >= remaining {
                break;
            }
            remaining -= n;
            if body_temp.len() > remaining {
                body_temp.resize(remaining, 0);
            }
        }
    }

    let req_str = String::from_utf8_lossy(&buffer).into_owned();
    let mut lines = req_str.lines();
    let request_line = match lines.next() {
        Some(line) => line,
        None => return Ok(()),
    };

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    let mut origin = String::new();
    let mut referer = String::new();
    for line in req_str.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.starts_with("origin:") {
            if let Some(val) = line.split_once(':') {
                origin = val.1.trim().to_string();
            }
        } else if line_lower.starts_with("referer:") {
            if let Some(val) = line.split_once(':') {
                referer = val.1.trim().to_string();
            }
        }
    }

    let allow_origin = if !origin.is_empty() {
        origin.clone()
    } else {
        "*".to_string()
    };
    let cors_headers = format!(
        "Access-Control-Allow-Origin: {}\r\n\
         Access-Control-Allow-Credentials: true\r\n\
         Access-Control-Allow-Private-Network: true\r\n",
        allow_origin
    );

    if method == "OPTIONS" {
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Access-Control-Allow-Origin: {}\r\n\
             Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
             Access-Control-Allow-Headers: Content-Type, Accept, Authorization, Access-Control-Allow-Private-Network\r\n\
             Access-Control-Allow-Credentials: true\r\n\
             Access-Control-Allow-Private-Network: true\r\n\
             Content-Length: 0\r\n\
             Connection: close\r\n\r\n",
            allow_origin
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if method == "GET" && path == "/posture" {
        let posture_data = crate::posture::get_posture();
        let body = serde_json::to_string(&posture_data).unwrap_or_default();
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             {}\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            cors_headers,
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if method == "GET" && path == "/drivers" {
        let paths = get_driver_paths();
        let active_info = detect_active_driver_and_label();
        
        let mut drivers_status = Vec::new();
        for p in paths {
            let exists = std::path::Path::new(&p).exists();
            let is_active = if let Some((ref act_p, _, _)) = active_info {
                act_p == &p
            } else {
                false
            };
            
            drivers_status.push(json!({
                "path": p,
                "installed": exists,
                "active": is_active
            }));
        }
        
        let body = json!({
            "active": active_info.map(|(p, l, _)| json!({ "path": p, "label": l })),
            "scanned": drivers_status
        }).to_string();
        
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if method == "POST" && path == "/drivers/register" {
        let body = if let Some(pos) = req_str.find("\r\n\r\n") {
            &req_str[pos + 4..]
        } else {
            ""
        };

        if let Ok(req_json) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(new_path) = req_json["path"].as_str() {
                let base_dir = crate::get_uid_data_dir();
                let config_path = format!("{}/config.json", base_dir);
                
                let mut custom_drivers = Vec::new();
                if let Ok(config_str) = fs::read_to_string(&config_path) {
                    if let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&config_str) {
                        if let Some(arr) = config_json["custom_drivers"].as_array() {
                            for val in arr {
                                if let Some(path) = val.as_str() {
                                    custom_drivers.push(path.to_string());
                                }
                            }
                        }
                    }
                }
                
                if !custom_drivers.contains(&new_path.to_string()) {
                    custom_drivers.push(new_path.to_string());
                    let new_config = json!({
                        "custom_drivers": custom_drivers
                    });
                    
                    let _ = fs::write(&config_path, new_config.to_string());
                    
                    clear_cached_driver();
                }
                
                let res_body = json!({ "success": true }).to_string();
                let response = format!(
                    "HTTP/1.1 200 OK\r\n\
                     Access-Control-Allow-Origin: *\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\
                     Connection: close\r\n\r\n{}",
                    res_body.len(),
                    res_body
                );
                stream.write_all(response.as_bytes()).await?;
                return Ok(());
            }
        }
        
        let res_body = json!({ "success": false, "error": "Invalid payload" }).to_string();
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            res_body.len(),
            res_body
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if method == "POST" && path == "/auth/sync" {
        let body = if let Some(pos) = req_str.find("\r\n\r\n") {
            &req_str[pos + 4..]
        } else {
            ""
        };

        if let Ok(req_json) = serde_json::from_str::<serde_json::Value>(body) {
            let base_dir = crate::get_uid_data_dir();
            let config_path = format!("{}/user.json", base_dir);
            
            let token = req_json["token"].as_str().unwrap_or_default();
            let name = req_json["user"]["name"].as_str().unwrap_or_default();
            let email = req_json["user"]["email"].as_str().unwrap_or_default();
            let avatar = req_json["user"]["avatar"].as_str();

            let user_profile = json!({
                "token": token,
                "name": name,
                "email": email,
                "avatar": avatar
            });

            let _ = fs::write(&config_path, user_profile.to_string());

            let res_body = json!({ "success": true }).to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 {}\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n{}",
                cors_headers,
                res_body.len(),
                res_body
            );
            stream.write_all(response.as_bytes()).await?;
            return Ok(());
        }

        let res_body = json!({ "success": false, "error": "Invalid payload" }).to_string();
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\n\
             {}\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            cors_headers,
            res_body.len(),
            res_body
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if (method == "POST" || method == "GET") && path == "/auth/logout" {
        let base_dir = crate::get_uid_data_dir();
        let path = format!("{}/user.json", base_dir);
        let _ = fs::remove_file(path);

        let res_body = json!({ "success": true }).to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             {}\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            cors_headers,
            res_body.len(),
            res_body
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if method == "GET" && path == "/auth/profile" {
        let base_dir = crate::get_uid_data_dir();
        let path = format!("{}/user.json", base_dir);
        
        let body = if let Ok(content) = fs::read_to_string(path) {
            if let Ok(profile_val) = serde_json::from_str::<serde_json::Value>(&content) {
                json!({ "authenticated": true, "profile": profile_val }).to_string()
            } else {
                json!({ "authenticated": false }).to_string()
            }
        } else {
            json!({ "authenticated": false }).to_string()
        };

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             {}\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            cors_headers,
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if method == "GET" && path == "/clipboard/latest" {
        if let Some(img_bytes) = read_clipboard_image() {
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 {}\
                 Content-Type: image/png\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n",
                cors_headers,
                img_bytes.len()
            );
            let mut full_response = response.into_bytes();
            full_response.extend_from_slice(&img_bytes);
            stream.write_all(&full_response).await?;
            return Ok(());
        }

        let body = json!({ "error": "No image in clipboard" }).to_string();
        let response = format!(
            "HTTP/1.1 204 No Content\r\n\
             {}\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            cors_headers,
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }


    if method == "GET" && (path == "/history" || path == "/signature-history" || path == "/signature_history") {
        let history = read_signature_history();
        let body = json!({
            "code": 0,
            "success": true,
            "history": history
        }).to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if (method == "GET" || method == "POST") && (path == "/certificates" || path == "/certs" || path == "/getCertificates" || path.contains("certificate")) {
        let pubkey_hex = hex::encode(keys.public_key().to_bytes());
        
        // Scan USB Token certificates
        let mut cert_list = get_usb_certificates();
        
        // Always append the local secure agent key
        cert_list.push(json!({
            "id": "agent_identity_key",
            "subject": format!("UID.one Identity Key (Attestation: {})", &pubkey_hex[..12]),
            "issuer": "UID.one Cryptographic Enclave",
            "validTo": "2036-01-01"
        }));
        
        // Format certificates with rich, multi-compatible fields
        let mut formatted_certs = Vec::new();
        for c in &cert_list {
            let id_str = c["id"].as_str().unwrap_or("usb_auto_detected");
            let subject_str = c["subject"].as_str().unwrap_or("");
            let issuer_str = c["issuer"].as_str().unwrap_or("");
            let valid_to_str = c["validTo"].as_str().unwrap_or("2029-12-31");
            let cert_data_str = c["certData"].as_str().unwrap_or("");
            let base64_val = if !cert_data_str.is_empty() {
                hex_to_base64(cert_data_str)
            } else {
                String::new()
            };

            formatted_certs.push(json!({
                "id": id_str,
                "certId": id_str,
                "subject": subject_str,
                "issuer": issuer_str,
                "validTo": valid_to_str,
                "valid_to": valid_to_str,
                "certData": cert_data_str,
                "cert_data": cert_data_str,
                "base64": base64_val,
                "cert": base64_val,
                "value": base64_val
            }));
        }

        let body = json!({
            "certificates": formatted_certs,
            "code": 0,
            "Status": 0,
            "data": formatted_certs,
            "Certificates": formatted_certs,
            "error": "",
            "Message": ""
        }).to_string();

        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if method == "POST" && (path == "/sign" || path == "/signXML" || path == "/signPDF" || path == "/signHash" || path.contains("sign")) {
        let body = if let Some(pos) = req_str.find("\r\n\r\n") {
            &req_str[pos + 4..]
        } else {
            ""
        };

        if let Ok(req_json) = serde_json::from_str::<serde_json::Value>(body) {
            let cert_id = req_json["certId"].as_str()
                .or_else(|| req_json["CertificateId"].as_str())
                .or_else(|| req_json["id"].as_str())
                .unwrap_or("usb_auto_detected");
            let hash_input = req_json["hash"].as_str()
                .or_else(|| req_json["dataToSign"].as_str())
                .or_else(|| req_json["Value"].as_str())
                .or_else(|| req_json["data"].as_str())
                .unwrap_or("");
            let pin = req_json["pin"].as_str()
                .or_else(|| req_json["PIN"].as_str())
                .unwrap_or("");

            let hash_hex = normalize_hash(hash_input);

            println!("[uid-agent] Received sign request for cert_id: '{}', normalized hash: '{}'", cert_id, hash_hex);

            #[cfg(target_os = "windows")]
            {
                let mut is_win_store = cert_id.starts_with("win_");
                let mut win_thumbprint = if is_win_store {
                    cert_id.trim_start_matches("win_").to_string()
                } else {
                    String::new()
                };

                // Fallback: if cert_id is usb_auto_detected/empty, check if we have windows store certs
                if !is_win_store && (cert_id == "usb_auto_detected" || cert_id.is_empty()) {
                    if let Some(win_certs) = get_windows_store_certificates() {
                        if !win_certs.is_empty() {
                            if let Some(id_str) = win_certs[0]["id"].as_str() {
                                is_win_store = true;
                                win_thumbprint = id_str.trim_start_matches("win_").to_string();
                            }
                        }
                    }
                }

                if is_win_store && !win_thumbprint.is_empty() {
                    match sign_with_windows_store(&win_thumbprint, &hash_hex) {
                        Ok(sig_hex) => {
                            // Read certificate data if it's auto_detected/empty for browser state update
                            let mut cert_data = String::new();
                            if cert_id == "usb_auto_detected" || cert_id.is_empty() {
                                if let Some(win_certs) = get_windows_store_certificates() {
                                    for c in win_certs {
                                        if c["id"].as_str().unwrap_or("").contains(&win_thumbprint) {
                                            cert_data = c["certData"].as_str().unwrap_or("").to_string();
                                            break;
                                        }
                                    }
                                }
                            }

                            let response_json = json!({
                                "success": true,
                                "signature": sig_hex,
                                "SignatureValue": sig_hex,
                                "data": sig_hex,
                                "certData": cert_data,
                                "code": 0,
                                "error": ""
                            });
                            let res_body = response_json.to_string();
                            let response = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 {}\
                                 Content-Type: application/json\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\r\n{}",
                                cors_headers,
                                res_body.len(),
                                res_body
                            );
                            stream.write_all(response.as_bytes()).await?;
                            return Ok(());
                        }
                        Err(e) => {
                            let err_json = json!({
                                "success": false,
                                "error": format!("Windows Store signing failed: {}", e),
                                "code": 1
                            });
                            let res_body = err_json.to_string();
                            let response = format!(
                                "HTTP/1.1 400 Bad Request\r\n\
                                 {}\
                                 Content-Type: application/json\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\r\n{}",
                                cors_headers,
                                res_body.len(),
                                res_body
                            );
                            stream.write_all(response.as_bytes()).await?;
                            return Ok(());
                        }
                    }
                }
            }

            if cert_id.starts_with("usb_") || cert_id == "usb_auto_detected" || cert_id.is_empty() {
                let mut resolved_pin = pin.to_string();
                if resolved_pin.is_empty() {
                    let label = if let Some((_, lbl, _)) = detect_active_driver_and_label() {
                        lbl
                    } else {
                        "USB Token".to_string()
                    };
                    if let Some(gui_pin) = prompt_gui_pin(&label) {
                        resolved_pin = gui_pin;
                    } else {
                        let err_json = json!({
                            "success": false,
                            "error": "Signing cancelled by user or PIN prompt failed"
                        });
                        let res_body = err_json.to_string();
                        let response = format!(
                            "HTTP/1.1 400 Bad Request\r\n\
                             Access-Control-Allow-Origin: *\r\n\
                             Content-Type: application/json\r\n\
                             Content-Length: {}\r\n\
                             Connection: close\r\n\r\n{}",
                            res_body.len(),
                            res_body
                        );
                        stream.write_all(response.as_bytes()).await?;
                        return Ok(());
                    }
                }

                // Physical USB Token signing
                if let Some((driver, _label, _login_required)) = detect_active_driver_and_label() {
                    let is_probe = hash_hex == "0000000000000000000000000000000000000000000000000000000000000000" || hash_hex.is_empty();
                    
                    let mut raw_id = if cert_id == "usb_auto_detected" {
                        "01".to_string()
                    } else {
                        cert_id.trim_start_matches("usb_").to_string()
                    };

                    let base_dir = crate::get_uid_data_dir();
                    let dot_uid = base_dir;

                    let mut sig_hex = String::new();
                    let mut cert_data = String::new();
                    let mut success = false;
                    let mut err_msg = String::new();

                    if is_probe {
                        // Probe: just read certificate after logging in (no signing required)
                        let temp_cert_path = format!("{}/temp_cert_{}.der", dot_uid, raw_id);
                        let read_output = new_command(get_pkcs11_tool_path(Some(&driver)))
                            .args([
                                "--module", &driver,
                                "--login",
                                "--pin", &resolved_pin,
                                "--read-object",
                                "--type", "cert",
                                "--id", &raw_id,
                                "--output-file", &temp_cert_path
                            ])
                            .output();
                            
                        let mut read_ok = false;
                        if let Ok(ref out) = read_output {
                            if out.status.success() {
                                if let Ok(bytes) = fs::read(&temp_cert_path) {
                                    cert_data = hex::encode(bytes);
                                    success = true;
                                    read_ok = true;
                                }
                            }
                        }
                        let _ = fs::remove_file(&temp_cert_path);

                        if !read_ok && cert_id == "usb_auto_detected" {
                            // Fallback: list objects to find real ID
                            let list_output = new_command(get_pkcs11_tool_path(Some(&driver)))
                                .args([
                                    "--module", &driver,
                                    "--login",
                                    "--pin", &resolved_pin,
                                    "--list-objects",
                                    "--type", "cert"
                                ])
                                .output();
                                
                            let mut resolved_id = None;
                            if let Ok(out) = list_output {
                                let stdout = String::from_utf8_lossy(&out.stdout);
                                for line in stdout.lines() {
                                    let line = line.trim();
                                    if line.starts_with("ID:") {
                                        resolved_id = Some(line.trim_start_matches("ID:").trim().to_string());
                                        break;
                                    }
                                }
                            }
                            
                            if let Some(rid) = resolved_id {
                                raw_id = rid;
                                let temp_cert_path2 = format!("{}/temp_cert_{}.der", dot_uid, raw_id);
                                let read_output2 = new_command(get_pkcs11_tool_path(Some(&driver)))
                                    .args([
                                        "--module", &driver,
                                        "--login",
                                        "--pin", &resolved_pin,
                                        "--read-object",
                                        "--type", "cert",
                                        "--id", &raw_id,
                                        "--output-file", &temp_cert_path2
                                    ])
                                    .output();
                                    
                                if let Ok(out) = read_output2 {
                                    if out.status.success() {
                                        if let Ok(bytes) = fs::read(&temp_cert_path2) {
                                            cert_data = hex::encode(bytes);
                                            success = true;
                                        } else {
                                            err_msg = "Failed to read fallback certificate file".to_string();
                                        }
                                    } else {
                                        err_msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
                                    }
                                } else {
                                    err_msg = "pkcs11-tool fallback execution failed".to_string();
                                }
                                let _ = fs::remove_file(&temp_cert_path2);
                            } else {
                                if let Ok(out) = read_output {
                                    err_msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
                                } else {
                                    err_msg = "No certificates found on token".to_string();
                                }
                            }
                        }
                    } else {
                        // Real Signing: perform sign, and only read cert if cert_id was auto_detected
                        if let Ok(hash_bytes) = hex::decode(&hash_hex) {
                            let temp_hash_path = format!("{}/temp_hash.bin", dot_uid);
                            let temp_sig_path = format!("{}/temp_sig.bin", dot_uid);
                            
                            let _ = fs::write(&temp_hash_path, &hash_bytes);
                            
                            let sign_output = new_command(get_pkcs11_tool_path(Some(&driver)))
                                .args([
                                    "--module", &driver,
                                    "--login",
                                    "--pin", &resolved_pin,
                                    "--sign",
                                    "--id", &raw_id,
                                    "--mechanism", "RSA-PKCS",
                                    "--input-file", &temp_hash_path,
                                    "--output-file", &temp_sig_path
                                ])
                                .output();
                                
                            let _ = fs::remove_file(&temp_hash_path);
                            
                            match sign_output {
                                Ok(out) if out.status.success() => {
                                    if let Ok(sig_bytes) = fs::read(&temp_sig_path) {
                                        sig_hex = hex::encode(sig_bytes);
                                        success = true;
                                    } else {
                                        err_msg = "Failed to read generated signature file".to_string();
                                    }
                                    let _ = fs::remove_file(&temp_sig_path);
                                    
                                    // Only read cert data if requested as auto-detect (browser needs it to update state)
                                    if cert_id == "usb_auto_detected" {
                                        let temp_cert_path = format!("{}/temp_cert_{}.der", dot_uid, raw_id);
                                        let read_output = new_command(get_pkcs11_tool_path(Some(&driver)))
                                            .args([
                                                "--module", &driver,
                                                "--read-object",
                                                "--type", "cert",
                                                "--id", &raw_id,
                                                "--output-file", &temp_cert_path
                                            ])
                                            .output();
                                            
                                        if let Ok(out) = read_output {
                                            if out.status.success() {
                                                if let Ok(bytes) = fs::read(&temp_cert_path) {
                                                    cert_data = hex::encode(bytes);
                                                }
                                            }
                                        }
                                        let _ = fs::remove_file(&temp_cert_path);
                                    }
                                }
                                Ok(out) => {
                                    err_msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
                                    let _ = fs::remove_file(&temp_sig_path);
                                }
                                Err(e) => {
                                    err_msg = format!("pkcs11-tool execution error: {}", e);
                                    let _ = fs::remove_file(&temp_sig_path);
                                }
                            }
                        } else {
                            err_msg = "Invalid hex hash payload".to_string();
                        }
                    }

                    if success {
                        if !cert_data.is_empty() {
                            let mut valid_from = "2024-01-01".to_string();
                            let mut valid_to = "2029-12-31".to_string();
                            let mut subject_name = _label.clone();
                            let mut issuer_name = _label.clone();
                            let mut serial_num = "N/A".to_string();

                            if let Ok(bytes) = hex::decode(&cert_data) {
                                if let Some((vf, vt, sub, iss, ser)) = parse_cert_info(&bytes) {
                                    valid_from = vf;
                                    valid_to = vt;
                                    subject_name = sub;
                                    issuer_name = iss;
                                    serial_num = ser;
                                }
                            }

                            let sig = get_usb_devices_signature();
                            let cert_list = vec![json!({
                                "id": format!("usb_{}", raw_id),
                                "label": subject_name.clone(),
                                "subject": subject_name.clone(),
                                "issuer": issuer_name.clone(),
                                "valid_from": valid_from.clone(),
                                "valid_to": valid_to.clone(),
                                "validTo": valid_to.clone(),
                                "serial": serial_num.clone(),
                                "certData": cert_data.clone()
                            })];
                            set_cached_certs(sig.clone(), cert_list.clone());
                            write_persistent_certs(&sig, &cert_list);
                        }

                        let base64_sig = if !sig_hex.is_empty() {
                            hex_to_base64(&sig_hex)
                        } else {
                            String::new()
                        };

                        log_signature_to_history(
                            &format!("usb_{}", raw_id),
                            &_label,
                            &hash_hex,
                            "success",
                            &origin,
                            &referer
                        );

                        let res_json = json!({
                            "success": true,
                            "code": 0,
                            "Status": 0,
                            "signature": sig_hex,
                            "Signature": base64_sig,
                            "data": base64_sig,
                            "signature_hex": sig_hex,
                            "signature_base64": base64_sig,
                            "publicKey": "",
                            "certificate": cert_data,
                            "error": "",
                            "Message": ""
                        });
                        let res_body = res_json.to_string();
                        let response = format!(
                            "HTTP/1.1 200 OK\r\n\
                             Access-Control-Allow-Origin: *\r\n\
                             Content-Type: application/json\r\n\
                             Content-Length: {}\r\n\
                             Connection: close\r\n\r\n{}",
                            res_body.len(),
                            res_body
                        );
                        stream.write_all(response.as_bytes()).await?;
                        return Ok(());
                    } else {
                        let err_json = json!({
                            "success": false,
                            "error": format!("USB operation failed: {}", err_msg)
                        });
                        let res_body = err_json.to_string();
                        let response = format!(
                            "HTTP/1.1 400 Bad Request\r\n\
                             Access-Control-Allow-Origin: *\r\n\
                             Content-Type: application/json\r\n\
                             Content-Length: {}\r\n\
                             Connection: close\r\n\r\n{}",
                            res_body.len(),
                            res_body
                        );
                        stream.write_all(response.as_bytes()).await?;
                        return Ok(());
                    }
                } else {
                    let err_json = json!({
                        "success": false,
                        "error": "No active PKCS#11 driver module could be detected for the connected USB Token"
                    });
                    let res_body = err_json.to_string();
                    let response = format!(
                        "HTTP/1.1 400 Bad Request\r\n\
                         Access-Control-Allow-Origin: *\r\n\
                         Content-Type: application/json\r\n\
                         Content-Length: {}\r\n\
                         Connection: close\r\n\r\n{}",
                        res_body.len(),
                        res_body
                    );
                    stream.write_all(response.as_bytes()).await?;
                    return Ok(());
                }
            } else {
                // Local Agent Key signing (default backup) - prompt for user consent
                let user_msg = format!("Do you approve signing request using UID.one Local Agent Key (Hash: {}...)?", &hash_hex[..12]);
                if !prompt_gui_approval(&user_msg) {
                    let err_json = json!({
                        "success": false,
                        "error": "Signing request was denied by user"
                    });
                    let res_body = err_json.to_string();
                    let response = format!(
                        "HTTP/1.1 400 Bad Request\r\n\
                         Access-Control-Allow-Origin: *\r\n\
                         Content-Type: application/json\r\n\
                         Content-Length: {}\r\n\
                         Connection: close\r\n\r\n{}",
                        res_body.len(),
                        res_body
                    );
                    stream.write_all(response.as_bytes()).await?;
                    return Ok(());
                }

                if let Ok(hash_bytes) = hex::decode(&hash_hex) {
                    let signature = keys.sign(&hash_bytes);
                    let sig_hex = hex::encode(signature.to_bytes());
                    let pubkey_hex = hex::encode(keys.public_key().to_bytes());

                    let base64_sig = if !sig_hex.is_empty() {
                        hex_to_base64(&sig_hex)
                    } else {
                        String::new()
                    };

                    log_signature_to_history(
                        "agent_identity_key",
                        "UID.one Local Agent Key",
                        &hash_hex,
                        "success",
                        &origin,
                        &referer
                    );

                    let res_json = json!({
                        "success": true,
                        "code": 0,
                        "Status": 0,
                        "signature": sig_hex,
                        "Signature": base64_sig,
                        "data": base64_sig,
                        "signature_hex": sig_hex,
                        "signature_base64": base64_sig,
                        "publicKey": pubkey_hex,
                        "error": "",
                        "Message": ""
                    });

                    let res_body = res_json.to_string();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\n\
                         Access-Control-Allow-Origin: *\r\n\
                         Content-Type: application/json\r\n\
                         Content-Length: {}\r\n\
                         Connection: close\r\n\r\n{}",
                        res_body.len(),
                        res_body
                    );
                    stream.write_all(response.as_bytes()).await?;
                    return Ok(());
                }
            }
        }

        let err_json = json!({
            "success": false,
            "error": "Invalid signature request payload or hash decoding failed"
        });
        let res_body = err_json.to_string();
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\n\
             Access-Control-Allow-Origin: *\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\r\n{}",
            res_body.len(),
            res_body
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    let response = "HTTP/1.1 404 Not Found\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Content-Length: 0\r\n\
                    Connection: close\r\n\r\n";
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

fn prompt_gui_pin(label: &str) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let output = new_command("zenity")
            .args([
                "--entry",
                "--hide-text",
                "--title=UID.one Token PIN Entry",
                &format!("--text=Please enter the PIN for token: {}", label)
            ])
            .output();
            
        if let Ok(out) = output {
            if out.status.success() {
                let pin = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !pin.is_empty() {
                    return Some(pin);
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display dialog \"Please enter the PIN for token: {}\" default answer \"\" with hidden answer buttons {{\"Cancel\", \"OK\"}} default button \"OK\" with title \"UID.one\"",
            label
        );
        let output = new_command("osascript")
            .args(["-e", &script])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if let Some(pos) = stdout.find("text returned:") {
                    let pin = stdout[pos + 14..].trim().to_string();
                    return Some(pin);
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let ps_code = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             Add-Type -AssemblyName System.Drawing; \
             $form = New-Object System.Windows.Forms.Form; \
             $form.Text = 'UID.one Token PIN Entry'; \
             $form.Size = New-Object System.Drawing.Size(340,190); \
             $form.StartPosition = 'CenterScreen'; \
             $form.FormBorderStyle = 'FixedDialog'; \
             $form.MaximizeBox = $false; \
             $form.MinimizeBox = $false; \
             $form.TopMost = $true; \
             $label = New-Object System.Windows.Forms.Label; \
             $label.Location = New-Object System.Drawing.Point(20,20); \
             $label.Size = New-Object System.Drawing.Size(300,30); \
             $label.Text = 'Please enter the PIN for token: {}'; \
             $textBox = New-Object System.Windows.Forms.TextBox; \
             $textBox.Location = New-Object System.Drawing.Point(20,60); \
             $textBox.Size = New-Object System.Drawing.Size(280,20); \
             $textBox.UseSystemPasswordChar = $true; \
             $button = New-Object System.Windows.Forms.Button; \
             $button.Location = New-Object System.Drawing.Point(110,100); \
             $button.Size = New-Object System.Drawing.Size(100,30); \
             $button.Text = 'OK'; \
             $button.DialogResult = [System.Windows.Forms.DialogResult]::OK; \
             $form.Controls.Add($label); \
             $form.Controls.Add($textBox); \
             $form.Controls.Add($button); \
             $form.AcceptButton = $button; \
             if ($form.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ \
                 Write-Output $textBox.Text \
             }}",
            label.replace("'", "''")
        );
        let output = new_command("powershell")
            .args(["-NoProfile", "-Command", &ps_code])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let pin = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !pin.is_empty() {
                    return Some(pin);
                }
            }
        }
    }
    None
}

fn prompt_gui_approval(message: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        let output = new_command("zenity")
            .args([
                "--question",
                "--title=UID.one Enclave Approval",
                &format!("--text={}", message)
            ])
            .output();
        if let Ok(out) = output {
            return out.status.success();
        }
    }
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display dialog \"{}\" buttons {{\"Deny\", \"Approve\"}} default button \"Approve\" with title \"UID.one\"",
            message
        );
        let output = new_command("osascript")
            .args(["-e", &script])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                return stdout.contains("button returned:Approve");
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        let ps_code = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $result = [System.Windows.Forms.MessageBox]::Show('{}', 'UID.one Enclave Approval', [System.Windows.Forms.MessageBoxButtons]::YesNo, [System.Windows.Forms.MessageBoxIcon]::Question, [System.Windows.Forms.MessageBoxDefaultButton]::Button1, [System.Windows.Forms.MessageBoxOptions]::DefaultDesktopOnly); \
             if ($result -eq [System.Windows.Forms.DialogResult]::Yes) {{ \
                 Write-Output 'Approve' \
             }}",
            message.replace("'", "''")
        );
        let output = new_command("powershell")
            .args(["-NoProfile", "-Command", &ps_code])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                return stdout.trim() == "Approve";
            }
        }
    }
    true
}

fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bytes = Vec::new();
    let mut buffer = 0u32;
    let mut count = 0;
    
    for c in input.chars() {
        if c.is_whitespace() || c == '=' {
            continue;
        }
        let val = match CHARSET.iter().position(|&x| x as char == c) {
            Some(p) => p as u32,
            None => return Err("Invalid character in base64"),
        };
        buffer = (buffer << 6) | val;
        count += 1;
        if count == 4 {
            bytes.push((buffer >> 16) as u8);
            bytes.push((buffer >> 8) as u8);
            bytes.push(buffer as u8);
            count = 0;
            buffer = 0;
        }
    }
    
    if count == 2 {
        bytes.push((buffer >> 4) as u8);
    } else if count == 3 {
        bytes.push((buffer >> 10) as u8);
        bytes.push((buffer >> 2) as u8);
    }
    
    Ok(bytes)
}

fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut i = 0;
    while i < input.len() {
        let chunk = &input[i..std::cmp::min(i + 3, input.len())];
        let mut val = 0u32;
        for (idx, &byte) in chunk.iter().enumerate() {
            val |= (byte as u32) << (16 - idx * 8);
        }
        let chars_to_write = match chunk.len() {
            1 => 2,
            2 => 3,
            _ => 4,
        };
        for idx in 0..chars_to_write {
            let char_idx = ((val >> (18 - idx * 6)) & 0x3F) as usize;
            result.push(CHARSET[char_idx] as char);
        }
        for _ in chars_to_write..4 {
            result.push('=');
        }
        i += 3;
    }
    result
}

fn hex_to_base64(hex_str: &str) -> String {
    if let Ok(bytes) = hex::decode(hex_str) {
        base64_encode(&bytes)
    } else {
        String::new()
    }
}

fn normalize_hash(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.len() == 64 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return trimmed.to_lowercase();
    }
    if let Ok(decoded) = base64_decode(trimmed) {
        if decoded.len() == 32 {
            return hex::encode(decoded);
        }
    }
    trimmed.to_string()
}

fn read_signature_history() -> Vec<serde_json::Value> {
    let base_dir = crate::get_uid_data_dir();
    let path = format!("{}/signature_history.json", base_dir);
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(arr) = json_val.as_array() {
                return arr.clone();
            }
        }
    }
    Vec::new()
}

fn log_signature_to_history(
    cert_id: &str,
    subject: &str,
    hash: &str,
    status: &str,
    origin: &str,
    referer: &str
) {
    let base_dir = crate::get_uid_data_dir();
    let path = format!("{}/signature_history.json", base_dir);

    let mut history = read_signature_history();
    
    let timestamp = if let Ok(time) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        let seconds = time.as_secs();
        format_timestamp_iso8601(seconds)
    } else {
        "2026-06-02T11:44:27Z".to_string()
    };

    let entry = json!({
        "timestamp": timestamp,
        "cert_id": cert_id,
        "subject": subject,
        "hash": hash,
        "status": status,
        "origin": origin,
        "referer": referer
    });

    history.insert(0, entry);

    if history.len() > 100 {
        history.truncate(100);
    }

    if let Ok(serialized) = serde_json::to_string_pretty(&history) {
        let _ = fs::write(&path, serialized);
    }
}

fn format_timestamp_iso8601(epoch_secs: u64) -> String {
    let secs_per_day = 86400;
    let days = epoch_secs / secs_per_day;
    let secs_in_day = epoch_secs % secs_per_day;

    let hour = secs_in_day / 3600;
    let minute = (secs_in_day % 3600) / 60;
    let second = secs_in_day % 60;

    let mut year = 1970;
    let mut days_left = days;

    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if is_leap { 366 } else { 365 };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        year += 1;
    }

    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_days = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &d in &month_days {
        if days_left < d {
            break;
        }
        days_left -= d;
        month += 1;
    }

    let day = days_left + 1;

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hour, minute, second)
}

#[allow(dead_code)]
fn decode_hex(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

fn read_clipboard_image() -> Option<Vec<u8>> {
    #[cfg(target_os = "linux")]
    {
        // Try wl-paste
        if let Ok(output) = new_command("wl-paste")
            .args(["-t", "image/png"])
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                return Some(output.stdout);
            }
        }
        // Try xclip
        if let Ok(output) = new_command("xclip")
            .args(["-selection", "clipboard", "-t", "image/png", "-o"])
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                return Some(output.stdout);
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        // PowerShell command to export clipboard image to stdout
        let ps_cmd = "[void] [System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms'); \
                      if ([System.Windows.Forms.Clipboard]::ContainsImage()) { \
                          $ms = New-Object System.IO.MemoryStream; \
                          [System.Windows.Forms.Clipboard]::GetImage().Save($ms, [System.Drawing.Imaging.ImageFormat]::Png); \
                          [System.Console]::OpenStandardOutput().Write($ms.ToArray(), 0, $ms.Length); \
                      }";
        if let Ok(output) = new_command("powershell")
            .args(["-NoProfile", "-Command", ps_cmd])
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                return Some(output.stdout);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let script = "get the clipboard as «class PNGf»";
        if let Ok(output) = new_command("osascript")
            .args(["-e", script])
            .output()
        {
            if output.status.success() {
                let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if stdout_str.starts_with("«data PNGf") && stdout_str.ends_with("»") {
                    let hex_content = stdout_str
                        .trim_start_matches("«data PNGf")
                        .trim_end_matches('»');
                    if let Ok(bytes) = decode_hex(hex_content) {
                        return Some(bytes);
                    }
                }
            }
        }
    }
    None
}



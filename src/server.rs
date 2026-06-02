use std::sync::{Arc, Mutex, OnceLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::crypto::AgentKeys;
use serde_json::json;
use std::process::Command;
use std::fs;

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
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
    let path = format!("{}/.uid/certs_cache.json", home);
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

fn write_persistent_certs(sig: &str, certs: &[serde_json::Value]) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
    let dot_uid = format!("{}/.uid", home);
    let _ = fs::create_dir_all(&dot_uid);
    let path = format!("{}/certs_cache.json", dot_uid);
    
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
    let output = Command::new("lsusb").output();
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

fn is_smartcard_reader_present(sig_string: &str) -> bool {
    let stdout = if sig_string.is_empty() {
        let output = Command::new("lsusb").output();
        if let Ok(out) = output {
            String::from_utf8_lossy(&out.stdout).to_string()
        } else {
            return true;
        }
    } else {
        let output = Command::new("lsusb").output();
        if let Ok(out) = output {
            String::from_utf8_lossy(&out.stdout).to_string()
        } else {
            sig_string.to_string()
        }
    };

    let stdout_lower = stdout.to_lowercase();
    let keywords = [
        "token", "smartcard", "smart card", "reader", "epass", "feitian",
        "gemalto", "safenet", "yubikey", "ccid", "pkcs", "cherry",
        "omnikey", "identive", "etoken", "vsign", "viettel", "vnpt",
        "fpt", "bkav", "misa"
    ];
    for kw in &keywords {
        if stdout_lower.contains(kw) {
            return true;
        }
    }
    false
}

fn check_driver_valid(driver: &str) -> Option<(String, bool)> {
    let output = Command::new("pkcs11-tool")
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

fn get_driver_paths() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
    
    let mut custom_drivers = Vec::new();
    let config_path = format!("{}/.uid/config.json", home);
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

    #[cfg(unix)]
    {
        let mut standard = vec![
            format!("{}/.uid/viettel-ca_v6.so", home),
            "/usr/lib/libvsignspkcs11.so".to_string(),
            "/usr/lib/libviettel-ca_v2.so".to_string(),
            "/usr/lib/libvsignspkcs11.so.1".to_string(),
            "/usr/lib64/libvsignspkcs11.so".to_string(),
            "/usr/lib/libvnpt-pkcs11.so".to_string(),
            "/usr/lib/libnpki-pkcs11.so".to_string(),
            "/usr/lib/libvnpt-pkcs11.so.1".to_string(),
            "/usr/lib/libcastle.so".to_string(),
            "/usr/lib/libfpt-pkcs11.so".to_string(),
            "/usr/lib64/libfpt-pkcs11.so".to_string(),
            "/usr/lib/libbkav-pkcs11.so".to_string(),
            "/usr/lib64/libbkav-pkcs11.so".to_string(),
            "/usr/lib/libmisa-pkcs11.so".to_string(),
            "/usr/lib/libvina-pkcs11.so".to_string(),
            "/usr/lib/libsafe-pkcs11.so".to_string(),
            "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so".to_string(),
            "/usr/lib/opensc-pkcs11.so".to_string(),
            "/usr/lib64/opensc-pkcs11.so".to_string(),
        ];
        custom_drivers.append(&mut standard);
        custom_drivers
    }
    
    #[cfg(not(unix))]
    {
        let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let mut standard = vec![
            format!("{}\\uid-agent\\viettel-ca_v6.dll", localappdata),
            "C:\\Windows\\System32\\vsignspkcs11.dll".to_string(),
            "C:\\Windows\\System32\\viettel-ca_v2.dll".to_string(),
            "C:\\Windows\\System32\\vnpt-pkcs11.dll".to_string(),
            "C:\\Windows\\System32\\npki-pkcs11.dll".to_string(),
            "C:\\Windows\\System32\\fpt-pkcs11.dll".to_string(),
            "C:\\Windows\\System32\\bkav-pkcs11.dll".to_string(),
            "C:\\Windows\\System32\\vina-pkcs11.dll".to_string(),
            "C:\\Windows\\System32\\misa-pkcs11.dll".to_string(),
            "C:\\Program Files\\OpenSC Project\\OpenSC\\pkcs11\\opensc-pkcs11.dll".to_string(),
            "C:\\Program Files (x86)\\OpenSC Project\\OpenSC\\pkcs11\\opensc-pkcs11.dll".to_string(),
        ];
        custom_drivers.append(&mut standard);
        custom_drivers
    }
}

pub async fn start_web_server(keys: Arc<AgentKeys>) -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:13013";
    let listener = TcpListener::bind(addr).await?;
    println!("[uid-agent] Local signing HTTP server listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let keys_clone = keys.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, keys_clone).await {
                eprintln!("[uid-agent] Web server connection error: {:?}", e);
            }
        });
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
    
        if !existing_drivers.is_empty() {
            let mut handles = Vec::new();
            for driver in existing_drivers {
                let handle = std::thread::spawn(move || {
                    if let Some((label, login_required)) = check_driver_valid(&driver) {
                        Some((driver, label, login_required))
                    } else {
                        None
                    }
                });
                handles.push(handle);
            }
        
            for handle in handles {
                if let Ok(Some(result)) = handle.join() {
                    if detected.is_none() {
                        detected = Some(result);
                    }
                }
            }
        }
    }

    if detected.is_none() {
        // Default Fallback: Check if generic OpenSC is installed
        let drivers = get_driver_paths();
        for driver in drivers {
            if driver.contains("opensc") && std::path::Path::new(&driver).exists() {
                detected = Some((driver, "USB Token".to_string(), false));
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
fn get_usb_certificates() -> Vec<serde_json::Value> {
    let sig = get_usb_devices_signature();
    
    // Check if smartcard/token reader is connected at all
    if !is_smartcard_reader_present(&sig) {
        return Vec::new();
    }
    
    if let Some(cached) = get_cached_certs(&sig) {
        return cached;
    }
    
    if let Some(cached) = read_persistent_certs(&sig) {
        set_cached_certs(sig.clone(), cached.clone());
        return cached;
    }
    
    let mut certs = Vec::new();
    
    if let Some((driver, label, login_required)) = detect_active_driver_and_label() {
        if login_required {
            // For tokens requiring login, return placeholder to avoid slow objects listing before login
            certs.push(json!({
                "id": "usb_auto_detected",
                "subject": label.clone(),
                "issuer": label.clone(),
                "validTo": "2029-12-31"
            }));
        } else {
            // Try listing certificates
            let output = Command::new("pkcs11-tool")
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
                        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
                        let temp_cert_path = format!("{}/.uid/temp_cert_{}.der", home, id);
                        
                        let read_output = Command::new("pkcs11-tool")
                            .args([
                                "--module", &driver,
                                "--read-object",
                                "--type", "cert",
                                "--id", id,
                                "--output-file", &temp_cert_path
                            ])
                            .output();
                            
                        let mut cert_data = String::new();
                        if let Ok(out) = read_output {
                            if out.status.success() {
                                if let Ok(bytes) = fs::read(&temp_cert_path) {
                                    cert_data = hex::encode(bytes);
                                }
                            }
                        }
                        let _ = fs::remove_file(&temp_cert_path);
    
                        certs.push(json!({
                            "id": format!("usb_{}", id),
                            "subject": lbl.clone(),
                            "issuer": label.clone(),
                            "validTo": "2029-12-31",
                            "certData": cert_data
                        }));
                        current_label = None;
                        current_id = None;
                        found_certs = true;
                    }
                }
            }
            
            // If the card hides certificates before login, return a placeholder using the token label
            if !found_certs {
                certs.push(json!({
                    "id": "usb_auto_detected",
                    "subject": label.clone(),
                    "issuer": label.clone(),
                    "validTo": "2029-12-31"
                }));
            }
        }
    }
    
    set_cached_certs(sig, certs.clone());
    certs
}

async fn handle_connection(mut stream: TcpStream, keys: Arc<AgentKeys>) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![0u8; 8192];
    let n = stream.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }

    let req_str = String::from_utf8_lossy(&buffer[..n]);
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

    if method == "OPTIONS" {
        let response = "HTTP/1.1 200 OK\r\n\
                        Access-Control-Allow-Origin: *\r\n\
                        Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                        Access-Control-Allow-Headers: Content-Type, Accept, Authorization\r\n\
                        Content-Length: 0\r\n\
                        Connection: close\r\n\r\n";
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }

    if method == "GET" && path == "/posture" {
        let posture_data = crate::posture::get_posture();
        let body = serde_json::to_string(&posture_data).unwrap_or_default();
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
                let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
                let config_path = format!("{}/.uid/config.json", home);
                
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
                    
                    let dot_uid = format!("{}/.uid", home);
                    let _ = fs::create_dir_all(&dot_uid);
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

    if method == "GET" && path == "/certificates" {
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
        
        let certs = json!({
            "certificates": cert_list
        });

        let body = certs.to_string();
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

    if method == "POST" && path == "/sign" {
        let body = if let Some(pos) = req_str.find("\r\n\r\n") {
            &req_str[pos + 4..]
        } else {
            ""
        };

        if let Ok(req_json) = serde_json::from_str::<serde_json::Value>(body) {
            let cert_id = req_json["certId"].as_str().unwrap_or("");
            let hash_hex = req_json["hash"].as_str().unwrap_or("");
            let pin = req_json["pin"].as_str().unwrap_or("");

            println!("[uid-agent] Received sign request for cert_id: '{}', hash: '{}'", cert_id, hash_hex);

            if cert_id.starts_with("usb_") || cert_id == "usb_auto_detected" {
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

                    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
                    let dot_uid = format!("{}/.uid", home);
                    let _ = fs::create_dir_all(&dot_uid);

                    let mut sig_hex = String::new();
                    let mut cert_data = String::new();
                    let mut success = false;
                    let mut err_msg = String::new();

                    if is_probe {
                        // Probe: just read certificate after logging in (no signing required)
                        let temp_cert_path = format!("{}/temp_cert_{}.der", dot_uid, raw_id);
                        let read_output = Command::new("pkcs11-tool")
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
                            let list_output = Command::new("pkcs11-tool")
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
                                let read_output2 = Command::new("pkcs11-tool")
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
                        if let Ok(hash_bytes) = hex::decode(hash_hex) {
                            let temp_hash_path = format!("{}/temp_hash.bin", dot_uid);
                            let temp_sig_path = format!("{}/temp_sig.bin", dot_uid);
                            
                            let _ = fs::write(&temp_hash_path, &hash_bytes);
                            
                            let sign_output = Command::new("pkcs11-tool")
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
                                        let read_output = Command::new("pkcs11-tool")
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
                            let sig = get_usb_devices_signature();
                            let cert_list = vec![json!({
                                "id": format!("usb_{}", raw_id),
                                "subject": _label.clone(),
                                "issuer": _label.clone(),
                                "validTo": "2029-12-31",
                                "certData": cert_data.clone()
                            })];
                            set_cached_certs(sig.clone(), cert_list.clone());
                            write_persistent_certs(&sig, &cert_list);
                        }

                        let res_json = json!({
                            "success": true,
                            "signature": sig_hex,
                            "publicKey": "",
                            "certificate": cert_data
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

                if let Ok(hash_bytes) = hex::decode(hash_hex) {
                    let signature = keys.sign(&hash_bytes);
                    let sig_hex = hex::encode(signature.to_bytes());
                    let pubkey_hex = hex::encode(keys.public_key().to_bytes());

                    let res_json = json!({
                        "success": true,
                        "signature": sig_hex,
                        "publicKey": pubkey_hex
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
        let output = Command::new("zenity")
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
        let output = Command::new("osascript")
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
    None
}

fn prompt_gui_approval(message: &str) -> bool {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("zenity")
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
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                return stdout.contains("button returned:Approve");
            }
        }
    }
    true
}

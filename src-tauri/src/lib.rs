use std::sync::Arc;
use std::process::Command;
use serde_json::json;

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
use tauri::{Manager, WindowEvent};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

// Tauri Commands

// Tauri Commands


#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct UserProfile {
    token: String,
    name: String,
    email: String,
    avatar: Option<String>,
}

fn get_home_dir() -> String {
    if cfg!(target_os = "windows") {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return profile;
        }
        let drive = std::env::var("HOMEDRIVE").unwrap_or_else(|_| "C:".to_string());
        if let Ok(path) = std::env::var("HOMEPATH") {
            return format!("{}{}", drive, path);
        }
        "C:\\".to_string()
    } else {
        if let Ok(home) = std::env::var("HOME") {
            return home;
        }
        if let Ok(user) = std::env::var("USER") {
            let path = format!("/home/{}", user);
            if std::path::Path::new(&path).exists() {
                return path;
            }
        }
        "/home/s".to_string()
    }
}

#[tauri::command]
async fn get_user_profile() -> Option<UserProfile> {
    let base_dir = uid_agent::get_uid_data_dir();
    let path = format!("{}/user.json", base_dir);
    if let Ok(content) = std::fs::read_to_string(path) {
        serde_json::from_str::<UserProfile>(&content).ok()
    } else {
        None
    }
}

#[tauri::command]
async fn logout_user() -> Result<(), String> {
    let base_dir = uid_agent::get_uid_data_dir();
    let path = format!("{}/user.json", base_dir);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[tauri::command]
async fn open_browser_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = new_command("cmd").args(["/C", "start", "", &url]).status();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = new_command("open").arg(&url).status();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = new_command("xdg-open").arg(&url).status();
    }
    Ok(())
}

#[tauri::command]
async fn show_notification(title: String, body: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let powershell_code = format!(
            "[void] [System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms'); \
             $notification = New-Object System.Windows.Forms.NotifyIcon; \
             $notification.Icon = [System.Drawing.SystemIcons]::Information; \
             $notification.BalloonTipTitle = '{}'; \
             $notification.BalloonTipText = '{}'; \
             $notification.Visible = $true; \
             $notification.ShowBalloonTip(5000);",
            title.replace("'", "''"),
            body.replace("'", "''")
        );
        let _ = new_command("powershell")
            .args(["-Command", &powershell_code])
            .status();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = new_command("osascript")
            .args(["-e", &format!("display notification \"{}\" with title \"{}\"", body.replace("\"", "\\\""), title.replace("\"", "\\\""))])
            .status();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = new_command("notify-send")
            .args([&title, &body])
            .status();
    }
    Ok(())
}


#[tauri::command]
async fn launch_sandbox_app(app_id: String, app_name: String, url: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let window_id = format!("sandbox_{}", app_id);
    let title = format!("UID Sandbox - {} (Isolated Data)", app_name);
    
    // Check if the window is already open. If so, focus it.
    if let Some(existing_window) = app_handle.get_webview_window(&window_id) {
        let _ = existing_window.show();
        let _ = existing_window.set_focus();
        return Ok(());
    }

    // Determine isolated path
    let local_data = get_agent_data_dir(&app_handle);
    let _ = std::fs::create_dir_all(&local_data);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&local_data) {
            let mut perms = meta.permissions();
            perms.set_mode(0o700);
            let _ = std::fs::set_permissions(&local_data, perms);
        }
    }

    let isolated_dir = local_data.join("apps").join(&app_id);
    let _ = std::fs::create_dir_all(&isolated_dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&isolated_dir) {
            let mut perms = meta.permissions();
            perms.set_mode(0o700);
            let _ = std::fs::set_permissions(&isolated_dir, perms);
        }
    }

    // Create a new window showing the target service URL
    let mut window_builder = tauri::WebviewWindowBuilder::new(
        &app_handle,
        &window_id,
        tauri::WebviewUrl::External(url.parse().map_err(|e| format!("Invalid URL: {}", e))?)
    )
    .title(&title)
    .inner_size(1024.0, 768.0)
    .resizable(true)
    .devtools(true)
    .initialization_script(r#"
        (function() {
            window.__tauriPasteImage = function(base64Data) {
                fetch(base64Data)
                    .then(res => res.blob())
                    .then(blob => {
                        const file = new File([blob], 'screenshot.png', { type: 'image/png' });
                        const dataTransfer = new DataTransfer();
                        dataTransfer.items.add(file);
                        const customEvent = new ClipboardEvent('paste', {
                            bubbles: true,
                            cancelable: true
                        });
                        Object.defineProperty(customEvent, 'clipboardData', {
                            value: dataTransfer,
                            writable: false
                        });
                        customEvent.__isCustomPaste = true;
                        const target = document.activeElement || document.body;
                        target.dispatchEvent(customEvent);
                    });
            };

            window.addEventListener('paste', function(e) {
                if (e.__isCustomPaste) return;
                if (e.clipboardData && (e.clipboardData.getData('text') || e.clipboardData.getData('text/html'))) {
                    return;
                }
                if (e.clipboardData && e.clipboardData.files && e.clipboardData.files.length > 0) {
                    return;
                }
                e.preventDefault();
                e.stopImmediatePropagation();
                
                // Trigger custom navigation protocol to communicate with Rust
                window.location.href = "uid-paste://trigger?" + Date.now();
            }, true);

            window.addEventListener('keydown', function(e) {
                if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'v') {
                    const target = document.activeElement;
                    if (target && (
                        target.tagName === 'INPUT' ||
                        target.tagName === 'TEXTAREA' ||
                        target.contentEditable === 'true' ||
                        target.getAttribute('contenteditable') === 'true'
                    )) {
                        // Trigger custom scheme to let Rust check clipboard image
                        window.location.href = "uid-paste://trigger?" + Date.now();
                    }
                }
            }, true);
        })();
    "#);


    let app_handle_clone = app_handle.clone();
    let window_id_clone = window_id.clone();
    window_builder = window_builder.on_navigation(move |url| {
        if url.scheme() == "uid-paste" {
            let app_handle = app_handle_clone.clone();
            let window_id = window_id_clone.clone();
            tauri::async_runtime::spawn(async move {
                if let Some(win) = app_handle.get_webview_window(&window_id) {
                    if let Some(img_bytes) = read_clipboard_image_bytes_helper().await {
                        use base64::{Engine as _, engine::general_purpose};
                        let base64_str = general_purpose::STANDARD.encode(&img_bytes);
                        let js = format!(
                            "if (window.__tauriPasteImage) {{ window.__tauriPasteImage('data:image/png;base64,{}'); }}",
                            base64_str
                        );
                        let _ = win.eval(&js);
                    }
                }
            });
            return false;
        }
        true
    });

    #[cfg(not(target_os = "macos"))]
    {
        window_builder = window_builder.data_directory(isolated_dir);
    }
    #[cfg(target_os = "macos")]
    {
        let mut key = [0u8; 16];
        let bytes = app_id.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            if i < 16 {
                key[i] = *b;
            }
        }
        window_builder = window_builder.data_store_identifier(key);
    }

    match window_builder.build() {
        Ok(win) => {
            let _ = win.show();
            let _ = win.set_focus();
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }


}

// Recursively copy directories helper
fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn get_agent_data_dir(app_handle: &tauri::AppHandle) -> std::path::PathBuf {
    if let Ok(env_val) = std::env::var("UID_AGENT_DATA_DIR") {
        let path = std::path::PathBuf::from(env_val);
        if std::fs::create_dir_all(&path).is_ok() {
            return path;
        }
    }
    if let Ok(local_dir) = app_handle.path().app_local_data_dir() {
        if std::fs::create_dir_all(&local_dir).is_ok() {
            return local_dir;
        }
    }
    let home = get_home_dir();
    let fallback = std::path::PathBuf::from(home).join(".uid-agent");
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

fn detect_chrome_profile() -> Option<std::path::PathBuf> {
    let home = get_home_dir();
    let home_path = std::path::PathBuf::from(&home);

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            let path = std::path::PathBuf::from(local_appdata)
                .join("Google")
                .join("Chrome")
                .join("User Data")
                .join("Default");
            if path.exists() {
                return Some(path);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let path = home_path
            .join("Library")
            .join("Application Support")
            .join("Google")
            .join("Chrome")
            .join("Default");
        if path.exists() {
            return Some(path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let standard_path = home_path.join(".config").join("google-chrome").join("Default");
        if standard_path.exists() {
            return Some(standard_path);
        }
        let flatpak_path = home_path
            .join(".var")
            .join("app")
            .join("com.google.Chrome")
            .join("config")
            .join("google-chrome")
            .join("Default");
        if flatpak_path.exists() {
            return Some(flatpak_path);
        }
        let chromium_path = home_path.join(".config").join("chromium").join("Default");
        if chromium_path.exists() {
            return Some(chromium_path);
        }
    }

    None
}

struct WorkspaceApp {
    name: &'static str,
    url: &'static str,
}

fn get_workspace_app(app_id: &str) -> Option<WorkspaceApp> {
    match app_id {
        "zalo" => Some(WorkspaceApp {
            name: "Zalo Messenger",
            url: "https://chat.zalo.me/",
        }),
        "misa" => Some(WorkspaceApp {
            name: "MISA Accounting",
            url: "https://act.misa.vn/",
        }),
        "acrobat" => Some(WorkspaceApp {
            name: "Acrobat PDF Signer",
            url: "https://acrobat.adobe.com/link/acrobat/signatures",
        }),
        _ => None,
    }
}

#[tauri::command]
async fn sync_sandbox_profile(app_id: String, _target_url: String, direction: String, app_handle: tauri::AppHandle) -> Result<String, String> {
    let local_data = get_agent_data_dir(&app_handle);
    let isolated_dir = local_data.join("apps").join(&app_id);
    
    if direction == "import" {
        let chrome_profile = detect_chrome_profile()
            .ok_or_else(|| "Google Chrome or Chromium profile directory not found on this system.".to_string())?;
        
        let _ = std::fs::create_dir_all(&isolated_dir);
        let mut files_copied = 0;
        
        // 1. Sync Cookies
        let chrome_cookies = chrome_profile.join("Network").join("Cookies");
        if chrome_cookies.exists() {
            let dest = isolated_dir.join("Cookies");
            if std::fs::copy(&chrome_cookies, &dest).is_ok() {
                files_copied += 1;
            }
        }
        
        // 2. Sync Local Storage
        let chrome_localstorage = chrome_profile.join("Local Storage");
        if chrome_localstorage.exists() {
            let dest = isolated_dir.join("Local Storage");
            let _ = copy_dir_all(&chrome_localstorage, &dest);
            files_copied += 1;
        }

        // 3. Sync IndexedDB
        let chrome_indexeddb = chrome_profile.join("IndexedDB");
        if chrome_indexeddb.exists() {
            let dest = isolated_dir.join("IndexedDB");
            let _ = copy_dir_all(&chrome_indexeddb, &dest);
            files_copied += 1;
        }

        if files_copied > 0 {
            Ok(format!("Successfully imported {} profile datasets from Chrome", files_copied))
        } else {
            Err("No compatible session files found in Chrome profile to import.".to_string())
        }
    } else if direction == "restore" {
        let backup_dir = local_data.join("backups").join(format!("{}_profile_backup", app_id));
        if !backup_dir.exists() {
            return Err("No backup found to restore.".to_string());
        }
        let _ = std::fs::remove_dir_all(&isolated_dir);
        let _ = std::fs::create_dir_all(&isolated_dir);
        copy_dir_all(&backup_dir, &isolated_dir)
            .map_err(|e| format!("Restore failed: {}", e))?;
        Ok("Successfully restored profile from backup!".to_string())
    } else {
        // Export profile to backup folder
        let backup_dir = local_data.join("backups");
        let _ = std::fs::create_dir_all(&backup_dir);
        let dest = backup_dir.join(format!("{}_profile_backup", app_id));
        let _ = std::fs::remove_dir_all(&dest);
        copy_dir_all(&isolated_dir, &dest).map_err(|e| format!("Export failed: {}", e))?;
        Ok(format!("Successfully exported profile to: {}", dest.to_string_lossy()))
    }
}


#[tauri::command]
async fn remediate_firewall() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let is_ufw = std::path::Path::new("/usr/sbin/ufw").exists() || std::path::Path::new("/usr/bin/ufw").exists();
        let is_firewalld = std::path::Path::new("/usr/sbin/firewalld").exists();
        
        if is_ufw {
            let status = new_command("pkexec")
                .args(["ufw", "enable"])
                .status()
                .map_err(|e| format!("Failed to execute pkexec ufw: {}", e))?;
            
            let _ = new_command("pkexec")
                .args(["systemctl", "enable", "ufw"])
                .status();
                
            if status.success() {
                return Ok(());
            }
        } else if is_firewalld {
            let status = new_command("pkexec")
                .args(["systemctl", "enable", "--now", "firewalld"])
                .status()
                .map_err(|e| format!("Failed to execute pkexec firewalld: {}", e))?;
                
            if status.success() {
                return Ok(());
            }
        }
        
        Err("No supported firewall (ufw/firewalld) found, or authentication failed.".to_string())
    }

    #[cfg(target_os = "macos")]
    {
        let status = new_command("sudo")
            .args(["/usr/libexec/ApplicationFirewall/socketfilterfw", "--setglobalstate", "on"])
            .status()
            .map_err(|e| format!("Failed to run socketfilterfw: {}", e))?;
        if status.success() {
            Ok(())
        } else {
            Err("Failed to enable Application Firewall. Admin privileges required.".to_string())
        }
    }

    #[cfg(target_os = "windows")]
    {
        let status = new_command("powershell")
            .args(["-NoProfile", "-Command", "Start-Process powershell -ArgumentList 'Set-NetFirewallProfile -All -Enabled True' -Verb RunAs -Wait"])
            .status()
            .map_err(|e| format!("Failed to run PowerShell: {}", e))?;
        if status.success() {
            Ok(())
        } else {
            Err("Failed to enable Windows Defender Firewall. Admin privileges required.".to_string())
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    Err("Unsupported operating system for firewall remediation.".to_string())
}

#[tauri::command]
async fn remediate_screen_lock() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let mut success = false;
        
        if let Ok(status) = new_command("gsettings")
            .args(["set", "org.gnome.desktop.screensaver", "lock-enabled", "true"])
            .status() {
            if status.success() {
                success = true;
            }
        }
        let _ = new_command("gsettings")
            .args(["set", "org.gnome.desktop.session", "idle-delay", "300"])
            .status();
            
        if let Ok(status) = new_command("kwriteconfig5")
            .args(["--group", "ScreenSaver", "--key", "Lock", "true"])
            .status() {
            if status.success() {
                success = true;
            }
        }
        
        if success {
            Ok(())
        } else {
            Err("Failed to automatically configure screen lock. Please enable lock-on-suspend/screensaver in your display settings.".to_string())
        }
    }

    #[cfg(target_os = "macos")]
    {
        let status = new_command("defaults")
            .args(["write", "com.apple.screensaver", "askForPassword", "-int", "1"])
            .status()
            .map_err(|e| format!("Failed to run defaults: {}", e))?;
        let _ = new_command("defaults")
            .args(["write", "com.apple.screensaver", "askForPasswordDelay", "-int", "0"])
            .status();
        if status.success() {
            Ok(())
        } else {
            Err("Failed to set macOS screen lock settings.".to_string())
        }
    }

    #[cfg(target_os = "windows")]
    {
        let status = new_command("powershell")
            .args(["-NoProfile", "-Command", "reg add 'HKEY_CURRENT_USER\\Control Panel\\Desktop' /v ScreenSaveActive /t REG_SZ /d 1 /f; reg add 'HKEY_CURRENT_USER\\Control Panel\\Desktop' /v ScreenSaverIsSecure /t REG_SZ /d 1 /f; reg add 'HKEY_CURRENT_USER\\Control Panel\\Desktop' /v ScreenSaveTimeOut /t REG_SZ /d 300 /f"])
            .status()
            .map_err(|e| format!("Failed to configure Windows registry for screen lock: {}", e))?;
        if status.success() {
            Ok(())
        } else {
            Err("Failed to modify registry settings for screen lock.".to_string())
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    Err("Unsupported operating system for screen lock remediation.".to_string())
}



#[tauri::command]
async fn get_posture() -> serde_json::Value {
    let p = uid_agent::posture::get_posture();
    json!({
        "os_family": p.os_name,
        "os_release": p.os_version,
        "hostname": p.hostname,
        "firewall_status": if p.firewall_active { "active" } else { "inactive" },
        "disk_encrypted": p.disk_encrypted,
        "secure_boot": p.secure_boot,
        "screen_lock_active": p.screen_lock_active,
        "ssh_keys_secure": p.ssh_keys_secure,
        "vpn_active": p.vpn_active
    })
}

#[tauri::command]
async fn get_certificates() -> Vec<serde_json::Value> {
    uid_agent::server::get_usb_certificates()
}

#[tauri::command]
async fn get_signature_history() -> Vec<serde_json::Value> {
    let base_dir = uid_agent::get_uid_data_dir();
    let path = format!("{}/signature_history.json", base_dir);
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(arr) = json_val.as_array() {
                return arr.clone();
            }
        }
    }
    Vec::new()
}


#[tauri::command]
async fn pin_to_dock(app_id: String) -> Result<String, String> {
    let (desktop_filename, _app_name) = if app_id == "agent" {
        ("uid-agent-desktop.desktop".to_string(), "UID Agent".to_string())
    } else {
        let filename = format!("uid-agent-app-{}.desktop", app_id);
        let app_name = match app_id.as_str() {
            "zalo" => "Zalo Messenger",
            "misa" => "MISA Accounting",
            "acrobat" => "Acrobat PDF Signer",
            _ => "Secure Workspace",
        };

        if let Ok(exe_path) = std::env::current_exe() {
            let home = get_home_dir();
            let desktop_dir = format!("{}/.local/share/applications", home);
            let _ = std::fs::create_dir_all(&desktop_dir);

            let exe_path_str = exe_path.to_string_lossy();

            let icon_file = match app_id.as_str() {
                "zalo" => "uid-agent-app-zalo.png",
                "misa" => "uid-agent-app-misa.ico",
                "acrobat" => "uid-agent-app-acrobat.ico",
                _ => "uid-agent-app-default.png",
            };

            let icon_path = format!("{}/.local/share/icons/{}", home, icon_file);
            let icon_dir = format!("{}/.local/share/icons", home);
            let _ = std::fs::create_dir_all(&icon_dir);

            if !std::path::Path::new(&icon_path).exists() {
                match app_id.as_str() {
                    "zalo" => {
                        let _ = std::process::Command::new("curl")
                            .arg("-L")
                            .arg("-o")
                            .arg(&icon_path)
                            .arg("https://stc-chat.zdn.vn/images/logo.png")
                            .status();
                    }
                    "misa" => {
                        let _ = std::process::Command::new("curl")
                            .arg("-L")
                            .arg("-o")
                            .arg(&icon_path)
                            .arg("https://amisapp.misa.vn/favicon.ico")
                            .status();
                    }
                    "acrobat" => {
                        let _ = std::process::Command::new("curl")
                            .arg("-L")
                            .arg("-o")
                            .arg(&icon_path)
                            .arg("https://www.adobe.com/favicon.ico")
                            .status();
                    }
                    _ => {
                        let default_agent_icon = format!("{}/.local/share/icons/uid-agent-desktop.png", home);
                        let _ = std::fs::copy(default_agent_icon, &icon_path);
                    }
                }
            }

            let desktop_content = match app_id.as_str() {
                "zalo" => format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name=Zalo Messenger (Workspace)\n\
                     Name[vi]=Zalo Messenger (Không gian làm việc)\n\
                     Exec=\"{}\" --launch-app zalo\n\
                     Icon={}\n\
                     Terminal=false\n\
                     Categories=Network;Chat;\n\
                     Comment=Isolated Secure Workspace for Zalo Messenger\n\
                     Comment[vi]=Không gian làm việc bảo mật cô lập cho Zalo Messenger\n",
                    exe_path_str, icon_path
                ),
                "misa" => format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name=MISA Accounting (Workspace)\n\
                     Name[vi]=MISA Accounting (Không gian làm việc)\n\
                     Exec=\"{}\" --launch-app misa\n\
                     Icon={}\n\
                     Terminal=false\n\
                     Categories=Office;Finance;\n\
                     Comment=Isolated Secure Workspace for MISA Accounting\n\
                     Comment[vi]=Không gian làm việc bảo mật cô lập cho MISA Accounting\n",
                    exe_path_str, icon_path
                ),
                "acrobat" => format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name=Acrobat PDF Signer (Workspace)\n\
                     Name[vi]=Acrobat PDF Signer (Không gian làm việc)\n\
                     Exec=\"{}\" --launch-app acrobat\n\
                     Icon={}\n\
                     Terminal=false\n\
                     Categories=Office;Security;\n\
                     Comment=Isolated Secure Workspace for Acrobat PDF Signer\n\
                     Comment[vi]=Không gian làm việc bảo mật cô lập cho Acrobat PDF Signer\n",
                    exe_path_str, icon_path
                ),
                _ => format!(
                    "[Desktop Entry]\n\
                     Type=Application\n\
                     Name=Secure Workspace\n\
                     Name[vi]=Không gian làm việc Bảo mật\n\
                     Exec=\"{}\" --launch-app {}\n\
                     Icon={}\n\
                     Terminal=false\n\
                     Categories=Utility;\n\
                     Comment=Isolated Secure Workspace\n\
                     Comment[vi]=Không gian làm việc bảo mật cô lập\n",
                    exe_path_str, app_id, icon_path
                ),
            };

            let dest_path = format!("{}/{}", desktop_dir, filename);
            if let Err(e) = std::fs::write(&dest_path, desktop_content) {
                return Err(format!("Failed to write desktop launcher: {}", e));
            }
        }

        (filename, app_name.to_string())
    };

    let output = new_command("gsettings")
        .args(&["get", "org.gnome.shell", "favorite-apps"])
        .output()
        .map_err(|e| format!("Failed to get GNOME settings: {}", e))?;

    let favorites_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if favorites_str.contains(&desktop_filename) {
        return Ok("Already pinned to Dock".to_string());
    }

    let new_favorites = if favorites_str == "@as []" || favorites_str == "[]" {
        format!("['{}']", desktop_filename)
    } else {
        let trimmed = favorites_str.trim_end_matches(']');
        format!("{trimmed}, '{desktop_filename}']")
    };

    let status = new_command("gsettings")
        .args(&["set", "org.gnome.shell", "favorite-apps", &new_favorites])
        .status()
        .map_err(|e| format!("Failed to set GNOME settings: {}", e))?;

    if status.success() {
        Ok(format!("Successfully pinned {} to Dock", desktop_filename))
    } else {
        Err("Failed to update GNOME favorites".to_string())
    }
}


#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
async fn check_for_updates() -> Result<String, String> {
    let current_version = env!("CARGO_PKG_VERSION");

    // Fetch latest version string from raw.githubusercontent.com
    let latest_version = {
        #[cfg(target_os = "windows")]
        {
            let output = new_command("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "(Invoke-RestMethod -Uri 'https://raw.githubusercontent.com/oneuid/uid-agent/main/src-tauri/tauri.conf.json').version"
                ])
                .output();
            if let Ok(out) = output {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                return Err("Failed to execute PowerShell update check".to_string());
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let output = new_command("sh")
                .args([
                    "-c",
                    "curl -s https://raw.githubusercontent.com/oneuid/uid-agent/main/src-tauri/tauri.conf.json | grep '\"version\"' | cut -d '\"' -f 4"
                ])
                .output();
            if let Ok(out) = output {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                return Err("Failed to execute curl update check".to_string());
            }
        }
    };

    if latest_version.is_empty() {
        return Err("Latest version info could not be fetched from GitHub.".to_string());
    }

    if latest_version == current_version {
        return Ok(format!("v{} (Already up to date)", current_version));
    }

    // Trigger update install
    #[cfg(target_os = "windows")]
    {
        let msi_url = format!(
            "https://github.com/oneuid/uid-agent/releases/download/v{}/uid-agent-desktop_{}_x64_en-US.msi",
            latest_version, latest_version
        );
        let status = new_command("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Start-Process msiexec.exe -ArgumentList '/i \"{}\" /passive' -Verb RunAs", msi_url)
            ])
            .status();

        if let Ok(stat) = status {
            if stat.success() {
                std::process::exit(0);
            }
        }
        return Err(format!("Failed to trigger installation of v{}", latest_version));
    }

    #[cfg(target_os = "linux")]
    {
        // Run install.sh script from main branch
        let status = new_command("sh")
            .args([
                "-c",
                "curl -fsSL https://raw.githubusercontent.com/oneuid/uid-agent/main/install.sh | bash"
            ])
            .status();

        if let Ok(stat) = status {
            if stat.success() {
                std::process::exit(0);
            }
        }
        return Err(format!("Failed to run update script for v{}", latest_version));
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        return Err(format!("New version v{} is available. Please download it from GitHub.", latest_version));
    }
}

const EMBEDDED_VERSION: &str = "1.3.1";

fn parse_store_version(xml: &str) -> Option<String> {
    if let Some(idx) = xml.find("version=\"") {
        let start = idx + "version=\"".len();
        if let Some(end) = xml[start..].find('"') {
            return Some(xml[start..start + end].to_string());
        }
    }
    None
}

fn get_installed_chrome_version(browser: &str, extension_id: &str) -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
    
    let path = if cfg!(target_os = "windows") {
        let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let sub_path = if browser == "edge" {
            "Microsoft\\Edge\\User Data\\Default\\Extensions"
        } else {
            "Google\\Chrome\\User Data\\Default\\Extensions"
        };
        format!("{}\\{}\\{}", local_app_data, sub_path, extension_id)
    } else if cfg!(target_os = "macos") {
        let sub_path = if browser == "edge" {
            "Library/Application Support/Microsoft Edge/Default/Extensions"
        } else {
            "Library/Application Support/Google/Chrome/Default/Extensions"
        };
        format!("{}/{}/{}", home, sub_path, extension_id)
    } else {
        // Linux
        let sub_path = if browser == "chromium" {
            ".config/chromium/Default/Extensions"
        } else {
            ".config/google-chrome/Default/Extensions"
        };
        format!("{}/{}/{}", home, sub_path, extension_id)
    };

    let path_buf = std::path::PathBuf::from(path);
    if path_buf.exists() && path_buf.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path_buf) {
            for entry in entries.flatten() {
                let manifest_path = entry.path().join("manifest.json");
                if manifest_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(manifest_path) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(version) = json.get("version").and_then(|v| v.as_str()) {
                                return Some(version.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn is_version_newer(current: &str, incoming: &str) -> bool {
    let current_parts: Vec<u32> = current.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    let incoming_parts: Vec<u32> = incoming.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    
    for i in 0..std::cmp::max(current_parts.len(), incoming_parts.len()) {
        let curr = *current_parts.get(i).unwrap_or(&0);
        let inc = *incoming_parts.get(i).unwrap_or(&0);
        if inc > curr {
            return true;
        } else if curr > inc {
            return false;
        }
    }
    false
}

async fn check_store_version(extension_id: &str) -> Option<String> {
    let url = format!(
        "https://clients2.google.com/service/update2/crx?x=id%3D{}%26v%3D0.0.0%26uc",
        extension_id
    );
    let client = reqwest::Client::new();
    if let Ok(response) = client.get(&url).send().await {
        if let Ok(body) = response.text().await {
            if body.contains("error-unknownApplication") {
                return None; // Not on the store
            }
            return parse_store_version(&body);
        }
    }
    None
}

async fn check_github_version() -> Option<String> {
    let url = "https://api.github.com/repos/oneuid/uid-extension/releases/latest";
    let client = reqwest::Client::builder()
        .user_agent("UID-Agent-Desktop/3.0")
        .build()
        .ok()?;
    if let Ok(response) = client.get(url).send().await {
        if let Ok(json) = response.json::<serde_json::Value>().await {
            if let Some(tag_name) = json.get("tag_name").and_then(|t| t.as_str()) {
                return Some(tag_name.trim_start_matches('v').to_string());
            }
        }
    }
    None
}

async fn download_file(url: &str, dest_path: &std::path::Path) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .user_agent("UID-Agent-Desktop/3.0")
        .build()
        .map_err(|e| e.to_string())?;
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    std::fs::write(dest_path, bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn extract_zip(zip_path: &std::path::Path, dest_dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    {
        let cmd = format!(
            "Expand-Archive -Force -Path '{}' -DestinationPath '{}'",
            zip_path.to_string_lossy().replace("'", "''"),
            dest_dir.to_string_lossy().replace("'", "''")
        );
        let status = std::process::Command::new("powershell")
            .args(["-Command", &cmd])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Powershell extraction failed".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let status = std::process::Command::new("unzip")
            .args([
                "-o",
                &zip_path.to_string_lossy(),
                "-d",
                &dest_dir.to_string_lossy(),
            ])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Unzip command failed".to_string());
        }
    }
    Ok(())
}

#[tauri::command]
async fn install_browser_extension(custom_chrome_id: Option<String>) -> Result<String, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;
    let exe_path_str = current_exe.to_string_lossy().to_string();

    let data_dir = uid_agent::get_uid_data_dir();
    let chrome_manifest_path = format!("{}/one.uid.agent.chrome.json", data_dir);
    let firefox_manifest_path = format!("{}/one.uid.agent.firefox.json", data_dir);

    // 1. Create Native Messaging Host Manifest for Chrome/Edge
    let mut allowed_origins = vec![
        "chrome-extension://coobgfinhhjocjlhjiaegcfolhdgiinb/".to_string(),
        "chrome-extension://hgoifnbplldplmkkllppgmdofijpfnii/".to_string(),
    ];
    if let Some(ref custom_id) = custom_chrome_id {
        if !custom_id.trim().is_empty() {
            allowed_origins.push(format!("chrome-extension://{}/", custom_id.trim()));
        }
    }

    let chrome_manifest = serde_json::json!({
        "name": "one.uid.agent",
        "description": "UID Endpoint Security Agent Native Messaging Host",
        "path": exe_path_str,
        "type": "stdio",
        "allowed_origins": allowed_origins
    });
    std::fs::write(&chrome_manifest_path, serde_json::to_string_pretty(&chrome_manifest).unwrap())
        .map_err(|e| format!("Failed to write Chrome manifest: {}", e))?;

    // 2. Create Native Messaging Host Manifest for Firefox
    let firefox_manifest = serde_json::json!({
        "name": "one.uid.agent",
        "description": "UID Endpoint Security Agent Native Messaging Host",
        "path": chrome_manifest["path"],
        "type": "stdio",
        "allowed_extensions": [
            "passkey@uid.one"
        ]
    });
    std::fs::write(&firefox_manifest_path, serde_json::to_string_pretty(&firefox_manifest).unwrap())
        .map_err(|e| format!("Failed to write Firefox manifest: {}", e))?;

    let mut logs = Vec::new();

    // Setup Local extension storage directory
    let local_extensions_dir = std::path::PathBuf::from(&data_dir).join("extensions");
    let _ = std::fs::create_dir_all(&local_extensions_dir);

    // 3. Register Native Messaging Host per Platform
    #[cfg(target_os = "windows")]
    {
        // Chrome Registry keys
        let cmd = format!(
            "reg add \"HKCU\\Software\\Google\\Chrome\\NativeMessagingHosts\\one.uid.agent\" /ve /t REG_SZ /d \"{}\" /f",
            chrome_manifest_path.replace("/", "\\")
        );
        let _ = new_command("cmd").args(["/C", &cmd]).status();

        // Edge Registry keys
        let cmd_edge = format!(
            "reg add \"HKCU\\Software\\Microsoft\\Edge\\NativeMessagingHosts\\one.uid.agent\" /ve /t REG_SZ /d \"{}\" /f",
            chrome_manifest_path.replace("/", "\\")
        );
        let _ = new_command("cmd").args(["/C", &cmd_edge]).status();

        // Firefox Registry keys
        let cmd_ff = format!(
            "reg add \"HKCU\\Software\\Mozilla\\NativeMessagingHosts\\one.uid.agent\" /ve /t REG_SZ /d \"{}\" /f",
            firefox_manifest_path.replace("/", "\\")
        );
        let _ = new_command("cmd").args(["/C", &cmd_ff]).status();

        logs.push("Registered Native Messaging Host manifest paths in Windows Registry.".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        
        let chrome_nm_dir = format!("{}/Library/Application Support/Google/Chrome/NativeMessagingHosts", home);
        let chromium_nm_dir = format!("{}/Library/Application Support/Chromium/NativeMessagingHosts", home);
        let ff_nm_dir = format!("{}/Library/Application Support/Mozilla/NativeMessagingHosts", home);

        let _ = std::fs::create_dir_all(&chrome_nm_dir);
        let _ = std::fs::copy(&chrome_manifest_path, format!("{}/one.uid.agent.json", chrome_nm_dir));

        let _ = std::fs::create_dir_all(&chromium_nm_dir);
        let _ = std::fs::copy(&chrome_manifest_path, format!("{}/one.uid.agent.json", chromium_nm_dir));

        let _ = std::fs::create_dir_all(&ff_nm_dir);
        let _ = std::fs::copy(&firefox_manifest_path, format!("{}/one.uid.agent.json", ff_nm_dir));

        logs.push("Configured Native Messaging Hosts on macOS.".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
        
        let chrome_nm_dir = format!("{}/.config/google-chrome/NativeMessagingHosts", home);
        let chromium_nm_dir = format!("{}/.config/chromium/NativeMessagingHosts", home);
        let ff_nm_dir = format!("{}/.mozilla/native-messaging-hosts", home);

        let _ = std::fs::create_dir_all(&chrome_nm_dir);
        let _ = std::fs::copy(&chrome_manifest_path, format!("{}/one.uid.agent.json", chrome_nm_dir));

        let _ = std::fs::create_dir_all(&chromium_nm_dir);
        let _ = std::fs::copy(&chrome_manifest_path, format!("{}/one.uid.agent.json", chromium_nm_dir));

        let _ = std::fs::create_dir_all(&ff_nm_dir);
        let _ = std::fs::copy(&firefox_manifest_path, format!("{}/one.uid.agent.json", ff_nm_dir));

        logs.push("Configured Native Messaging Hosts on Linux.".to_string());
    }

    // 4. Chrome / Edge Extension Install and Version Sync
    let chrome_id = "coobgfinhhjocjlhjiaegcfolhdgiinb";
    let edge_id = "hgoifnbplldplmkkllppgmdofijpfnii";

    let chrome_inst_ver = get_installed_chrome_version("chrome", chrome_id);
    let edge_inst_ver = get_installed_chrome_version("edge", edge_id);

    logs.push(format!("Current Installed Version: Chrome [{}], Edge [{}]", 
        chrome_inst_ver.as_deref().unwrap_or("None"), 
        edge_inst_ver.as_deref().unwrap_or("None")));

    // Check Chrome Web Store
    let store_chrome_ver = check_store_version(chrome_id).await;
    let store_edge_ver = check_store_version(edge_id).await;

    let use_chrome_store = store_chrome_ver.is_some();
    let use_edge_store = store_edge_ver.is_some();

    // Chrome flow
    if use_chrome_store {
        let store_ver = store_chrome_ver.unwrap();
        logs.push(format!("Chrome extension found on Web Store (Version: {}).", store_ver));
        #[cfg(target_os = "windows")]
        {
            let cmd_ext = format!("reg add \"HKCU\\Software\\Google\\Chrome\\Extensions\\{}\" /v update_url /t REG_SZ /d \"https://clients2.google.com/service/update2/crx\" /f", chrome_id);
            let _ = new_command("cmd").args(["/C", &cmd_ext]).status();
        }
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME").unwrap_or_default();
            let chrome_ext_dir = format!("{}/Library/Application Support/Google/Chrome/External Extensions", home);
            if std::fs::create_dir_all(&chrome_ext_dir).is_ok() {
                let ext_json = serde_json::json!({
                    "external_update_url": "https://clients2.google.com/service/update2/crx"
                });
                let _ = std::fs::write(format!("{}/{}.json", chrome_ext_dir, chrome_id), serde_json::to_string_pretty(&ext_json).unwrap());
            }
        }
        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
            let chrome_ext_dir = format!("{}/.config/google-chrome/External Extensions", home);
            if std::fs::create_dir_all(&chrome_ext_dir).is_ok() {
                let ext_json = serde_json::json!({
                    "external_update_url": "https://clients2.google.com/service/update2/crx"
                });
                let _ = std::fs::write(format!("{}/{}.json", chrome_ext_dir, chrome_id), serde_json::to_string_pretty(&ext_json).unwrap());
            }
        }
        logs.push("Configured Chrome to download and update extension from Web Store.".to_string());
    } else {
        logs.push("Chrome extension not available on Web Store. Initiating Offline / GitHub deployment.".to_string());
        let github_ver = check_github_version().await.unwrap_or_else(|| EMBEDDED_VERSION.to_string());
        let current_installed = chrome_inst_ver.clone().unwrap_or_else(|| "0.0.0".to_string());
        
        let target_dir = local_extensions_dir.join(chrome_id);
        let zip_path = local_extensions_dir.join("chrome-extension.zip");

        if is_version_newer(&current_installed, &github_ver) || !target_dir.exists() {
            logs.push(format!("Upgrading Chrome Extension from v{} to v{}", current_installed, github_ver));
            let mut download_success = false;
            let github_download_url = format!("https://github.com/oneuid/uid-extension/releases/download/v{}/uid-link-firefox.zip", github_ver);
            if download_file(&github_download_url, &zip_path).await.is_ok() {
                download_success = true;
            }

            if !download_success {
                logs.push("GitHub download unavailable. Packaging local embedded extension resource.".to_string());
                let local_zip_bytes = include_bytes!("../resources/uid-link-firefox.zip");
                let _ = std::fs::write(&zip_path, local_zip_bytes);
            }

            let _ = std::fs::remove_dir_all(&target_dir);
            if extract_zip(&zip_path, &target_dir).is_ok() {
                logs.push(format!("Chrome Extension unpacked successfully at: {}", target_dir.to_string_lossy()));
            }
            let _ = std::fs::remove_file(&zip_path);
        }

        #[cfg(not(target_os = "windows"))]
        {
            let home = std::env::var("HOME").unwrap_or_default();
            let chrome_ext_dir = if cfg!(target_os = "macos") {
                format!("{}/Library/Application Support/Google/Chrome/External Extensions", home)
            } else {
                format!("{}/.config/google-chrome/External Extensions", home)
            };
            if std::fs::create_dir_all(&chrome_ext_dir).is_ok() {
                let ext_json = serde_json::json!({
                    "external_dir": target_dir.to_string_lossy(),
                    "external_version": github_ver
                });
                let _ = std::fs::write(format!("{}/{}.json", chrome_ext_dir, chrome_id), serde_json::to_string_pretty(&ext_json).unwrap());
                logs.push("Configured Chrome to load local unpacked extension.".to_string());
            }
        }
        #[cfg(target_os = "windows")]
        {
            let cmd_ext = format!("reg add \"HKCU\\Software\\Google\\Chrome\\Extensions\\{}\" /v path /t REG_SZ /d \"{}\" /f", chrome_id, target_dir.to_string_lossy().replace("/", "\\"));
            let _ = new_command("cmd").args(["/C", &cmd_ext]).status();
            let cmd_ext_v = format!("reg add \"HKCU\\Software\\Google\\Chrome\\Extensions\\{}\" /v version /t REG_SZ /d \"{}\" /f", chrome_id, github_ver);
            let _ = new_command("cmd").args(["/C", &cmd_ext_v]).status();
            logs.push("Saved local unpacked path to Registry. Please verify Developer Mode is ON in Chrome/Edge if the extension does not appear.".to_string());
        }
    }

    // Edge flow (similar to Chrome)
    if use_edge_store {
        let store_ver = store_edge_ver.unwrap();
        logs.push(format!("Edge extension found on Microsoft Add-ons Store (Version: {}).", store_ver));
        #[cfg(target_os = "windows")]
        {
            let cmd_edge_ext = format!("reg add \"HKCU\\Software\\Microsoft\\Edge\\Extensions\\{}\" /v update_url /t REG_SZ /d \"https://clients2.google.com/service/update2/crx\" /f", edge_id);
            let _ = new_command("cmd").args(["/C", &cmd_edge_ext]).status();
        }
        logs.push("Configured Edge to download/update from Microsoft Web Store.".to_string());
    } else {
        logs.push("Edge extension not available on store. Using side-loaded fallback.".to_string());
        let target_dir = local_extensions_dir.join(edge_id);
        let zip_path = local_extensions_dir.join("edge-extension.zip");
        let github_ver = check_github_version().await.unwrap_or_else(|| EMBEDDED_VERSION.to_string());
        let current_installed = edge_inst_ver.unwrap_or_else(|| "0.0.0".to_string());

        if is_version_newer(&current_installed, &github_ver) || !target_dir.exists() {
            let mut download_success = false;
            let github_download_url = format!("https://github.com/oneuid/uid-extension/releases/download/v{}/uid-link-firefox.zip", github_ver);
            if download_file(&github_download_url, &zip_path).await.is_ok() {
                download_success = true;
            }
            if !download_success {
                let local_zip_bytes = include_bytes!("../resources/uid-link-firefox.zip");
                let _ = std::fs::write(&zip_path, local_zip_bytes);
            }
            let _ = std::fs::remove_dir_all(&target_dir);
            if extract_zip(&zip_path, &target_dir).is_ok() {
                logs.push(format!("Edge Extension unpacked successfully."));
            }
            let _ = std::fs::remove_file(&zip_path);
        }

        #[cfg(target_os = "windows")]
        {
            let cmd_edge_ext = format!("reg add \"HKCU\\Software\\Microsoft\\Edge\\Extensions\\{}\" /v path /t REG_SZ /d \"{}\" /f", edge_id, target_dir.to_string_lossy().replace("/", "\\"));
            let _ = new_command("cmd").args(["/C", &cmd_edge_ext]).status();
            let cmd_edge_v = format!("reg add \"HKCU\\Software\\Microsoft\\Edge\\Extensions\\{}\" /v version /t REG_SZ /d \"{}\" /f", edge_id, github_ver);
            let _ = new_command("cmd").args(["/C", &cmd_edge_v]).status();
        }
    }

    // 5. Firefox Extension Injection
    let firefox_zip_bytes = include_bytes!("../resources/uid-link-firefox.zip");
    let ff_profiles_dir = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").map(|d| format!("{}/Mozilla/Firefox/Profiles", d)).ok()
    } else if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        Some(format!("{}/Library/Application Support/Firefox/Profiles", home))
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
        Some(format!("{}/.mozilla/firefox", home))
    };

    if let Some(profiles_dir) = ff_profiles_dir {
        if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let folder_name = path.file_name().unwrap_or_default().to_string_lossy();
                    if folder_name.contains("default") || folder_name.contains("release") || folder_name.contains("dev-edition") {
                        let ext_folder = path.join("extensions");
                        let _ = std::fs::create_dir_all(&ext_folder);
                        let dest_xpi = ext_folder.join("passkey@uid.one.xpi");
                        
                        let needs_firefox_update = if dest_xpi.exists() {
                            true
                        } else {
                            true
                        };

                        if needs_firefox_update {
                            if std::fs::write(&dest_xpi, firefox_zip_bytes).is_ok() {
                                logs.push(format!("Firefox: Synchronized passkey@uid.one.xpi (v{}) into profile: {}", EMBEDDED_VERSION, folder_name));
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let _ = new_command("open")
            .args(["-b", "com.apple.Safari", "--args", "--show-extension-preferences"])
            .status();
    }

    Ok(logs.join("\n"))
}

// Entry Point
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 1. Setup background signing server & system tray
        .setup(|app| {
            // Load keys and start local HTTP signing server in background tokio thread
            let keys = Arc::new(uid_agent::crypto::AgentKeys::load_or_create().unwrap());
            let keys_clone = keys.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = uid_agent::server::start_web_server(keys_clone).await {
                    eprintln!("[uid-agent-desktop] Web server error: {:?}", e);
                }
            });

            // Write custom app icons to user local share icons directory
            let home = get_home_dir();
            let icons_dir = format!("{}/.local/share/icons", home);
            let _ = std::fs::create_dir_all(&icons_dir);

            // Embed gold shield agent icon bytes and save it locally
            let agent_icon_bytes = include_bytes!("../icons/128x128.png");
            let agent_icon_path = format!("{}/uid-agent-desktop.png", icons_dir);
            let _ = std::fs::write(&agent_icon_path, agent_icon_bytes);

            // Register main agent desktop launcher dynamically with the current binary path
            if let Ok(exe_path) = std::env::current_exe() {
                let desktop_dir = format!("{}/.local/share/applications", home);
                let _ = std::fs::create_dir_all(&desktop_dir);

                let agent_desktop_content = format!(
                    "[Desktop Entry]\n\
                     Name=UID Agent\n\
                     Comment=Endpoint Security Attestation Agent\n\
                     Exec={}\n\
                     Icon=uid-agent-desktop\n\
                     Terminal=false\n\
                     Type=Application\n\
                     Categories=Security;System;\n",
                    exe_path.to_string_lossy()
                );
                let _ = std::fs::write(format!("{}/uid-agent-desktop.desktop", desktop_dir), agent_desktop_content);
            }

            // Check if application was launched for a specific sandbox workspace app shortcut
            let args: Vec<String> = std::env::args().collect();
            let mut launch_target: Option<String> = None;
            for i in 0..args.len() {
                if args[i] == "--launch-app" && i + 1 < args.len() {
                    launch_target = Some(args[i + 1].clone());
                    break;
                }
            }

            // Single instance lock check using loopback TCP socket on port 13014
            use std::io::Write;
            if let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:13014") {
                if let Some(app_id) = launch_target {
                    let _ = stream.write_all(format!("launch:{}", app_id).as_bytes());
                } else {
                    let _ = stream.write_all(b"focus-main");
                }
                std::process::exit(0);
            }


            // Start loopback TCP socket single-instance listener on port 13014 for the first running process
            let app_handle_ipc = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(listener) = tokio::net::TcpListener::bind("127.0.0.1:13014").await {
                    loop {
                        if let Ok((mut stream, _)) = listener.accept().await {
                            let app_handle = app_handle_ipc.clone();
                            tokio::spawn(async move {
                                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                                let mut buf = [0u8; 1024];
                                if let Ok(n) = stream.read(&mut buf).await {
                                    let msg = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                                    if msg.starts_with("launch:") {
                                        let app_id = msg.replace("launch:", "");
                                        if let Some(wapp) = get_workspace_app(&app_id) {
                                            let _ = launch_sandbox_app(app_id, wapp.name.to_string(), wapp.url.to_string(), app_handle);
                                        }
                                    } else if msg == "focus-main" {
                                        if let Some(main_win) = app_handle.get_webview_window("main") {
                                            let _ = main_win.show();
                                            let _ = main_win.set_focus();
                                        }
                                    }
                                }
                                let _ = stream.write_all(b"OK").await;
                            });
                        }
                    }
                }
            });

            if let Some(app_id) = launch_target {
                if let Some(wapp) = get_workspace_app(&app_id) {
                    let app_handle = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                        let _ = launch_sandbox_app(app_id, wapp.name.to_string(), wapp.url.to_string(), app_handle);
                    });

                    // Hide main window if created
                    let app_handle_main = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        if let Some(main_win) = app_handle_main.get_webview_window("main") {
                            let _ = main_win.hide();
                        }
                    });
                }
            }

            // Initialize system tray menu
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show Dashboard", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            // Build tray icon and register click events
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                window.show().unwrap();
                                window.set_focus().unwrap();
                            }
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            window.show().unwrap();
                            window.set_focus().unwrap();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        // 2. Hide window when closed instead of terminating application
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "main" {
                    api.prevent_close();
                    window.hide().unwrap();
                } else {
                    // Sandbox window close: exit app if no other visible windows exist
                    let app_handle = window.app_handle().clone();
                    let window_label = window.label().to_string();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        let mut visible_windows = 0;
                        for (label, win) in app_handle.webview_windows() {
                            if label != window_label {
                                if let Ok(true) = win.is_visible() {
                                    visible_windows += 1;
                                }
                            }
                        }
                        if visible_windows == 0 {
                            app_handle.exit(0);
                        }
                    });
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_posture,
            get_certificates,
            pin_to_dock,
            check_for_updates,
            get_user_profile,
            logout_user,
            open_browser_url,
            show_notification,
            launch_sandbox_app,
            sync_sandbox_profile,
            remediate_firewall,
            remediate_screen_lock,
            get_signature_history,
            get_app_version,
            install_browser_extension
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn read_clipboard_image_bytes_helper() -> Option<Vec<u8>> {
    #[cfg(target_os = "linux")]
    {
        // Try wl-paste (Wayland)
        if let Ok(output) = std::process::Command::new("wl-paste")
            .args(["-t", "image/png"])
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                return Some(output.stdout);
            }
        }
        // Try xclip (X11)
        if let Ok(output) = std::process::Command::new("xclip")
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
        let ps_cmd = "[void] [System.Reflection.Assembly]::LoadWithPartialName('System.Windows.Forms'); \
                      if ([System.Windows.Forms.Clipboard]::ContainsImage()) { \
                          $ms = New-Object System.IO.MemoryStream; \
                          [System.Windows.Forms.Clipboard]::GetImage().Save($ms, [System.Drawing.Imaging.ImageFormat]::Png); \
                          [System.Console]::OpenStandardOutput().Write($ms.ToArray(), 0, $ms.Length); \
                      }";
        if let Ok(output) = std::process::Command::new("powershell")
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
        if let Ok(output) = std::process::Command::new("osascript")
            .args(["-e", script])
            .output()
        {
            if output.status.success() {
                let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if stdout_str.starts_with("«data PNGf") && stdout_str.ends_with("»") {
                    let hex_content = stdout_str
                        .trim_start_matches("«data PNGf")
                        .trim_end_matches('»');
                    let decode_hex = |s: &str| -> Option<Vec<u8>> {
                        (0..s.len())
                            .step_by(2)
                            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
                            .collect()
                    };
                    if let Some(bytes) = decode_hex(hex_content) {
                        return Some(bytes);
                    }
                }
            }
        }
    }
    None
}


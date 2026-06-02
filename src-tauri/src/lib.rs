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
fn get_user_profile() -> Option<UserProfile> {
    let home = get_home_dir();
    let path = format!("{}/.config/uid/user.json", home);
    if let Ok(content) = std::fs::read_to_string(path) {
        serde_json::from_str::<UserProfile>(&content).ok()
    } else {
        None
    }
}

#[tauri::command]
fn logout_user() -> Result<(), String> {
    let home = get_home_dir();
    let path = format!("{}/.config/uid/user.json", home);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[tauri::command]
fn open_browser_url(url: String) -> Result<(), String> {
    let _ = new_command("xdg-open").arg(url).status();
    Ok(())
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
fn get_posture() -> serde_json::Value {
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
fn get_certificates() -> Vec<serde_json::Value> {
    uid_agent::server::get_usb_certificates()
}

#[tauri::command]
fn get_signature_history() -> Vec<serde_json::Value> {
    let home = get_home_dir();
    let path = format!("{}/.uid/signature_history.json", home);
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
fn pin_to_dock(app_id: String) -> Result<String, String> {
    if app_id != "agent" {
        return Err("Unsupported app".to_string());
    }
    let desktop_filename = "uid-agent-desktop.desktop".to_string();

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
async fn check_for_updates() -> Result<String, String> {
    let current_version = "3.0.3"; // matches tauri.conf.json

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
                api.prevent_close();
                window.hide().unwrap();
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
            remediate_firewall,
            remediate_screen_lock,
            get_signature_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

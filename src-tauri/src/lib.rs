use std::sync::Arc;
use std::process::Command;
use serde_json::json;
use tauri::{Manager, WindowEvent};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

// Tauri Commands

#[tauri::command]
fn get_posture() -> serde_json::Value {
    serde_json::to_value(uid_agent::posture::get_posture()).unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
fn get_certificates() -> Vec<serde_json::Value> {
    uid_agent::server::get_usb_certificates()
}

#[tauri::command]
fn check_docker_installed() -> bool {
    Command::new("docker")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
fn check_app_status(app_id: String) -> serde_json::Value {
    if app_id != "zalo" {
        return json!({ "status": "unknown" });
    }

    // Check if docker container exists and check status
    let output = Command::new("docker")
        .args(&["ps", "-a", "--filter", "name=uid-zalo", "--format", "{{.Status}}"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let status_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if status_str.is_empty() {
                json!({ "status": "not_installed" })
            } else if status_str.contains("Up") {
                json!({ "status": "running" })
            } else {
                json!({ "status": "stopped" })
            }
        }
        _ => json!({ "status": "not_installed" }),
    }
}

#[tauri::command]
fn pin_to_dock(app_id: String) -> Result<String, String> {
    let desktop_filename = if app_id == "agent" {
        "uid-agent-desktop.desktop"
    } else if app_id == "zalo" {
        "zalo-sandbox.desktop"
    } else {
        return Err("Unsupported app".to_string());
    };

    // GNOME gsettings command to get current favorites and append our desktop file
    let output = Command::new("gsettings")
        .args(&["get", "org.gnome.shell", "favorite-apps"])
        .output()
        .map_err(|e| format!("Failed to get GNOME settings: {}", e))?;

    let favorites_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if favorites_str.contains(desktop_filename) {
        return Ok("Already pinned to Dock".to_string());
    }

    // Append to GNOME favorite-apps array
    let new_favorites = if favorites_str == "@as []" || favorites_str == "[]" {
        format!("['{}']", desktop_filename)
    } else {
        // Remove trailing bracket and append
        let trimmed = favorites_str.trim_end_matches(']');
        format!("{trimmed}, '{desktop_filename}']")
    };

    let status = Command::new("gsettings")
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
async fn install_app(app_id: String) -> Result<String, String> {
    if app_id != "zalo" {
        return Err("Unsupported application".to_string());
    }

    if !check_docker_installed() {
        return Err("Docker is not installed on this system. Please install Docker first.".to_string());
    }

    // Authorize local docker X11 access
    let _ = Command::new("xhost").arg("+local:docker").status();
    let _ = Command::new("xhost").arg("+local:root").status();

    // Pull the Wine Docker image
    let pull_status = Command::new("docker")
        .args(&["pull", "scottyhardy/docker-wine:latest"])
        .status()
        .map_err(|e| format!("Failed to pull docker image: {}", e))?;

    if !pull_status.success() {
        return Err("Failed to pull docker-wine image".to_string());
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
    
    // Create folders on host to persist data (chat history, registry, configurations)
    let wine_persist_dir = format!("{}/.local/share/uid/apps/zalo/wineprefix", home);
    let downloads_dir = format!("{}/Downloads/Zalo", home);
    let _ = std::fs::create_dir_all(&wine_persist_dir);
    let _ = std::fs::create_dir_all(&downloads_dir);

    let wine_volume_mount = format!("{}:/home/wineuser/.wine", wine_persist_dir);
    let downloads_volume_mount = format!("{}:/home/wineuser/downloads", downloads_dir);

    // Create the container with X11, audio, GPU acceleration and persistent storage mounts, tail -f to keep alive
    let create_status = Command::new("docker")
        .args(&[
            "run", "-d",
            "--name", "uid-zalo",
            "--net=host",
            "--ipc=host",
            "-v", "/tmp/.X11-unix:/tmp/.X11-unix:ro",
            "-e", "DISPLAY",
            "-v", "/run/user/1000/pulse/native:/tmp/pulse-socket",
            "-e", "PULSE_SERVER=unix:/tmp/pulse-socket",
            "--device", "/dev/dri",
            "-v", &wine_volume_mount,
            "-v", &downloads_volume_mount,
            "scottyhardy/docker-wine:latest",
            "tail", "-f", "/dev/null"
        ])
        .status()
        .map_err(|e| format!("Failed to create container: {}", e))?;

    if !create_status.success() {
        // Container might already exist, remove and recreate
        let _ = Command::new("docker").args(&["stop", "uid-zalo"]).status();
        let _ = Command::new("docker").args(&["rm", "uid-zalo"]).status();
        
        let retry_status = Command::new("docker")
            .args(&[
                "run", "-d",
                "--name", "uid-zalo",
                "--net=host",
                "--ipc=host",
                "-v", "/tmp/.X11-unix:/tmp/.X11-unix:ro",
                "-e", "DISPLAY",
                "-v", "/run/user/1000/pulse/native:/tmp/pulse-socket",
                "-e", "PULSE_SERVER=unix:/tmp/pulse-socket",
                "--device", "/dev/dri",
                "-v", &wine_volume_mount,
                "-v", &downloads_volume_mount,
                "scottyhardy/docker-wine:latest",
                "tail", "-f", "/dev/null"
            ])
            .status();
        if retry_status.is_err() || !retry_status.unwrap().success() {
            return Err("Failed to start docker container".to_string());
        }
    }

    // Write Zalo SVG Icon to local user share icons folder
    let icons_dir = format!("{}/.local/share/icons", home);
    let _ = std::fs::create_dir_all(&icons_dir);
    let zalo_icon_path = format!("{}/zalo-sandbox.svg", icons_dir);
    if !std::path::Path::new(&zalo_icon_path).exists() {
        let _ = Command::new("curl")
            .args(&[
                "-L", "-o", &zalo_icon_path,
                "https://upload.wikimedia.org/wikipedia/commons/9/91/Icon_of_Zalo.svg"
            ])
            .status();
    }

    // Write desktop shortcut file
    let desktop_dir = format!("{}/.local/share/applications", home);
    let _ = std::fs::create_dir_all(&desktop_dir);
    
    // The Exec command checks if zalo is installed, if not, downloads and runs ZaloSetup.exe first
    let desktop_content = "[Desktop Entry]\n\
                           Name=Zalo (UID Sandbox)\n\
                           Comment=Run Zalo safely inside a Docker container\n\
                           Exec=xhost +local:docker && docker start uid-zalo && docker exec -d uid-zalo bash -c \"[ -f /home/wineuser/.wine/drive_c/users/wineuser/AppData/Local/Programs/Zalo/Zalo.exe ] && wine /home/wineuser/.wine/drive_c/users/wineuser/AppData/Local/Programs/Zalo/Zalo.exe || (wget -U 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36' -O /home/wineuser/downloads/ZaloSetup.exe 'https://zalo.me/download/zalo-pc?utm=90000' && wine /home/wineuser/downloads/ZaloSetup.exe)\"\n\
                           Icon=zalo-sandbox\n\
                           Terminal=false\n\
                           Type=Application\n\
                           Categories=Network;InstantMessaging;\n";
    
    std::fs::write(format!("{}/zalo-sandbox.desktop", desktop_dir), desktop_content)
        .map_err(|e| format!("Failed to write desktop shortcut: {}", e))?;

    // Download and install inside container in background task
    tauri::async_runtime::spawn(async move {
        // Run wget inside container to download ZaloSetup.exe (resolving redirect using Windows User-Agent)
        let _ = Command::new("docker")
            .args(&[
                "exec",
                "uid-zalo",
                "wget",
                "-U", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                "-O", "/home/wineuser/downloads/ZaloSetup.exe",
                "https://zalo.me/download/zalo-pc?utm=90000"
            ])
            .status();

        // Run installer
        let _ = Command::new("docker")
            .args(&[
                "exec", "-d",
                "uid-zalo",
                "wine",
                "/home/wineuser/downloads/ZaloSetup.exe"
            ])
            .status();
    });

    Ok("Successfully configured container. Downloading Zalo installer inside container and starting Wine setup...".to_string())
}

#[tauri::command]
async fn launch_app(app_id: String) -> Result<(), String> {
    if app_id != "zalo" {
        return Err("Unsupported application".to_string());
    }

    // Authorize local docker X11 access
    let _ = Command::new("xhost").arg("+local:docker").status();
    let _ = Command::new("xhost").arg("+local:root").status();

    // Start container
    let _ = Command::new("docker")
        .args(&["start", "uid-zalo"])
        .status()
        .map_err(|e| format!("Failed to start container: {}", e))?;

    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
    
    // Check if Zalo is installed
    let zalo_path = format!("{}/.local/share/uid/apps/zalo/wineprefix/drive_c/users/wineuser/AppData/Local/Programs/Zalo/Zalo.exe", home);
    let is_installed = std::path::Path::new(&zalo_path).exists();

    if is_installed {
        // Run Zalo
        Command::new("docker")
            .args(&[
                "exec", "-d",
                "uid-zalo",
                "wine",
                "/home/wineuser/.wine/drive_c/users/wineuser/AppData/Local/Programs/Zalo/Zalo.exe"
            ])
            .status()
            .map_err(|e| format!("Failed to launch Zalo: {}", e))?;
    } else {
        // Zalo is not installed, download and run installer inside container (no permission issue)
        let _ = Command::new("docker")
            .args(&[
                "exec",
                "uid-zalo",
                "wget",
                "-U", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                "-O", "/home/wineuser/downloads/ZaloSetup.exe",
                "https://zalo.me/download/zalo-pc?utm=90000"
            ])
            .status();

        Command::new("docker")
            .args(&[
                "exec", "-d",
                "uid-zalo",
                "wine",
                "/home/wineuser/downloads/ZaloSetup.exe"
            ])
            .status()
            .map_err(|e| format!("Failed to launch Zalo Setup: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn stop_app(app_id: String) -> Result<(), String> {
    if app_id != "zalo" {
        return Err("Unsupported application".to_string());
    }

    let status = Command::new("docker")
        .args(&["stop", "uid-zalo"])
        .status()
        .map_err(|e| format!("Failed to stop container: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("Failed to stop container".to_string())
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
            let home = std::env::var("HOME").unwrap_or_else(|_| "/home/s".to_string());
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
            check_docker_installed,
            check_app_status,
            install_app,
            launch_app,
            stop_app,
            pin_to_dock
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

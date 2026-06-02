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
async fn install_app(app_id: String) -> Result<String, String> {
    if app_id != "zalo" {
        return Err("Unsupported application".to_string());
    }

    if !check_docker_installed() {
        return Err("Docker is not installed on this system. Please install Docker first.".to_string());
    }

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

    // Create the container with X11, audio, GPU acceleration and persistent storage mounts
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
            "scottyhardy/docker-wine:latest"
        ])
        .status()
        .map_err(|e| format!("Failed to create container: {}", e))?;

    if !create_status.success() {
        // Container might already exist but stopped, recreate it to update configuration
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
                "scottyhardy/docker-wine:latest"
            ])
            .status();
        if retry_status.is_err() || !retry_status.unwrap().success() {
            return Err("Failed to start docker container".to_string());
        }
    }

    // Write desktop shortcut file
    let desktop_dir = format!("{}/.local/share/applications", home);
    let _ = std::fs::create_dir_all(&desktop_dir);
    
    let desktop_content = "[Desktop Entry]\n\
                           Name=Zalo (UID Sandbox)\n\
                           Comment=Run Zalo safely inside a Docker container\n\
                           Exec=docker start uid-zalo\n\
                           Icon=zalo\n\
                           Terminal=false\n\
                           Type=Application\n\
                           Categories=Network;InstantMessaging;\n";
    
    std::fs::write(format!("{}/zalo-sandbox.desktop", desktop_dir), desktop_content)
        .map_err(|e| format!("Failed to write desktop shortcut: {}", e))?;

    Ok("Successfully installed Zalo in Docker sandbox with persistent storage and registered desktop shortcut.".to_string())
}

#[tauri::command]
async fn launch_app(app_id: String) -> Result<(), String> {
    if app_id != "zalo" {
        return Err("Unsupported application".to_string());
    }

    let status = Command::new("docker")
        .args(&["start", "uid-zalo"])
        .status()
        .map_err(|e| format!("Failed to start container: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("Failed to start container".to_string())
    }
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
            stop_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

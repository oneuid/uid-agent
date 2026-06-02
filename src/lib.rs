pub mod crypto;
pub mod posture;
pub mod server;
#[cfg(unix)]
pub mod ssh_agent;
pub mod websocket;

pub fn get_uid_data_dir() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let mut path = std::path::PathBuf::from(app_data);
            path.push("uid");
            let dir = path.to_string_lossy().to_string();
            let _ = std::fs::create_dir_all(&dir);
            return dir;
        }
        let home = std::env::var("USERPROFILE")
            .or_else(|_| {
                let drive = std::env::var("HOMEDRIVE").unwrap_or_else(|_| "C:".to_string());
                std::env::var("HOMEPATH").map(|p| format!("{}{}", drive, p))
            })
            .unwrap_or_else(|_| "C:\\".to_string());
        let mut path = std::path::PathBuf::from(home);
        path.push("AppData");
        path.push("Roaming");
        path.push("uid");
        let dir = path.to_string_lossy().to_string();
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let mut path = std::path::PathBuf::from(home);
        path.push("Library");
        path.push("Application Support");
        path.push("uid");
        let dir = path.to_string_lossy().to_string();
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            let mut path = std::path::PathBuf::from(xdg_config);
            path.push("uid");
            let dir = path.to_string_lossy().to_string();
            let _ = std::fs::create_dir_all(&dir);
            return dir;
        }
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USER").map(|u| format!("/home/{}", u)))
            .unwrap_or_else(|_| "/home/s".to_string());
        let mut path = std::path::PathBuf::from(home);
        path.push(".config");
        path.push("uid");
        let dir = path.to_string_lossy().to_string();
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
}

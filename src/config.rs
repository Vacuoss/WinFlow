use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub device_id: String,
    pub device_name: String,
    pub tcp_port: u16,
    pub udp_port: u16,
    pub peer_ip: String,
    pub peer_tcp_port: u16,
    pub peer_udp_port: u16,
    pub switch_edge: String,
    pub clipboard_enabled: bool,
    pub drops_enabled: bool,
    pub drops_folder: String,
    pub hotkey_switch: String,
    pub hotkey_disconnect: String,
    pub autostart_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device_id: Uuid::new_v4().to_string(),
            device_name: hostname_fallback(),
            tcp_port: 45455,
            udp_port: 45456,

            peer_ip: "127.0.0.1".to_string(),
            peer_tcp_port: 45455,
            peer_udp_port: 45456,

            switch_edge: "right".to_string(),
            clipboard_enabled: true,
            drops_enabled: true,

            drops_folder: "WinFlow Drops".to_string(),

            hotkey_switch: "Ctrl+Alt+Right".to_string(),
            hotkey_disconnect: "Shift+LeftAlt".to_string(),

            autostart_enabled: false,
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    let dir = base.join("WinFlow");
    let _ = fs::create_dir_all(&dir);

    dir.join("config.json")
}

pub fn load_config() -> Config {
    let path = config_path();

    if !path.exists() {
        let cfg = Config::default();
        save_config(&cfg);
        return cfg;
    }

    let raw = fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_config(cfg: &Config) {
    let raw = serde_json::to_string_pretty(cfg).unwrap();
    let _ = fs::write(config_path(), raw);
}

fn hostname_fallback() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Windows-PC".to_string())
}

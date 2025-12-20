use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub proxy_port: String,
    pub api_address: String,
    pub allow_lan: bool,
    pub auto_start: bool,
    pub check_update_on_boot: bool,
    pub tun_mode: bool,
    pub log_level: String,
    pub log_output: String,
    pub log_disabled: bool,
    pub node_advanced_monitoring: bool,
    pub node_exit_status_monitoring: bool,
    pub node_exit_ip_purity: bool,
    pub keep_node_traffic_history: bool,
    pub traffic_stats_period: String,
    pub kernel_watchdog: bool,
    pub language: String,
    pub foreground_only: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            proxy_port: "7890".to_string(),
            api_address: "127.0.0.1:9090".to_string(),
            allow_lan: false,
            auto_start: false,
            check_update_on_boot: true,
            tun_mode: false,
            log_level: "info".to_string(),
            log_output: "".to_string(),
            log_disabled: false,
            node_advanced_monitoring: false,
            node_exit_status_monitoring: false,
            node_exit_ip_purity: false,
            keep_node_traffic_history: false,
            traffic_stats_period: "month".to_string(),
            kernel_watchdog: true,
            language: "zh_CN".to_string(),
            foreground_only: false,
        }
    }
}

pub struct SettingsManager {
    file_path: PathBuf,
}

impl SettingsManager {
    pub fn new(app: &AppHandle) -> Self {
        let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
        // Ensure directory exists
        if !app_data_dir.exists() {
            let _ = fs::create_dir_all(&app_data_dir);
        }
        let file_path = app_data_dir.join("settings.json");
        Self { file_path }
    }

    pub fn get_settings(&self) -> Settings {
        if !self.file_path.exists() {
            return Settings::default();
        }

        match fs::read_to_string(&self.file_path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(settings) => {
                    // NOTE: We currently trust the on-disk JSON structure; if it deserializes,
                    // we just return it as-is and fall back to defaults only when it is missing
                    // or corrupt.
                    settings
                }
                Err(_) => Settings::default(),
            },
            Err(_) => Settings::default(),
        }
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        fs::write(&self.file_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub struct SettingsState(pub Mutex<SettingsManager>);

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, SettingsState>) -> Result<Settings, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    Ok(manager.get_settings())
}

#[tauri::command]
pub fn save_settings(
    state: tauri::State<'_, SettingsState>,
    settings: Settings
) -> Result<serde_json::Value, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.save_settings(&settings)?;
    Ok(serde_json::json!({ "success": true }))
}

// Auto-launch wrappers using tauri-plugin-autostart
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_auto_launch(app: AppHandle) -> Result<serde_json::Value, String> {
    let autostart_manager = app.autolaunch();
    let enabled = autostart_manager.is_enabled().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "success": true, "enabled": enabled }))
}

#[tauri::command]
pub fn set_auto_launch(app: AppHandle, enable: bool) -> Result<serde_json::Value, String> {
    let autostart_manager = app.autolaunch();
    if enable {
        autostart_manager.enable().map_err(|e| e.to_string())?;
    } else {
        autostart_manager.disable().map_err(|e| e.to_string())?;
    }
    
    // Also update settings file to reflect this preference
    // This requires access to SettingsState, but we can rely on frontend calling save_settings separately
    // or we can try to update it here if we inject state. For now, frontend usually saves settings too.
    
    Ok(serde_json::json!({ "success": true }))
}

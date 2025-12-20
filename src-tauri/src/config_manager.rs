use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use serde_json::Value;

pub struct ConfigManager {
    current_config_path: Option<PathBuf>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            current_config_path: None,
        }
    }

    pub fn get_path(&self) -> Option<String> {
        self.current_config_path.as_ref().map(|p| p.to_string_lossy().to_string())
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.current_config_path = Some(path);
    }
}

pub struct ConfigState(pub Mutex<ConfigManager>);

/// 获取当前配置文件路径
#[tauri::command]
pub async fn get_config_path(state: tauri::State<'_, ConfigState>) -> Result<Option<String>, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    Ok(manager.get_path())
}

/// 设置当前配置文件路径
#[tauri::command]
pub async fn set_config_path(
    state: tauri::State<'_, ConfigState>,
    config_path: String,
) -> Result<(), String> {
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    let path = PathBuf::from(config_path);
    
    // 验证文件是否存在
    if !path.exists() {
        return Err(format!("Config file does not exist: {:?}", path));
    }
    
    manager.set_path(path);
    Ok(())
}

/// 获取当前配置文件内容
#[tauri::command]
pub async fn get_current_config(state: tauri::State<'_, ConfigState>) -> Result<Value, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    
    if let Some(path) = manager.get_path() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        let config: Value = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config JSON: {}", e))?;
        
        Ok(config)
    } else {
        Err("No config file is currently set".to_string())
    }
}

/// 修复配置文件（用于处理不完整的配置）
#[tauri::command]
pub async fn profile_fix(
    app: AppHandle,
    file_name: String,
) -> Result<(), String> {
    // 获取订阅管理器
    let subscription_state = app.state::<crate::subscription::SubscriptionState>();
    let manager = subscription_state.0.lock().map_err(|e| e.to_string())?;

    // 从数据库获取订阅记录，确认订阅存在
    let subscription_opt = manager
        .get_subscription(&file_name)
        .map_err(|e| e.to_string())?;

    if subscription_opt.is_none() {
        return Err(format!("Subscription not found: {}", file_name));
    }

    // 计算配置文件路径：<app_data_dir>/configs/<file_name>
    // 与旧版 Electron 逻辑保持一致
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let config_dir = app_data_dir.join("configs");
    let path = config_dir.join(&file_name);

    // 检查文件是否存在
    if !path.exists() {
        return Err(format!("Config file not found: {:?}", path));
    }

    // 尝试读取和验证配置文件
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    // 尝试解析 JSON
    let config: Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config JSON: {}", e))?;

    // 基本验证：检查是否有必需的字段
    if !config.is_object() {
        return Err("Config is not a valid JSON object".to_string());
    }

    // 如果配置有效，标记为完整
    drop(manager); // 释放锁，避免长时间持有
    let manager = subscription_state.0.lock().map_err(|e| e.to_string())?;
    manager
        .update_subscription(&file_name, &serde_json::json!({
            "isComplete": true,
            "status": "active"
        }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

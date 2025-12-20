use serde::{Serialize, Deserialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use chrono::Utc;

/// 日志级别，对齐自 src/utils/logger.js
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

/// 日志类型，对齐自 src/utils/logger.js
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogType {
    System,
    Singbox,
    Network,
    Proxy,
    Config,
}

/// 日志消息结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogMessage {
    pub level: LogLevel,
    pub log_type: LogType,
    pub message: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl LogMessage {
    pub fn new(level: LogLevel, log_type: LogType, message: String) -> Self {
        Self {
            level,
            log_type,
            message,
            timestamp: Utc::now().timestamp_millis(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// 日志管理器
pub struct LoggerManager {
    history: VecDeque<LogMessage>,
    max_history: usize,
}

impl LoggerManager {
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            max_history: 1000, // 保留最近1000条日志
        }
    }

    pub fn log(&mut self, app: &AppHandle, message: LogMessage) {
        // 添加到历史
        self.history.push_back(message.clone());
        
        // 限制历史记录大小
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }

        // 发送事件到前端
        let _ = app.emit("log-message", &message);
    }

    pub fn get_history(&self, limit: Option<usize>) -> Vec<LogMessage> {
        let limit = limit.unwrap_or(100);
        self.history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

pub struct LoggerState(pub Mutex<LoggerManager>);

/// Tauri 命令：获取日志历史
#[tauri::command]
pub fn get_log_history(
    state: tauri::State<'_, LoggerState>,
    limit: Option<usize>
) -> Result<Vec<LogMessage>, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    Ok(manager.get_history(limit))
}

/// Tauri 命令：清除日志历史
#[tauri::command]
pub fn clear_log_history(
    state: tauri::State<'_, LoggerState>
) -> Result<String, String> {
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.clear_history();
    Ok("Log history cleared".to_string())
}

/// 辅助函数：记录系统日志
pub fn log_system(app: &AppHandle, level: LogLevel, message: String) {
    let state = app.state::<LoggerState>();
    if let Ok(mut manager) = state.0.lock() {
        let log_message = LogMessage::new(level, LogType::System, message);
        manager.log(app, log_message);
    };
}

/// 辅助函数：记录 sing-box 日志
pub fn log_singbox(app: &AppHandle, level: LogLevel, message: String) {
    let state = app.state::<LoggerState>();
    if let Ok(mut manager) = state.0.lock() {
        let log_message = LogMessage::new(level, LogType::Singbox, message);
        manager.log(app, log_message);
    };
}

/// 辅助函数：记录网络日志
pub fn log_network(app: &AppHandle, level: LogLevel, message: String) {
    let state = app.state::<LoggerState>();
    if let Ok(mut manager) = state.0.lock() {
        let log_message = LogMessage::new(level, LogType::Network, message);
        manager.log(app, log_message);
    };
}

/// 辅助函数：记录代理日志
pub fn log_proxy(app: &AppHandle, level: LogLevel, message: String) {
    let state = app.state::<LoggerState>();
    if let Ok(mut manager) = state.0.lock() {
        let log_message = LogMessage::new(level, LogType::Proxy, message);
        manager.log(app, log_message);
    };
}

/// 辅助函数：记录配置日志
pub fn log_config(app: &AppHandle, level: LogLevel, message: String) {
    let state = app.state::<LoggerState>();
    if let Ok(mut manager) = state.0.lock() {
        let log_message = LogMessage::new(level, LogType::Config, message);
        manager.log(app, log_message);
    };
}
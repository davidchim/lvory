use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use chrono::Utc;

/// 连接日志条目，对齐自 src/utils/connection-logger.js
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionLog {
    pub session_id: String,
    pub domain: String,
    pub direction: String,
    pub network_type: String,
    pub node_group: Option<String>,
    pub delay: Option<i64>,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_ip: Option<String>,
}

/// 连接日志分组统计
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionGroup {
    pub domain: String,
    pub network_type: String,
    pub node_group: Option<String>,
    pub direction: String,
    pub count: usize,
    pub first_seen: i64,
    pub last_seen: i64,
    pub recent_logs: Vec<ConnectionLog>,
}

/// 连接日志管理器
pub struct ConnectionLoggerManager {
    recent_logs: VecDeque<ConnectionLog>,
    groups: HashMap<String, ConnectionGroup>,
    max_recent_logs: usize,
    max_group_logs: usize,
}

impl ConnectionLoggerManager {
    pub fn new() -> Self {
        Self {
            recent_logs: VecDeque::new(),
            groups: HashMap::new(),
            max_recent_logs: 500,
            max_group_logs: 10,
        }
    }

    /// 生成分组键
    fn get_group_key(domain: &str, network_type: &str, node_group: &Option<String>, direction: &str) -> String {
        format!(
            "{}:{}:{}:{}",
            domain,
            network_type,
            node_group.as_deref().unwrap_or("unknown"),
            direction
        )
    }

    /// 添加连接日志
    pub fn add_log(&mut self, app: &AppHandle, log: ConnectionLog) {
        // 添加到最近日志
        self.recent_logs.push_back(log.clone());
        if self.recent_logs.len() > self.max_recent_logs {
            self.recent_logs.pop_front();
        }

        // 更新分组统计
        let group_key = Self::get_group_key(&log.domain, &log.network_type, &log.node_group, &log.direction);
        
        let group = self.groups.entry(group_key.clone()).or_insert_with(|| {
            ConnectionGroup {
                domain: log.domain.clone(),
                network_type: log.network_type.clone(),
                node_group: log.node_group.clone(),
                direction: log.direction.clone(),
                count: 0,
                first_seen: log.timestamp,
                last_seen: log.timestamp,
                recent_logs: Vec::new(),
            }
        });

        group.count += 1;
        group.last_seen = log.timestamp;
        group.recent_logs.push(log.clone());
        
        // 限制每个分组的日志数量
        if group.recent_logs.len() > self.max_group_logs {
            group.recent_logs.remove(0);
        }

        // 发送事件到前端
        let _ = app.emit("connection-log", &log);
    }

    /// 获取最近的连接日志
    pub fn get_recent_logs(&self, limit: Option<usize>) -> Vec<ConnectionLog> {
        let limit = limit.unwrap_or(100);
        self.recent_logs.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// 获取连接分组统计
    pub fn get_groups(&self) -> Vec<ConnectionGroup> {
        let mut groups: Vec<ConnectionGroup> = self.groups.values().cloned().collect();
        groups.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
        groups
    }

    /// 清除历史
    pub fn clear_history(&mut self) {
        self.recent_logs.clear();
        self.groups.clear();
    }
}

pub struct ConnectionLoggerState(pub Mutex<ConnectionLoggerManager>);

/// Tauri 命令：获取连接日志历史
#[tauri::command]
pub fn get_connection_log_history(
    state: tauri::State<'_, ConnectionLoggerState>,
    limit: Option<usize>
) -> Result<Vec<ConnectionLog>, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    Ok(manager.get_recent_logs(limit))
}

/// Tauri 命令：获取连接分组统计
#[tauri::command]
pub fn get_connection_groups(
    state: tauri::State<'_, ConnectionLoggerState>
) -> Result<Vec<ConnectionGroup>, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    Ok(manager.get_groups())
}

/// Tauri 命令：清除连接日志历史
#[tauri::command]
pub fn clear_connection_log_history(
    state: tauri::State<'_, ConnectionLoggerState>
) -> Result<String, String> {
    let mut manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.clear_history();
    Ok("Connection log history cleared".to_string())
}

/// 辅助函数：从 sing-box 日志解析连接信息
pub fn parse_connection_from_singbox_log(line: &str) -> Option<ConnectionLog> {
    // 这里需要根据实际的 sing-box 日志格式进行解析
    // 示例格式可能是 JSON 或特定的文本格式
    // 以下是一个简化的示例实现
    
    // 尝试解析 JSON 格式
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        // 假设 sing-box 输出的连接日志包含这些字段
        let session_id = json.get("session")?.as_str()?.to_string();
        let domain = json.get("domain")?.as_str()?.to_string();
        let direction = json.get("direction")?.as_str().unwrap_or("outbound").to_string();
        let network_type = json.get("network")?.as_str().unwrap_or("tcp").to_string();
        let node_group = json.get("outbound").and_then(|v| v.as_str()).map(|s| s.to_string());
        let delay = json.get("delay").and_then(|v| v.as_i64());
        
        Some(ConnectionLog {
            session_id,
            domain,
            direction,
            network_type,
            node_group,
            delay,
            timestamp: Utc::now().timestamp_millis(),
            source_ip: json.get("source").and_then(|v| v.as_str()).map(|s| s.to_string()),
            destination_ip: json.get("destination").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    } else {
        None
    }
}

/// 辅助函数：记录连接日志
pub fn log_connection(app: &AppHandle, log: ConnectionLog) {
    let state = app.state::<ConnectionLoggerState>();
    if let Ok(mut manager) = state.0.lock() {
        manager.add_log(app, log);
    };
}
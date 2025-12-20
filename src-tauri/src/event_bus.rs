use tauri::{AppHandle, Emitter};
use serde::Serialize;
use std::sync::Mutex;
use std::collections::HashMap;

// 全局状态结构，对齐 JS StateManager
#[derive(Debug, Clone, Serialize)]
pub struct GlobalState {
    pub is_running: bool,
    pub is_initialized: bool,
    pub last_error: Option<String>,
    pub start_time: Option<i64>,
    pub config_path: Option<String>,
    pub connection_monitor_enabled: bool,
    // 其他动态状态可以放入 Hashmap
    pub additional: HashMap<String, String>,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            is_running: false,
            is_initialized: false,
            last_error: None,
            start_time: None,
            config_path: None,
            connection_monitor_enabled: false,
            additional: HashMap::new(),
        }
    }
}

pub struct StateManager {
    state: Mutex<GlobalState>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(GlobalState::default()),
        }
    }

    pub fn update<F>(&self, app: &AppHandle, update_fn: F)
    where
        F: FnOnce(&mut GlobalState),
    {
        if let Ok(mut state) = self.state.lock() {
            update_fn(&mut state);
            // 广播状态更新
            let _ = app.emit("state-updated", state.clone());
        }
    }

    pub fn get_state(&self) -> GlobalState {
        self.state.lock().unwrap().clone()
    }
}

pub struct StateManagerState(pub Mutex<StateManager>);

// 事件总线功能 - 在 Tauri 中主要通过 emit/listen 实现
// 这里提供辅助函数来统一事件名称，避免 "magic strings"
pub mod events {
    pub const CORE_STARTED: &str = "core-started";
    pub const CORE_STOPPING: &str = "core-stopping";
    pub const CORE_STOPPED: &str = "core-stopped";
    pub const CORE_START_FAILED: &str = "core-start-failed";
    
    pub const MONITOR_ENABLED: &str = "connection-monitor-enabled";
    pub const MONITOR_DISABLED: &str = "connection-monitor-disabled";
    pub const MONITOR_RESET: &str = "connection-monitor-reset";
    pub const RETRY_FAILED: &str = "connection-retry-failed";
    pub const RETRY_RECORDED: &str = "connection-retry-recorded";
}

// 辅助发送事件的函数
pub fn emit_state_change(app: &AppHandle, event_type: &str, payload: impl Serialize + Clone) {
    // 1. 发送具体的事件
    let _ = app.emit(event_type, payload.clone());
    
    // 2. 发送通用的 state-change 事件 (模拟 JS EventBus 行为)
    // 注意：Tauri 的事件系统不像 Node EventEmitter 那样灵活支持通配符
    // 前端如果需要监听所有 state-change，可以订阅这个特定频道
    #[derive(Serialize, Clone)]
    struct StateChangePayload<T> {
        #[serde(rename = "type")]
        event_type: String,
        data: T,
        timestamp: i64,
    }
    
    let wrapper = StateChangePayload {
        event_type: event_type.to_string(),
        data: payload,
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    
    let _ = app.emit("state-change", wrapper);
}

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use tauri::AppHandle;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrafficStats {
    pub up: u64,
    pub down: u64,
    pub timestamp: i64,
}

pub struct TrafficManager {
    history: VecDeque<TrafficStats>,
    current: TrafficStats,
}

impl TrafficManager {
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            current: TrafficStats { up: 0, down: 0, timestamp: Utc::now().timestamp_millis() },
        }
    }

    pub fn update(&mut self, up: u64, down: u64) {
        let now = Utc::now().timestamp_millis();
        let stats = TrafficStats { up, down, timestamp: now };
        
        // Add to history
        self.history.push_back(stats.clone());
        
        // Keep only recent history (e.g., last 1 hour of data points if we assume 1 sec interval = 3600 points)
        // Adjust limit as needed
        if self.history.len() > 3600 {
            self.history.pop_front();
        }

        self.current = stats;
    }

    pub fn get_current(&self) -> TrafficStats {
        self.current.clone()
    }
    
    pub fn get_history(&self, limit: usize) -> Vec<TrafficStats> {
        self.history.iter().rev().take(limit).cloned().collect()
    }
}

pub struct TrafficState(pub Mutex<TrafficManager>);

#[tauri::command]
pub fn traffic_stats_get_current(state: tauri::State<'_, TrafficState>) -> Result<TrafficStats, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    Ok(manager.get_current())
}

#[tauri::command]
pub fn traffic_stats_get_history(state: tauri::State<'_, TrafficState>, limit: Option<usize>) -> Result<Vec<TrafficStats>, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(60); // Default to last 60 points
    Ok(manager.get_history(limit))
}

// Function to parse sing-box output and update traffic
// This should be called from the sing-box output handler in lib.rs
pub fn parse_and_update_traffic(_app: &AppHandle, _line: &str) {
    // Example logic: Parse JSON output from sing-box if it emits traffic stats
    // or if we have a separate stats monitoring loop.
    // For now, let's assume we receive a specific event or line format.
    // Real implementation depends on how sing-box outputs stats (usually via API or specific log format)
    
    // Placeholder: If line contains traffic info
    // let (up, down) = parse_traffic(line);
    // let state = app.state::<TrafficState>();
    // if let Ok(mut manager) = state.0.lock() {
    //     manager.update(up, down);
    //     app.emit("traffic-update", manager.get_current()).unwrap();
    // }
}

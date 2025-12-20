use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeHistory {
    pub node_id: String,
    pub timestamp: String,
    pub up: u64,
    pub down: u64,
    pub delay: Option<i64>,
}

pub struct NodeHistoryManager {
    db_path: std::path::PathBuf,
    enabled: bool,
}

impl NodeHistoryManager {
    pub fn new(app: &AppHandle) -> Self {
        let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
        // Ensure directory exists
        if !app_data_dir.exists() {
             let _ = std::fs::create_dir_all(&app_data_dir);
        }
        let db_path = app_data_dir.join("node_history.db");
        
        let manager = Self { db_path, enabled: false };
        manager.init_db().expect("failed to init node history database");
        manager
    }

    fn get_connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
    }

    fn init_db(&self) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS node_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                up INTEGER DEFAULT 0,
                down INTEGER DEFAULT 0,
                delay INTEGER
            )",
            [],
        )?;
        
        // Index for faster queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_node_timestamp ON node_history(node_id, timestamp)",
            [],
        )?;
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn add_history(&self, node_id: &str, up: u64, down: u64, delay: Option<i64>) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let conn = self.get_connection()?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        
        conn.execute(
            "INSERT INTO node_history (node_id, timestamp, up, down, delay) VALUES (?, ?, ?, ?, ?)",
            params![node_id, timestamp, up, down, delay],
        )?;
        Ok(())
    }

    pub fn get_node_history(&self, node_id: &str, limit: i64) -> Result<Vec<NodeHistory>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT node_id, timestamp, up, down, delay FROM node_history WHERE node_id = ? ORDER BY timestamp DESC LIMIT ?")?;
        
        let history_iter = stmt.query_map(params![node_id, limit], |row| {
            Ok(NodeHistory {
                node_id: row.get(0)?,
                timestamp: row.get(1)?,
                up: row.get(2)?,
                down: row.get(3)?,
                delay: row.get(4)?,
            })
        })?;

        let mut history = Vec::new();
        for h in history_iter {
            history.push(h?);
        }
        Ok(history)
    }

    pub fn get_total_traffic(&self, node_id: &str) -> Result<(u64, u64)> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT SUM(up), SUM(down) FROM node_history WHERE node_id = ?")?;
        let mut rows = stmt.query(params![node_id])?;

        if let Some(row) = rows.next()? {
            let up: Option<u64> = row.get(0)?;
            let down: Option<u64> = row.get(1)?;
            Ok((up.unwrap_or(0), down.unwrap_or(0)))
        } else {
            Ok((0, 0))
        }
    }
}

pub struct NodeHistoryState(pub Mutex<NodeHistoryManager>);

#[tauri::command]
pub fn get_node_history(
    state: tauri::State<'_, NodeHistoryState>,
    node_id: String,
    limit: Option<i64>
) -> Result<Vec<NodeHistory>, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.get_node_history(&node_id, limit.unwrap_or(100)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_node_total_traffic(
    state: tauri::State<'_, NodeHistoryState>,
    node_id: String
) -> Result<serde_json::Value, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    let (up, down) = manager.get_total_traffic(&node_id).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "up": up, "down": down }))
}
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription {
    pub file_name: String,
    pub url: Option<String>,
    pub protocol: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub timestamp: Option<String>,
    pub last_updated: Option<String>,
    pub update_count: Option<i32>,
    pub fail_count: Option<i32>,
    pub last_error: Option<String>,
    pub last_attempt: Option<String>,
    pub loaded_at: Option<String>,
    pub singbox_cache: Option<String>,
    pub generated_from: Option<String>,
    pub last_generated: Option<String>,
    pub last_processed: Option<String>,
    pub is_cache: bool,
}

pub struct SubscriptionManager {
    db_path: std::path::PathBuf,
}

impl SubscriptionManager {
    pub fn new(app: &AppHandle) -> Self {
        let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
        std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");
        let db_path = app_data_dir.join("subscriptions.db");
        
        let manager = Self { db_path };
        manager.init_db().expect("failed to init database");
        manager
    }

    fn get_connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
    }

    fn init_db(&self) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS subscriptions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL UNIQUE,
                url TEXT,
                protocol TEXT DEFAULT 'singbox',
                status TEXT DEFAULT 'active',
                source TEXT,
                timestamp TEXT,
                last_updated TEXT,
                update_count INTEGER DEFAULT 0,
                fail_count INTEGER DEFAULT 0,
                last_error TEXT,
                last_attempt TEXT,
                loaded_at TEXT,
                singbox_cache TEXT,
                generated_from TEXT,
                last_generated TEXT,
                last_processed TEXT,
                is_cache INTEGER DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_name ON subscriptions(file_name)",
            [],
        )?;
        Ok(())
    }

    pub fn add_subscription(&self, file_name: &str, metadata: &serde_json::Value) -> Result<()> {
        let conn = self.get_connection()?;
        
        let url = metadata["url"].as_str();
        let protocol = metadata["protocol"].as_str().unwrap_or("singbox");
        let status = metadata["status"].as_str().unwrap_or("active");
        let source = metadata["source"].as_str();
        let timestamp = metadata["timestamp"].as_str();
        let last_updated = metadata["lastUpdated"].as_str();
        let update_count = metadata["updateCount"].as_i64().unwrap_or(0);
        let fail_count = metadata["failCount"].as_i64().unwrap_or(0);
        let last_error = metadata["lastError"].as_str();
        let last_attempt = metadata["lastAttempt"].as_str();
        let loaded_at = metadata["loadedAt"].as_str();
        let singbox_cache = metadata["singboxCache"].as_str();
        let generated_from = metadata["generatedFrom"].as_str();
        let last_generated = metadata["lastGenerated"].as_str();
        let last_processed = metadata["lastProcessed"].as_str();
        let is_cache = metadata["isCache"].as_bool().unwrap_or(false);

        conn.execute(
            "INSERT INTO subscriptions (
                file_name, url, protocol, status, source,
                timestamp, last_updated, update_count, fail_count,
                last_error, last_attempt, loaded_at, singbox_cache,
                generated_from, last_generated, last_processed, is_cache
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(file_name) DO UPDATE SET
                url = excluded.url,
                protocol = excluded.protocol,
                status = excluded.status,
                source = excluded.source,
                timestamp = excluded.timestamp,
                last_updated = excluded.last_updated,
                update_count = excluded.update_count,
                fail_count = excluded.fail_count,
                last_error = excluded.last_error,
                last_attempt = excluded.last_attempt,
                loaded_at = excluded.loaded_at,
                singbox_cache = excluded.singbox_cache,
                generated_from = excluded.generated_from,
                last_generated = excluded.last_generated,
                last_processed = excluded.last_processed,
                is_cache = excluded.is_cache,
                updated_at = datetime('now')",
            params![
                file_name, url, protocol, status, source,
                timestamp, last_updated, update_count, fail_count,
                last_error, last_attempt, loaded_at, singbox_cache,
                generated_from, last_generated, last_processed, is_cache
            ],
        )?;

        Ok(())
    }

    pub fn get_all_subscriptions(&self) -> Result<serde_json::Value> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM subscriptions ORDER BY created_at DESC")?;
        
        let sub_iter = stmt.query_map([], |row| {
            Ok(Subscription {
                file_name: row.get("file_name")?,
                url: row.get("url")?,
                protocol: row.get("protocol")?,
                status: row.get("status")?,
                source: row.get("source")?,
                timestamp: row.get("timestamp")?,
                last_updated: row.get("last_updated")?,
                update_count: row.get("update_count")?,
                fail_count: row.get("fail_count")?,
                last_error: row.get("last_error")?,
                last_attempt: row.get("last_attempt")?,
                loaded_at: row.get("loaded_at")?,
                singbox_cache: row.get("singbox_cache")?,
                generated_from: row.get("generated_from")?,
                last_generated: row.get("last_generated")?,
                last_processed: row.get("last_processed")?,
                is_cache: row.get::<_, i32>("is_cache")? != 0,
            })
        })?;

        let mut subscriptions = serde_json::Map::new();
        for sub in sub_iter {
            if let Ok(s) = sub {
                subscriptions.insert(s.file_name.clone(), serde_json::to_value(s).unwrap());
            }
        }

        Ok(serde_json::Value::Object(subscriptions))
    }

    pub fn get_subscription(&self, file_name: &str) -> Result<Option<Subscription>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT * FROM subscriptions WHERE file_name = ?")?;
        
        let mut rows = stmt.query(params![file_name])?;
        
        if let Some(row) = rows.next()? {
            Ok(Some(Subscription {
                file_name: row.get("file_name")?,
                url: row.get("url")?,
                protocol: row.get("protocol")?,
                status: row.get("status")?,
                source: row.get("source")?,
                timestamp: row.get("timestamp")?,
                last_updated: row.get("last_updated")?,
                update_count: row.get("update_count")?,
                fail_count: row.get("fail_count")?,
                last_error: row.get("last_error")?,
                last_attempt: row.get("last_attempt")?,
                loaded_at: row.get("loaded_at")?,
                singbox_cache: row.get("singbox_cache")?,
                generated_from: row.get("generated_from")?,
                last_generated: row.get("last_generated")?,
                last_processed: row.get("last_processed")?,
                is_cache: row.get::<_, i32>("is_cache")? != 0,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_subscription(&self, file_name: &str, updates: &serde_json::Value) -> Result<()> {
        let conn = self.get_connection()?;
        
        let mut fields = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(url) = updates["url"].as_str() { fields.push("url = ?"); values.push(Box::new(url.to_string())); }
        if let Some(protocol) = updates["protocol"].as_str() { fields.push("protocol = ?"); values.push(Box::new(protocol.to_string())); }
        if let Some(status) = updates["status"].as_str() { fields.push("status = ?"); values.push(Box::new(status.to_string())); }
        if let Some(source) = updates["source"].as_str() { fields.push("source = ?"); values.push(Box::new(source.to_string())); }
        if let Some(timestamp) = updates["timestamp"].as_str() { fields.push("timestamp = ?"); values.push(Box::new(timestamp.to_string())); }
        if let Some(last_updated) = updates["lastUpdated"].as_str() { fields.push("last_updated = ?"); values.push(Box::new(last_updated.to_string())); }
        if let Some(update_count) = updates["updateCount"].as_i64() { fields.push("update_count = ?"); values.push(Box::new(update_count)); }
        if let Some(fail_count) = updates["failCount"].as_i64() { fields.push("fail_count = ?"); values.push(Box::new(fail_count)); }
        if let Some(last_error) = updates["lastError"].as_str() { fields.push("last_error = ?"); values.push(Box::new(last_error.to_string())); }
        if let Some(last_attempt) = updates["lastAttempt"].as_str() { fields.push("last_attempt = ?"); values.push(Box::new(last_attempt.to_string())); }
        if let Some(loaded_at) = updates["loadedAt"].as_str() { fields.push("loaded_at = ?"); values.push(Box::new(loaded_at.to_string())); }
        if let Some(singbox_cache) = updates["singboxCache"].as_str() { fields.push("singbox_cache = ?"); values.push(Box::new(singbox_cache.to_string())); }
        if let Some(generated_from) = updates["generatedFrom"].as_str() { fields.push("generated_from = ?"); values.push(Box::new(generated_from.to_string())); }
        if let Some(last_generated) = updates["lastGenerated"].as_str() { fields.push("last_generated = ?"); values.push(Box::new(last_generated.to_string())); }
        if let Some(last_processed) = updates["lastProcessed"].as_str() { fields.push("last_processed = ?"); values.push(Box::new(last_processed.to_string())); }
        if let Some(is_cache) = updates["isCache"].as_bool() { fields.push("is_cache = ?"); values.push(Box::new(if is_cache { 1 } else { 0 })); }

        if fields.is_empty() {
            return Ok(());
        }

        fields.push("updated_at = datetime('now')");
        
        // Use proper parameter placeholders based on fields count
        let sql = format!("UPDATE subscriptions SET {} WHERE file_name = ?", fields.join(", "));
        
        // Add file_name to values for the WHERE clause
        values.push(Box::new(file_name.to_string()));

        // Convert Vec<Box<dyn ToSql>> to slice of references
        let params_refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();

        conn.execute(&sql, params_refs.as_slice())?;
        
        Ok(())
    }

    pub fn delete_subscription(&self, file_name: &str) -> Result<()> {
        let conn = self.get_connection()?;
        conn.execute("DELETE FROM subscriptions WHERE file_name = ?", params![file_name])?;
        Ok(())
    }
}

pub struct SubscriptionState(pub Mutex<SubscriptionManager>);

#[tauri::command]
pub fn subscription_add(
    state: tauri::State<'_, SubscriptionState>,
    file_name: String,
    metadata: serde_json::Value
) -> Result<serde_json::Value, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.add_subscription(&file_name, &metadata).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn subscription_get_all(
    state: tauri::State<'_, SubscriptionState>
) -> Result<serde_json::Value, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    let subscriptions = manager.get_all_subscriptions().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "success": true, "subscriptions": subscriptions }))
}

#[tauri::command]
pub fn subscription_get(
    state: tauri::State<'_, SubscriptionState>,
    file_name: String
) -> Result<serde_json::Value, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    let sub = manager.get_subscription(&file_name).map_err(|e| e.to_string())?;
    if let Some(s) = sub {
        Ok(serde_json::json!({ "success": true, "metadata": s }))
    } else {
        Ok(serde_json::json!({ "success": false, "error": "Subscription not found" }))
    }
}

#[tauri::command]
pub fn subscription_update(
    state: tauri::State<'_, SubscriptionState>,
    file_name: String,
    updates: serde_json::Value
) -> Result<serde_json::Value, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.update_subscription(&file_name, &updates).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub fn subscription_delete(
    state: tauri::State<'_, SubscriptionState>,
    file_name: String
) -> Result<serde_json::Value, String> {
    let manager = state.0.lock().map_err(|e| e.to_string())?;
    manager.delete_subscription(&file_name).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "success": true }))
}
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, Duration};
use tauri::{AppHandle, Manager};

pub struct LogCleanupManager {
    log_dir: PathBuf,
}

impl LogCleanupManager {
    pub fn new(app: &AppHandle) -> Self {
        let app_log_dir = app.path().app_log_dir().expect("failed to get app log dir");
        // Ensure log directory exists
        if !app_log_dir.exists() {
             let _ = fs::create_dir_all(&app_log_dir);
        }
        Self { log_dir: app_log_dir }
    }

    pub fn perform_cleanup(&self, retention_days: u64) -> Result<(u64, u64), String> {
        let cutoff_time = SystemTime::now()
            .checked_sub(Duration::from_secs(retention_days * 24 * 60 * 60))
            .ok_or("Failed to calculate cutoff time")?;

        let mut deleted_count = 0;
        let mut deleted_size = 0;

        if let Ok(entries) = fs::read_dir(&self.log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                         if extension == "log" || extension == "txt" {
                            if let Ok(metadata) = fs::metadata(&path) {
                                if let Ok(modified) = metadata.modified() {
                                    if modified < cutoff_time {
                                        let size = metadata.len();
                                        if fs::remove_file(&path).is_ok() {
                                            deleted_count += 1;
                                            deleted_size += size;
                                        }
                                    }
                                }
                            }
                         }
                    }
                }
            }
        }

        Ok((deleted_count, deleted_size))
    }
}

#[tauri::command]
pub fn perform_log_cleanup(
    app: AppHandle,
    retention_days: u64
) -> Result<serde_json::Value, String> {
    let manager = LogCleanupManager::new(&app);
    match manager.perform_cleanup(retention_days) {
        Ok((count, size)) => Ok(serde_json::json!({
            "success": true,
            "deletedCount": count,
            "deletedSize": size
        })),
        Err(e) => Ok(serde_json::json!({
            "success": false,
            "error": e
        }))
    }
}
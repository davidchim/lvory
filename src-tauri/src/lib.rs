use std::sync::{Arc, Mutex};
use std::process::{Command, Stdio, Child};
use std::io::{BufReader, BufRead};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

mod subscription;
mod log_cleanup;
mod settings;
mod traffic;
mod node_history;
mod core_manager;
mod system_proxy;
mod logger;
mod connection_logger;
mod traceroute;
mod config_manager;
mod event_bus;

// Wrapper for the child process to handle both Sidecar and Custom binaries
enum CoreProcess {
    Sidecar(tauri_plugin_shell::process::CommandChild),
    Custom(Child),
}

// 存储子进程句柄的状态
struct SingBoxState {
    child: Arc<Mutex<Option<CoreProcess>>>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn singbox_start_core(
    app: AppHandle,
    state: tauri::State<'_, SingBoxState>,
    state_manager: tauri::State<'_, event_bus::StateManagerState>,
    options: serde_json::Value,
) -> Result<String, String> {
    let mut child_guard = state.child.lock().unwrap();

    // 如果已经运行，先停止
    if child_guard.is_some() {
        return Err("SingBox core is already running".to_string());
    }

    // 从 options 解析 config_path
    let config_path = options.get("configPath")
        .and_then(|v| v.as_str())
        .ok_or("Missing configPath")?;
    
    // 更新状态为正在启动（实际没有 start 状态，但可以标记初始化）
    let sm = state_manager.0.lock().unwrap();
    sm.update(&app, |s| {
        s.is_initialized = true;
        s.last_error = None;
        s.config_path = Some(config_path.to_string());
        // is_running 只有在真正启动后才置为 true
    });
    drop(sm); // 释放锁

    println!("Starting SingBox with config: {}", config_path);

    // Check for active custom binary
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let active_bin_dir = app_data_dir.join("active_bin");
    let exe_ext = std::env::consts::EXE_EXTENSION;
    let target_name = if exe_ext.is_empty() { "sing-box" } else { "sing-box.exe" };
    let custom_bin_path = active_bin_dir.join(target_name);

    let start_result = if custom_bin_path.exists() {
        println!("Using custom binary at: {:?}", custom_bin_path);
        
        let mut command = Command::new(&custom_bin_path);
        command.args(["run", "-c", config_path]);
        
        // Setup pipes
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Spawn
        match command.spawn() {
            Ok(mut child) => {
                // Handle Output
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                let app_handle = app.clone();
                let sm_handle = state_manager.0.lock().unwrap();
                // We need to clone AppHandle for threads, but StateManager logic is tricky in threads without Arc
                // Ideally we use channels or Arc<Mutex<StateManager>> if it was sharable easily.
                // For now, simpler error buffering might be done in JS, but here we emit events.
                
                if let Some(stdout) = stdout {
                    let app_handle = app_handle.clone();
                    std::thread::spawn(move || {
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                 traffic::parse_and_update_traffic(&app_handle, &line);
                                 let _ = app_handle.emit("singbox-output", line);
                            }
                        }
                    });
                }
                
                if let Some(stderr) = stderr {
                    let app_handle = app_handle.clone();
                    std::thread::spawn(move || {
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                 let _ = app_handle.emit("singbox-output", line);
                                 // TODO: Buffer stderr for startup error detection
                            }
                        }
                    });
                }
                
                *child_guard = Some(CoreProcess::Custom(child));
                Ok("SingBox started (Custom)".to_string())
            },
            Err(e) => Err(format!("Failed to spawn custom binary: {}", e))
        }

    } else {
        println!("Using sidecar binary");
        // Use Sidecar
        match app.shell().sidecar("sing-box")
            .map_err(|e| e.to_string())?
            .args(["run", "-c", config_path])
            .spawn() 
        {
            Ok((mut rx, child)) => {
                *child_guard = Some(CoreProcess::Sidecar(child));

                // 异步监听输出
                let app_handle = app.clone();
                let state_manager_handle = state_manager.0.lock().unwrap(); // Keep it accessible? No, can't move non-cloneable
                
                tauri::async_runtime::spawn(async move {
                    while let Some(event) = rx.recv().await {
                        match event {
                            CommandEvent::Stdout(line) => {
                                let text = String::from_utf8_lossy(&line);
                                traffic::parse_and_update_traffic(&app_handle, &text);
                                let _ = app_handle.emit("singbox-output", text.to_string());
                            }
                            CommandEvent::Stderr(line) => {
                                let text = String::from_utf8_lossy(&line);
                                let _ = app_handle.emit("singbox-output", text.to_string());
                            }
                            CommandEvent::Terminated(payload) => {
                                 let _ = app_handle.emit("singbox-exit", payload.clone());
                                 // TODO: Handle exit in StateManager via invoke or another command
                                 // Or emit an event that the frontend catches to call getStatus
                                 event_bus::emit_state_change(&app_handle, event_bus::events::CORE_STOPPED, serde_json::json!({
                                     "code": payload.code,
                                     "signal": payload.signal
                                 }));
                            }
                            _ => {}
                        }
                    }
                });

                Ok("SingBox started (Sidecar)".to_string())
            },
            Err(e) => Err(e.to_string())
        }
    };
    
    match start_result {
        Ok(msg) => {
             // 启动成功，更新状态
            let sm = state_manager.0.lock().unwrap();
            sm.update(&app, |s| {
                s.is_running = true;
                s.start_time = Some(chrono::Utc::now().timestamp_millis());
            });
            
            // 发送 core-started 事件
             event_bus::emit_state_change(&app, event_bus::events::CORE_STARTED, serde_json::json!({
                 "configPath": config_path
             }));
             
             Ok(msg)
        },
        Err(e) => {
             // 启动失败
            let sm = state_manager.0.lock().unwrap();
            sm.update(&app, |s| {
                s.is_running = false;
                s.last_error = Some(e.clone());
            });
             event_bus::emit_state_change(&app, event_bus::events::CORE_START_FAILED, serde_json::json!({
                 "error": e
             }));
            Err(e)
        }
    }
}

#[tauri::command]
async fn singbox_stop_core(
    app: AppHandle,
    state: tauri::State<'_, SingBoxState>,
    state_manager: tauri::State<'_, event_bus::StateManagerState>,
) -> Result<String, String> {
    let mut child_guard = state.child.lock().unwrap();
    
    // 发送 stopping 事件
    event_bus::emit_state_change(&app, event_bus::events::CORE_STOPPING, serde_json::json!({}));
    
    if let Some(process) = child_guard.take() {
        let result = match process {
            CoreProcess::Sidecar(child) => {
                 child.kill().map_err(|e| e.to_string())
            }
            CoreProcess::Custom(mut child) => {
                 let res = child.kill().map_err(|e| e.to_string());
                 let _ = child.wait(); // Clean up zombie
                 res
            }
        };
        
        // 更新状态
        let sm = state_manager.0.lock().unwrap();
        sm.update(&app, |s| {
            s.is_running = false;
            // last_error?
        });
        
        event_bus::emit_state_change(&app, event_bus::events::CORE_STOPPED, serde_json::json!({}));
        
        match result {
            Ok(_) => Ok("SingBox stopped".to_string()),
            Err(e) => Err(e)
        }
    } else {
        Ok("SingBox was not running".to_string())
    }
}

#[tauri::command]
async fn singbox_get_status(
    state_manager: tauri::State<'_, event_bus::StateManagerState>,
) -> Result<event_bus::GlobalState, String> {
    let sm = state_manager.0.lock().unwrap();
    Ok(sm.get_state())
}

#[tauri::command]
async fn singbox_get_detailed_status(
    state_manager: tauri::State<'_, event_bus::StateManagerState>,
    // TODO: Add other states like Traffic, ConnectionMonitor status if needed
) -> Result<serde_json::Value, String> {
    let sm = state_manager.0.lock().unwrap();
    let state = sm.get_state();
    
    // Combine with other info if needed, for now just return state + success wrapper
    Ok(serde_json::json!({
        "success": true,
        "isRunning": state.is_running,
        "globalState": state,
        // "proxyConfig": ... // Get from settings or similar if managed there
    }))
}

#[tauri::command]
async fn singbox_check_installed(app: AppHandle) -> Result<serde_json::Value, String> {
    // Check custom first
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let active_bin_dir = app_data_dir.join("active_bin");
    let exe_ext = std::env::consts::EXE_EXTENSION;
    let target_name = if exe_ext.is_empty() { "sing-box" } else { "sing-box.exe" };
    if active_bin_dir.join(target_name).exists() {
        return Ok(serde_json::json!({
            "success": true,
            "installed": true
        }));
    }

    let _ = app.shell().sidecar("sing-box").map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "success": true,
        "installed": true
    }))
}

#[tauri::command]
async fn singbox_get_version(app: AppHandle) -> Result<serde_json::Value, String> {
    // Check custom first
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let active_bin_dir = app_data_dir.join("active_bin");
    let exe_ext = std::env::consts::EXE_EXTENSION;
    let target_name = if exe_ext.is_empty() { "sing-box" } else { "sing-box.exe" };
    let custom_bin_path = active_bin_dir.join(target_name);

    if custom_bin_path.exists() {
        let output = Command::new(&custom_bin_path)
            .arg("version")
            .output()
            .map_err(|e| e.to_string())?;
        
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Ok(serde_json::json!({
                "success": true,
                "version": version
            }));
        }
    }

    let sidecar_command = app.shell().sidecar("sing-box").map_err(|e| e.to_string())?;
    let output = sidecar_command.args(["version"]).output().await.map_err(|e| e.to_string())?;
    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(serde_json::json!({
            "success": true,
            "version": version
        }))
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Ok(serde_json::json!({
            "success": false,
            "error": error
        }))
    }
}

#[tauri::command]
async fn singbox_check_config(app: AppHandle, config_path: String) -> Result<String, String> {
    // Check custom first
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let active_bin_dir = app_data_dir.join("active_bin");
    let exe_ext = std::env::consts::EXE_EXTENSION;
    let target_name = if exe_ext.is_empty() { "sing-box" } else { "sing-box.exe" };
    let custom_bin_path = active_bin_dir.join(target_name);

    if custom_bin_path.exists() {
        let output = Command::new(&custom_bin_path)
            .args(["check", "-c", &config_path])
            .output()
            .map_err(|e| e.to_string())?;
         if output.status.success() {
             return Ok("Config is valid".to_string());
        }
    }

    let sidecar_command = app.shell().sidecar("sing-box").map_err(|e| e.to_string())?;
    let output = sidecar_command.args(["check", "-c", &config_path]).output().await.map_err(|e| e.to_string())?;
    if output.status.success() {
         Ok("Config is valid".to_string())
    } else {
         Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
async fn singbox_format_config(app: AppHandle, config_path: String) -> Result<bool, String> {
     let sidecar_command = app.shell().sidecar("sing-box").map_err(|e| e.to_string())?;
    let output = sidecar_command.args(["format", "-c", &config_path, "-w"]).output().await.map_err(|e| e.to_string())?;
    if output.status.success() {
         Ok(true)
    } else {
         Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

#[tauri::command]
fn get_network_interfaces() -> Result<Vec<String>, String> {
    // Stub
    Ok(vec![])
}

#[tauri::command]
fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

#[tauri::command]
fn get_arch() -> String {
    std::env::consts::ARCH.to_string()
}

#[tauri::command]
fn app_quit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
async fn get_app_version() -> Result<serde_json::Value, String> {
    // 从 Cargo.toml 获取应用版本
    let version = env!("CARGO_PKG_VERSION");
    Ok(serde_json::json!({
        "success": true,
        "version": version
    }))
}

#[tauri::command]
async fn get_build_date() -> Result<serde_json::Value, String> {
    // 从构建时的环境变量中获取构建日期
    // 如果没有设置，使用包版本作为后备
    let build_date = option_env!("BUILD_DATE")
        .unwrap_or(env!("CARGO_PKG_VERSION"));
    Ok(serde_json::json!({
        "success": true,
        "buildDate": build_date
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![])))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main").expect("no main window").set_focus();
        }))
        .setup(|app| {
            let subscription_manager = subscription::SubscriptionManager::new(app.handle());
            app.manage(subscription::SubscriptionState(Mutex::new(subscription_manager)));

            let settings_manager = settings::SettingsManager::new(app.handle());
            app.manage(settings::SettingsState(Mutex::new(settings_manager)));

            let traffic_manager = traffic::TrafficManager::new();
            app.manage(traffic::TrafficState(Mutex::new(traffic_manager)));

            let node_history_manager = node_history::NodeHistoryManager::new(app.handle());
            app.manage(node_history::NodeHistoryState(Mutex::new(node_history_manager)));

            let logger_manager = logger::LoggerManager::new();
            app.manage(logger::LoggerState(Mutex::new(logger_manager)));

            let connection_logger_manager = connection_logger::ConnectionLoggerManager::new();
            app.manage(connection_logger::ConnectionLoggerState(Mutex::new(connection_logger_manager)));

            let config_manager = config_manager::ConfigManager::new();
            app.manage(config_manager::ConfigState(Mutex::new(config_manager)));

            // Initialize EventBus/StateManager
            let state_manager = event_bus::StateManager::new();
            app.manage(event_bus::StateManagerState(Mutex::new(state_manager)));

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app: &AppHandle, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray: &TrayIcon, event| match event {
                    TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .manage(SingBoxState {
            child: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            singbox_start_core,
            singbox_stop_core,
            subscription::subscription_add,
            subscription::subscription_get,
            subscription::subscription_get_all,
            subscription::subscription_update,
            subscription::subscription_delete,
            log_cleanup::perform_log_cleanup,
            settings::get_settings,
            settings::save_settings,
            settings::get_auto_launch,
            settings::set_auto_launch,
            traffic::traffic_stats_get_current,
            traffic::traffic_stats_get_history,
            node_history::get_node_history,
            node_history::get_node_total_traffic,
            singbox_check_installed,
            singbox_get_version,
            singbox_check_config,
            singbox_format_config,
            get_network_interfaces,
            get_platform,
            get_arch,
            app_quit,
            core_manager::singbox_get_releases,
            core_manager::singbox_get_installed_versions,
            core_manager::singbox_download_version,
            core_manager::singbox_delete_version,
            core_manager::singbox_switch_version,
            get_app_version,
            get_build_date,
            system_proxy::set_system_proxy,
            system_proxy::clear_system_proxy,
            logger::get_log_history,
            logger::clear_log_history,
            connection_logger::get_connection_log_history,
            connection_logger::get_connection_groups,
            connection_logger::clear_connection_log_history,
            traceroute::traceroute_execute,
            traceroute::traceroute_validate,
            config_manager::get_config_path,
            config_manager::set_config_path,
            config_manager::get_current_config,
            config_manager::profile_fix,
            singbox_get_status,
            singbox_get_detailed_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

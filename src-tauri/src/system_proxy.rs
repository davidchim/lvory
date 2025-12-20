use std::process::Command;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
}

/// 设置系统代理
/// 行为对齐自 src/utils/system-proxy.js
#[tauri::command]
pub async fn set_system_proxy(host: String, port: u16) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    return set_windows_proxy(&host, port);
    
    #[cfg(target_os = "linux")]
    return set_linux_proxy(&host, port);
    
    #[cfg(target_os = "macos")]
    return set_macos_proxy(&host, port);
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return Err(format!("Unsupported OS: {}", std::env::consts::OS));
}

/// 清除系统代理
#[tauri::command]
pub async fn clear_system_proxy() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    return clear_windows_proxy();
    
    #[cfg(target_os = "linux")]
    return clear_linux_proxy();
    
    #[cfg(target_os = "macos")]
    return clear_macos_proxy();
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return Err(format!("Unsupported OS: {}", std::env::consts::OS));
}

// Windows 实现
#[cfg(target_os = "windows")]
fn set_windows_proxy(host: &str, port: u16) -> Result<String, String> {
    use winreg::enums::*;
    use winreg::RegKey;
    
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu
        .open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings", KEY_WRITE)
        .map_err(|e| e.to_string())?;
    
    // 设置代理服务器
    let proxy_server = format!("{}:{}", host, port);
    internet_settings
        .set_value("ProxyServer", &proxy_server)
        .map_err(|e| e.to_string())?;
    
    // 启用代理
    internet_settings
        .set_value("ProxyEnable", &1u32)
        .map_err(|e| e.to_string())?;
    
    // 设置代理覆盖（本地地址不使用代理）
    internet_settings
        .set_value("ProxyOverride", &"localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*")
        .map_err(|e| e.to_string())?;
    
    // 通知系统刷新网络设置
    let output = Command::new("rundll32.exe")
        .args(&["wininet.dll,InternetSetOption", "0", "37", "0", "0"])
        .output()
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        Ok(format!("Windows proxy set to {}:{}", host, port))
    } else {
        Err("Failed to notify system of proxy change".to_string())
    }
}

#[cfg(target_os = "windows")]
fn clear_windows_proxy() -> Result<String, String> {
    use winreg::enums::*;
    use winreg::RegKey;
    
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu
        .open_subkey_with_flags("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings", KEY_WRITE)
        .map_err(|e| e.to_string())?;
    
    // 禁用代理
    internet_settings
        .set_value("ProxyEnable", &0u32)
        .map_err(|e| e.to_string())?;
    
    // 通知系统刷新网络设置
    let output = Command::new("rundll32.exe")
        .args(&["wininet.dll,InternetSetOption", "0", "37", "0", "0"])
        .output()
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        Ok("Windows proxy cleared".to_string())
    } else {
        Err("Failed to notify system of proxy change".to_string())
    }
}

// Linux 实现 (使用 gsettings for GNOME)
#[cfg(target_os = "linux")]
fn set_linux_proxy(host: &str, port: u16) -> Result<String, String> {
    // 设置 HTTP 代理
    Command::new("gsettings")
        .args(&["set", "org.gnome.system.proxy.http", "host", host])
        .output()
        .map_err(|e| e.to_string())?;
    
    Command::new("gsettings")
        .args(&["set", "org.gnome.system.proxy.http", "port", &port.to_string()])
        .output()
        .map_err(|e| e.to_string())?;
    
    // 设置 HTTPS 代理
    Command::new("gsettings")
        .args(&["set", "org.gnome.system.proxy.https", "host", host])
        .output()
        .map_err(|e| e.to_string())?;
    
    Command::new("gsettings")
        .args(&["set", "org.gnome.system.proxy.https", "port", &port.to_string()])
        .output()
        .map_err(|e| e.to_string())?;
    
    // 设置 SOCKS 代理
    Command::new("gsettings")
        .args(&["set", "org.gnome.system.proxy.socks", "host", host])
        .output()
        .map_err(|e| e.to_string())?;
    
    Command::new("gsettings")
        .args(&["set", "org.gnome.system.proxy.socks", "port", &port.to_string()])
        .output()
        .map_err(|e| e.to_string())?;
    
    // 设置忽略列表
    let ignore_hosts = "['localhost', '127.0.0.0/8', '10.0.0.0/8', '172.16.0.0/12', '192.168.0.0/16']";
    Command::new("gsettings")
        .args(&["set", "org.gnome.system.proxy", "ignore-hosts", ignore_hosts])
        .output()
        .map_err(|e| e.to_string())?;
    
    // 启用代理模式为 manual
    let output = Command::new("gsettings")
        .args(&["set", "org.gnome.system.proxy", "mode", "manual"])
        .output()
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        Ok(format!("Linux proxy set to {}:{}", host, port))
    } else {
        Err("Failed to set Linux proxy".to_string())
    }
}

#[cfg(target_os = "linux")]
fn clear_linux_proxy() -> Result<String, String> {
    // 设置代理模式为 none
    let output = Command::new("gsettings")
        .args(&["set", "org.gnome.system.proxy", "mode", "none"])
        .output()
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        Ok("Linux proxy cleared".to_string())
    } else {
        Err("Failed to clear Linux proxy".to_string())
    }
}

// macOS 实现
#[cfg(target_os = "macos")]
fn set_macos_proxy(host: &str, port: u16) -> Result<String, String> {
    // 获取所有网络服务
    let output = Command::new("networksetup")
        .args(&["-listallnetworkservices"])
        .output()
        .map_err(|e| e.to_string())?;
    
    let services = String::from_utf8_lossy(&output.stdout);
    let service_list: Vec<&str> = services.lines()
        .skip(1) // 跳过第一行提示
        .collect();
    
    for service in service_list {
        let service = service.trim();
        if service.is_empty() || service.starts_with("*") {
            continue;
        }
        
        // 设置 Web Proxy (HTTP)
        Command::new("networksetup")
            .args(&["-setwebproxy", service, host, &port.to_string()])
            .output()
            .map_err(|e| e.to_string())?;
        
        // 设置 Secure Web Proxy (HTTPS)
        Command::new("networksetup")
            .args(&["-setsecurewebproxy", service, host, &port.to_string()])
            .output()
            .map_err(|e| e.to_string())?;
        
        // 设置 SOCKS Proxy
        Command::new("networksetup")
            .args(&["-setsocksfirewallproxy", service, host, &port.to_string()])
            .output()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(format!("macOS proxy set to {}:{}", host, port))
}

#[cfg(target_os = "macos")]
fn clear_macos_proxy() -> Result<String, String> {
    // 获取所有网络服务
    let output = Command::new("networksetup")
        .args(&["-listallnetworkservices"])
        .output()
        .map_err(|e| e.to_string())?;
    
    let services = String::from_utf8_lossy(&output.stdout);
    let service_list: Vec<&str> = services.lines()
        .skip(1)
        .collect();
    
    for service in service_list {
        let service = service.trim();
        if service.is_empty() || service.starts_with("*") {
            continue;
        }
        
        // 关闭 Web Proxy
        Command::new("networksetup")
            .args(&["-setwebproxystate", service, "off"])
            .output()
            .map_err(|e| e.to_string())?;
        
        // 关闭 Secure Web Proxy
        Command::new("networksetup")
            .args(&["-setsecurewebproxystate", service, "off"])
            .output()
            .map_err(|e| e.to_string())?;
        
        // 关闭 SOCKS Proxy
        Command::new("networksetup")
            .args(&["-setsocksfirewallproxystate", service, "off"])
            .output()
            .map_err(|e| e.to_string())?;
    }
    
    Ok("macOS proxy cleared".to_string())
}

// 非支持平台的存根实现
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn set_windows_proxy(_host: &str, _port: u16) -> Result<String, String> {
    Err("Not implemented for this platform".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn clear_windows_proxy() -> Result<String, String> {
    Err("Not implemented for this platform".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn set_linux_proxy(_host: &str, _port: u16) -> Result<String, String> {
    Err("Not implemented for this platform".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn clear_linux_proxy() -> Result<String, String> {
    Err("Not implemented for this platform".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn set_macos_proxy(_host: &str, _port: u16) -> Result<String, String> {
    Err("Not implemented for this platform".to_string())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn clear_macos_proxy() -> Result<String, String> {
    Err("Not implemented for this platform".to_string())
}
use std::process::Command;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TracerouteHop {
    pub hop: u32,
    pub ip: Option<String>,
    pub hostname: Option<String>,
    pub rtt: Vec<Option<f64>>,
}

/// 验证目标地址是否有效
#[tauri::command]
pub async fn traceroute_validate(target: String) -> Result<bool, String> {
    // 简单的验证逻辑
    if target.trim().is_empty() {
        return Ok(false);
    }
    
    // IPv4 正则验证
    let ipv4_regex = regex::Regex::new(
        r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$"
    ).unwrap();
    
    // 域名正则验证
    let domain_regex = regex::Regex::new(
        r"^[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*$"
    ).unwrap();
    
    Ok(ipv4_regex.is_match(&target) || domain_regex.is_match(&target))
}

/// 执行 traceroute 命令
#[tauri::command]
pub async fn traceroute_execute(target: String) -> Result<Vec<TracerouteHop>, String> {
    // 验证目标
    if !traceroute_validate(target.clone()).await? {
        return Err("Invalid target address".to_string());
    }
    
    // 根据操作系统选择命令
    let (command, args) = if cfg!(target_os = "windows") {
        ("tracert", vec!["-d", "-h", "30", &target])
    } else {
        ("traceroute", vec!["-n", "-m", "30", &target])
    };
    
    // 执行命令
    let output = Command::new(command)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute traceroute: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Traceroute failed: {}", stderr));
    }
    
    // 解析输出
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_traceroute_output(&stdout, cfg!(target_os = "windows"))
}

/// 解析 traceroute 输出
fn parse_traceroute_output(output: &str, is_windows: bool) -> Result<Vec<TracerouteHop>, String> {
    let mut hops = Vec::new();
    
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        
        // 跳过标题行
        if line.starts_with("Tracing") || line.starts_with("traceroute") || 
           line.starts_with("over a maximum") {
            continue;
        }
        
        // 解析跳数
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        
        // 尝试解析跳数
        if let Ok(hop_num) = parts[0].parse::<u32>() {
            let mut ip = None;
            let mut hostname = None;
            let mut rtt = Vec::new();
            
            // Windows 格式: hop_num  rtt ms  rtt ms  rtt ms  ip
            // Linux 格式: hop_num  ip  rtt ms  rtt ms  rtt ms
            
            for &part in &parts[1..] {
                // 检查是否是 IP 地址
                if part.contains('.') && !part.ends_with("ms") {
                    ip = Some(part.to_string());
                }
                // 检查是否是 RTT 值
                else if part.ends_with("ms") {
                    if let Ok(rtt_val) = part.trim_end_matches("ms").trim().parse::<f64>() {
                        rtt.push(Some(rtt_val));
                    }
                }
                // 处理超时情况
                else if part == "*" || part.to_lowercase() == "request" {
                    rtt.push(None);
                }
            }
            
            hops.push(TracerouteHop {
                hop: hop_num,
                ip,
                hostname,
                rtt,
            });
        }
    }
    
    if hops.is_empty() {
        return Err("No valid hops found in traceroute output".to_string());
    }
    
    Ok(hops)
}

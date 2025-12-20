use tauri::{AppHandle, Manager};
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use reqwest::header::USER_AGENT;
use futures_util::StreamExt;
use std::io::Write;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Release {
    pub tag_name: String,
    pub name: String,
    pub prerelease: bool,
    pub published_at: String,
    // derived field for frontend filtering: "stable" / "alpha"
    pub version_type: String,
    pub assets: Vec<Asset>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Serialize)]
pub struct InstalledVersion {
    pub version: String,
    pub path: String,
}

#[tauri::command]
pub async fn singbox_get_releases() -> Result<Vec<Release>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/SagerNet/sing-box/releases")
        .header(USER_AGENT, "Lvory")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Failed to fetch releases: {}", response.status()));
    }

    let releases: Vec<serde_json::Value> = response.json().await.map_err(|e| e.to_string())?;
    
    // Map minimal needed fields
    let mapped_releases = releases
        .into_iter()
        .map(|r| {
            let prerelease = r["prerelease"].as_bool().unwrap_or(false);
            let version_type = if prerelease { "alpha" } else { "stable" }.to_string();

            let assets_arr: Vec<serde_json::Value> = r["assets"]
                .as_array()
                .cloned()
                .unwrap_or_default();

            let assets = assets_arr
                .into_iter()
                .map(|a| Asset {
                    name: a["name"].as_str().unwrap_or("").to_string(),
                    browser_download_url: a["browser_download_url"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    size: a["size"].as_u64().unwrap_or(0),
                })
                .collect();

            Release {
                tag_name: r["tag_name"].as_str().unwrap_or("").to_string(),
                name: r["name"].as_str().unwrap_or("").to_string(),
                prerelease,
                published_at: r["published_at"].as_str().unwrap_or("").to_string(),
                version_type,
                assets,
            }
        })
        .collect();

    Ok(mapped_releases)
}

fn get_kernels_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let kernels_dir = app_data_dir.join("kernels");
    if !kernels_dir.exists() {
        fs::create_dir_all(&kernels_dir).map_err(|e| e.to_string())?;
    }
    Ok(kernels_dir)
}

#[tauri::command]
pub async fn singbox_get_installed_versions(app: AppHandle) -> Result<Vec<String>, String> {
    let kernels_dir = get_kernels_dir(&app)?;
    let mut versions = Vec::new();

    if let Ok(entries) = fs::read_dir(kernels_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        if let Ok(file_name) = entry.file_name().into_string() {
                            versions.push(file_name);
                        }
                    }
                }
            }
        }
    }
    
    // Check bundled version (sidecar) - maybe add "bundled" to list or handle in frontend
    // versions.push("bundled".to_string());

    Ok(versions)
}

#[tauri::command]
pub async fn singbox_download_version(app: AppHandle, version: String, url: String) -> Result<String, String> {
    let kernels_dir = get_kernels_dir(&app)?;
    let version_dir = kernels_dir.join(&version);

    if version_dir.exists() {
        return Ok("Version already exists".to_string());
    }

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header(USER_AGENT, "Lvory")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()));
    }

    let temp_file_path = kernels_dir.join(format!("{}.tmp", version));
    let mut file = fs::File::create(&temp_file_path).map_err(|e| e.to_string())?;
    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
    }
    
    // Extract
    let file = fs::File::open(&temp_file_path).map_err(|e| e.to_string())?;
    
    // Simple logic: if zip, unzip; if tar.gz, untar
    if url.ends_with(".zip") {
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        // Find the binary inside
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            // We want to extract into version_dir
            // But usually the zip has a folder inside. We just want the binary.
            // Simplified: Extract everything to version_dir
            
            let dest_path = version_dir.join(outpath);
            if let Some(p) = dest_path.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
            }
            
             if file.name().ends_with('/') {
                fs::create_dir_all(&dest_path).map_err(|e| e.to_string())?;
            } else {
                let mut outfile = fs::File::create(&dest_path).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            }
        }
    } else if url.ends_with(".tar.gz") {
        let tar = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(&version_dir).map_err(|e| e.to_string())?;
    } else {
        // Assume direct binary or unknown
        fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;
        // Move temp file to version_dir/sing-box (or .exe)
        // This part is tricky if we don't know the binary name.
        // For now, let's assume valid archives.
    }

    // Clean up temp file
    let _ = fs::remove_file(temp_file_path);

    Ok("Download complete".to_string())
}

#[tauri::command]
pub async fn singbox_delete_version(app: AppHandle, version: String) -> Result<String, String> {
    let kernels_dir = get_kernels_dir(&app)?;
    let version_dir = kernels_dir.join(&version);
    
    if version_dir.exists() {
        fs::remove_dir_all(version_dir).map_err(|e| e.to_string())?;
        Ok("Deleted".to_string())
    } else {
        Err("Version not found".to_string())
    }
}

// Helper to find the executable in a version directory
pub fn find_singbox_binary(version_dir: &Path) -> Option<PathBuf> {
    // Search recursively or just check top level
    // Sing-box archives usually have a folder `sing-box-version-os-arch/sing-box(.exe)`
    if !version_dir.exists() { return None; }

    let walker = |dir: &Path| -> Option<PathBuf> {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            // Check for sing-box or sing-box.exe
                            if name == "sing-box" || name == "sing-box.exe" {
                                return Some(path);
                            }
                        }
                    } else if path.is_dir() {
                        // Go one level deep?
                        // archives usually: sing-box-1.8.0-linux-amd64/sing-box
                    }
                }
            }
        }
        None
    };

    // First check root of version_dir
    if let Some(p) = walker(version_dir) {
        return Some(p);
    }
    
    // Check subdirectories
    if let Ok(entries) = fs::read_dir(version_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    if let Some(p) = walker(&entry.path()) {
                        return Some(p);
                    }
                }
            }
        }
    }

    None
}

#[tauri::command]
pub async fn singbox_switch_version(app: AppHandle, version: String) -> Result<String, String> {
    // If version is "default" or "bundled", we clear the active binary to use sidecar
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let active_bin_dir = app_data_dir.join("active_bin");
    
    if !active_bin_dir.exists() {
        fs::create_dir_all(&active_bin_dir).map_err(|e| e.to_string())?;
    }

    let exe_ext = std::env::consts::EXE_EXTENSION;
    let target_name = if exe_ext.is_empty() { "sing-box" } else { "sing-box.exe" }; // Simplified name
    let target_path = active_bin_dir.join(target_name);

    if version == "bundled" || version == "default" {
        if target_path.exists() {
            fs::remove_file(target_path).map_err(|e| e.to_string())?;
        }
        return Ok("Switched to bundled version".to_string());
    }

    let kernels_dir = get_kernels_dir(&app)?;
    let version_dir = kernels_dir.join(&version);
    
    let binary_path = find_singbox_binary(&version_dir).ok_or("Binary not found in version directory")?;
    
    // Copy to active_bin
    fs::copy(&binary_path, &target_path).map_err(|e| e.to_string())?;

    // On Unix, ensure executable permission
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target_path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target_path, perms).map_err(|e| e.to_string())?;
    }

    Ok(format!("Switched to version {}", version))
}

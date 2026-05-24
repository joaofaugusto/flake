use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Manager;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const CACHE_TTL_SECS: u64 = 3600;

#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledApp {
    pub name: String,
    pub path: String,
    pub category: String,
}

#[tauri::command]
pub fn track_launch(app: tauri::AppHandle, path: String) {
    let Some(data_dir) = app.path().app_data_dir().ok() else {
        return;
    };
    let clicks_path = data_dir.join("clicks.json");

    let mut clicks: HashMap<String, u64> = std::fs::read(&clicks_path)
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default();

    *clicks.entry(path).or_insert(0) += 1;

    if let Some(parent) = clicks_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(&clicks) {
        let _ = std::fs::write(&clicks_path, json);
    }
}

fn load_clicks(data_dir: &std::path::Path) -> HashMap<String, u64> {
    let clicks_path = data_dir.join("clicks.json");
    std::fs::read(clicks_path)
        .ok()
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default()
}

fn rank_by_clicks(
    mut apps: Vec<InstalledApp>,
    data_dir: &Option<std::path::PathBuf>,
) -> Vec<InstalledApp> {
    let clicks = data_dir
        .as_ref()
        .map(|d| load_clicks(d))
        .unwrap_or_default();

    apps.sort_by(|a, b| {
        let a_clicks = clicks.get(&a.path).copied().unwrap_or(0);
        let b_clicks = clicks.get(&b.path).copied().unwrap_or(0);
        b_clicks.cmp(&a_clicks).then(a.name.cmp(&b.name))
    });

    apps
}

#[derive(Serialize, Deserialize)]
struct AppCache {
    timestamp: u64,
    apps: Vec<InstalledApp>,
}

#[tauri::command]
pub fn get_apps(app: tauri::AppHandle) -> Vec<InstalledApp> {
    let cache_path = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("apps_cache.json"));

    let data_dir = app.path().app_data_dir().ok();

    if let Some(ref path) = cache_path {
        if let Ok(data) = std::fs::read(path) {
            if let Ok(cached) = serde_json::from_slice::<AppCache>(&data) {
                let now = now_secs();
                if now.saturating_sub(cached.timestamp) < CACHE_TTL_SECS {
                    return rank_by_clicks(cached.apps, &data_dir);
                }
            }
        }
    }

    let apps = scan_shell_apps_folder();

    if let Some(ref path) = cache_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let cached = AppCache {
            timestamp: now_secs(),
            apps: apps.clone(),
        };
        if let Ok(json) = serde_json::to_string(&cached) {
            let _ = std::fs::write(path, json);
        }
    }

    rank_by_clicks(apps, &data_dir)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn scan_shell_apps_folder() -> Vec<InstalledApp> {
    use std::process::{Command, Stdio};

    let script = r#"
$shell = New-Object -ComObject Shell.Application
$ns    = $shell.NameSpace('shell:AppsFolder')
$skip  = @(
    'runtime','framework','sdk','redistributable','vcredist',
    'directx','visual c++','desktop app runtime','windows app runtime',
    '.net ','asp.net','c++ ','webview2'
)
$result = @()
foreach ($item in $ns.Items()) {
    $name = $item.Name
    $path = $item.Path
    if ([string]::IsNullOrWhiteSpace($name) -or [string]::IsNullOrWhiteSpace($path)) { continue }
    $nl = $name.ToLower()
    $bad = $false
    foreach ($w in $skip) { if ($nl.Contains($w)) { $bad = $true; break } }
    if ($bad) { continue }
    $result += [PSCustomObject]@{ name = $name; path = $path; category = '' }
}
ConvertTo-Json -InputObject @($result) -Compress
"#;

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let Ok(output) = cmd.output() else {
        return Vec::new();
    };

    let text = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<Vec<InstalledApp>>(&text)
        .or_else(|_| serde_json::from_str::<InstalledApp>(&text).map(|a| vec![a]))
        .unwrap_or_default()
}

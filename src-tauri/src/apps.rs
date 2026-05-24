use serde::{Deserialize, Serialize};
use tauri::Manager;
use std::time::{SystemTime, UNIX_EPOCH};

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

    if let Some(ref path) = cache_path {
        if let Ok(data) = std::fs::read(path) {
            if let Ok(cached) = serde_json::from_slice::<AppCache>(&data) {
                let now = now_secs();
                if now.saturating_sub(cached.timestamp) < CACHE_TTL_SECS {
                    return cached.apps;
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

    apps
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

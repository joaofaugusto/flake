use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(serde::Serialize)]
pub struct InstalledApp {
    name: String,
    path: String,
    category: String,
}

#[tauri::command]
fn get_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();

    let dirs: Vec<PathBuf> = [
        std::env::var("APPDATA").ok().map(|v| {
            PathBuf::from(v).join(r"Microsoft\Windows\Start Menu\Programs")
        }),
        std::env::var("PROGRAMDATA").ok().map(|v| {
            PathBuf::from(v).join(r"Microsoft\Windows\Start Menu\Programs")
        }),
    ]
    .into_iter()
    .flatten()
    .collect();

    for base in &dirs {
        scan_dir(base, base, &mut apps);
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name.to_lowercase() == b.name.to_lowercase());
    apps
}

fn scan_dir(dir: &Path, base: &Path, apps: &mut Vec<InstalledApp>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_dir(&path, base, apps);
        } else if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("lnk")) {
            let Some(stem) = path.file_stem() else {
                continue;
            };
            let name = stem.to_string_lossy().to_string();
            if !is_user_app(&name) {
                continue;
            }
            let category = path
                .parent()
                .filter(|p| *p != base)
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            apps.push(InstalledApp {
                name,
                path: path.to_string_lossy().to_string(),
                category,
            });
        }
    }
}

fn is_user_app(name: &str) -> bool {
    let l = name.to_lowercase();
    !l.starts_with("uninstall")
        && !l.contains("uninstaller")
        && !l.starts_with("remove ")
        && !l.ends_with(" help")
        && !l.ends_with(" documentation")
        && !l.ends_with(" readme")
        && l != "desktop"
        && l != "startup"
}

#[tauri::command]
fn launch_app(app: tauri::AppHandle, path: String) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    // ShellExecute via explorer handles .lnk files natively on Windows
    std::process::Command::new("explorer.exe")
        .arg(&path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_apps, launch_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

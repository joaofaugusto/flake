use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

#[derive(serde::Serialize)]
pub struct InstalledApp {
    name: String,
    path: String,
    category: String,
}

#[tauri::command]
fn get_apps() -> Vec<InstalledApp> {
    let mut apps = Vec::new();

    // Start Menu — user
    if let Ok(appdata) = std::env::var("APPDATA") {
        let base = PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs");
        scan_lnk_dir(&base, &base, &mut apps);
    }

    // Start Menu — system
    if let Ok(programdata) = std::env::var("PROGRAMDATA") {
        let base = PathBuf::from(programdata).join(r"Microsoft\Windows\Start Menu\Programs");
        scan_lnk_dir(&base, &base, &mut apps);
    }

    // %LOCALAPPDATA%\Programs — Electron and per-user installers (Squirrel, etc.)
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        let base = PathBuf::from(&localappdata).join("Programs");
        scan_lnk_dir(&base, &base, &mut apps);
        scan_exe_in_named_dirs(&base, &mut apps);
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name.to_lowercase() == b.name.to_lowercase());
    apps
}

fn scan_lnk_dir(dir: &Path, base: &Path, apps: &mut Vec<InstalledApp>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_lnk_dir(&path, base, apps);
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

// Catches apps like Claude that install to %LOCALAPPDATA%\Programs\<AppName>\<AppName>.exe
fn scan_exe_in_named_dirs(programs_dir: &Path, apps: &mut Vec<InstalledApp>) {
    let Ok(entries) = std::fs::read_dir(programs_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let dir_name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let Ok(files) = std::fs::read_dir(&dir) else {
            continue;
        };
        for file in files.flatten() {
            let fpath = file.path();
            if !fpath.extension().map_or(false, |e| e.eq_ignore_ascii_case("exe")) {
                continue;
            }
            let exe_name = fpath
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Only include exe when its name matches the parent directory
            if exe_name.to_lowercase() == dir_name.to_lowercase() && is_user_app(&exe_name) {
                apps.push(InstalledApp {
                    name: exe_name,
                    path: fpath.to_string_lossy().to_string(),
                    category: String::new(),
                });
                break;
            }
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
    let mut cmd = std::process::Command::new("explorer.exe");
    cmd.arg(&path);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(())
}

// Extracts icons for a list of paths in a single PowerShell call.
// Returns a map of path -> base64-encoded PNG.
#[tauri::command]
async fn get_icons(paths: Vec<String>) -> HashMap<String, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let Ok(paths_json) = serde_json::to_string(&paths) else {
        return HashMap::new();
    };

    // System.Drawing.Icon.ExtractAssociatedIcon follows .lnk files automatically.
    let script = r#"
Add-Type -AssemblyName System.Drawing
$paths = [Console]::In.ReadToEnd() | ConvertFrom-Json
$result = @{}
foreach ($path in $paths) {
    try {
        $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($path)
        if ($null -ne $icon) {
            $bmp = New-Object System.Drawing.Bitmap($icon.ToBitmap(), 32, 32)
            $ms  = New-Object System.IO.MemoryStream
            $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
            $result[$path] = [Convert]::ToBase64String($ms.ToArray())
            $ms.Dispose(); $bmp.Dispose(); $icon.Dispose()
        }
    } catch {}
}
$result | ConvertTo-Json -Compress
"#;

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let Ok(mut child) = cmd.spawn() else {
        return HashMap::new();
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(paths_json.as_bytes());
    }

    let Ok(output) = child.wait_with_output() else {
        return HashMap::new();
    };

    serde_json::from_slice(&output.stdout).unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Tray icon
            let show = MenuItem::with_id(app, "show", "Mostrar Flake", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Flake — Alt+Space")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => toggle_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(&tray.app_handle().clone());
                    }
                })
                .build(app)?;

            // Global shortcut: Alt+Space toggles the launcher
            app.handle().global_shortcut().on_shortcut(
                "Alt+Space",
                move |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_window(app);
                    }
                },
            )?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_apps, launch_app, get_icons])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

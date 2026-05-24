#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[tauri::command]
pub fn launch_app(app: tauri::AppHandle, path: String) -> Result<(), String> {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    let mut cmd = std::process::Command::new("explorer.exe");
    if path.contains('!') {
        cmd.arg(format!("shell:AppsFolder\\{path}"));
    } else {
        cmd.arg(&path);
    }
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.spawn().map_err(|e| e.to_string())?;
    Ok(())
}

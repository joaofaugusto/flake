use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use tauri::Manager;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[tauri::command]
pub async fn get_icons(app: tauri::AppHandle, paths: Vec<String>) -> HashMap<String, String> {
    let cache_path = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("icons_cache.json"));

    let mut cache: HashMap<String, String> = cache_path
        .as_ref()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|data: Vec<u8>| serde_json::from_slice::<HashMap<String, String>>(&data).ok())
        .unwrap_or_default();

    let missing: Vec<String> = paths
        .iter()
        .filter(|p| !cache.contains_key(*p))
        .cloned()
        .collect();

    if !missing.is_empty() {
        let fresh = extract_icons_via_ps(missing).await;
        cache.extend(fresh);

        if let Some(ref path) = cache_path {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string(&cache) {
                let _ = std::fs::write(path, json);
            }
        }
    }

    paths
        .into_iter()
        .filter_map(|p| cache.remove(&p).map(|v| (p, v)))
        .collect()
}

async fn extract_icons_via_ps(paths: Vec<String>) -> HashMap<String, String> {
    if paths.is_empty() {
        return HashMap::new();
    }
    let Ok(paths_json) = serde_json::to_string(&paths) else {
        return HashMap::new();
    };

    // Handles multiple path types from shell:AppsFolder:
    //   Win32  — absolute .exe/.lnk  → ExtractAssociatedIcon
    //   MSIX   — AUMID (contains '!') → AppxManifest logo PNG
    //   Other  — application refs, UNC, etc. → resolved via .lnk tracking or image fallback
    let script = r#"
Add-Type -AssemblyName System.Drawing
$paths  = [Console]::In.ReadToEnd() | ConvertFrom-Json
$allPkg = Get-AppxPackage
$result = @{}

foreach ($path in $paths) {
    try {
        if ($path.Contains('!')) {
            # --- MSIX / UWP branch (AUMID contains '!') ---
            $family = $path.Split('!')[0]
            $pkg    = $allPkg | Where-Object { $_.PackageFamilyName -eq $family } | Select-Object -First 1
            if ($null -eq $pkg) { continue }

            $mfPath = Join-Path $pkg.InstallLocation 'AppxManifest.xml'
            if (-not (Test-Path $mfPath)) { continue }
            [xml]$mf = Get-Content $mfPath -Encoding UTF8

            $appNode = $mf.Package.Applications.Application
            if ($appNode -is [array]) { $appNode = $appNode[0] }
            $logoRel = $null
            if ($null -ne $appNode) { $logoRel = $appNode.VisualElements.Square44x44Logo }
            if ([string]::IsNullOrEmpty($logoRel)) { $logoRel = $mf.Package.Properties.Logo }
            if ([string]::IsNullOrEmpty($logoRel)) { continue }

            $logoBase = Join-Path $pkg.InstallLocation $logoRel
            $logoDir  = Split-Path $logoBase -Parent
            $logoName = [System.IO.Path]::GetFileNameWithoutExtension($logoBase)
            $logoExt  = [System.IO.Path]::GetExtension($logoBase)

            # Prefer higher-resolution scale variants
            $candidates = @()
            if (Test-Path $logoBase) { $candidates += $logoBase }
            $scaled = Get-ChildItem -Path $logoDir -Filter "$logoName.scale-*$logoExt" -ErrorAction SilentlyContinue |
                Sort-Object { [int]($_.BaseName -replace '.*scale-', '') } -Descending
            foreach ($s in $scaled) { $candidates += $s.FullName }
            $logo = $candidates | Select-Object -First 1
            if ($null -eq $logo) { continue }

            $img = [System.Drawing.Image]::FromFile($logo)
            $bmp = New-Object System.Drawing.Bitmap($img, 32, 32)
            $ms  = New-Object System.IO.MemoryStream
            $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
            $result[$path] = [Convert]::ToBase64String($ms.ToArray())
            $ms.Dispose(); $bmp.Dispose(); $img.Dispose()
        } else {
            # --- Win32 / non-MSIX branch ---
            $resolvedPath = $path
            $icon = $null

            # Attempt 1: Direct ExtractAssociatedIcon on the original path
            try { $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($resolvedPath) } catch {}

            # Attempt 2: If path is a .lnk and icon extraction failed, resolve the shortcut target
            if ($null -eq $icon -and $resolvedPath -like '*.lnk') {
                try {
                    $wsh = New-Object -ComObject WScript.Shell
                    $sc  = $wsh.CreateShortcut($resolvedPath)
                    $target = $sc.TargetPath
                    if (-not [string]::IsNullOrWhiteSpace($target) -and (Test-Path $target)) {
                        $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($target)
                        if ($null -ne $icon) { $resolvedPath = $target }
                    }
                } catch {}
            }

            # Attempt 3: Final fallback — try loading the resolved path as an image directly
            if ($null -eq $icon) {
                try {
                    if (Test-Path $resolvedPath) {
                        $img = [System.Drawing.Image]::FromFile($resolvedPath)
                        $bmp = New-Object System.Drawing.Bitmap($img, 32, 32)
                        $ms  = New-Object System.IO.MemoryStream
                        $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
                        $result[$path] = [Convert]::ToBase64String($ms.ToArray())
                        $ms.Dispose(); $bmp.Dispose(); $img.Dispose()
                    }
                } catch {}
            } else {
                # Icon was extracted via Attempt 1 or 2 — convert to PNG
                $bmp = New-Object System.Drawing.Bitmap($icon.ToBitmap(), 32, 32)
                $ms  = New-Object System.IO.MemoryStream
                $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
                $result[$path] = [Convert]::ToBase64String($ms.ToArray())
                $ms.Dispose(); $bmp.Dispose(); $icon.Dispose()
            }
        }
    } catch {
        # Individual path failure — skip silently, other paths unaffected
    }
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

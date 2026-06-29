#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// On WSL2 the WebKitGTK DMABUF/Zink renderer crashes the app on launch
/// (`MESA: error: ZINK: failed to choose pdev` → `egl: failed to create dri2 screen`
/// → `Gdk-Message: Error flushing display: Broken pipe`). Force the safe software
/// path before the webview is created. We only do this when running under WSL and
/// never override an explicit choice from the user's environment.
#[cfg(target_os = "linux")]
fn apply_wsl_webkit_workarounds() {
    let is_wsl = std::fs::read_to_string("/proc/version")
        .map(|version| {
            let version = version.to_ascii_lowercase();
            version.contains("microsoft") || version.contains("wsl")
        })
        .unwrap_or(false);
    if !is_wsl {
        return;
    }
    for (key, value) in [
        ("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
        ("WEBKIT_DISABLE_COMPOSITING_MODE", "1"),
    ] {
        if std::env::var_os(key).is_none() {
            std::env::set_var(key, value);
        }
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    apply_wsl_webkit_workarounds();

    clia_local_lib::run();
}

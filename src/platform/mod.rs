//! OS-specific bits: window titles, input idle time, process check, start at login.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

/// A top-level window.
pub struct Window {
    /// App name: executable name without ".exe" on Windows (e.g. "chrome", "Figma"),
    /// application name on macOS (e.g. "Google Chrome", "Figma")
    pub owner: String,
    pub title: String,
}

/// Running from a build folder (`cargo run` / target/...)? Then don't touch login items.
pub fn is_dev_build() -> bool {
    cfg!(debug_assertions)
        || std::env::current_exe()
            .map(|p| {
                let p = p.to_string_lossy().replace('\\', "/");
                p.contains("/target/release/") || p.contains("/target/debug/")
            })
            .unwrap_or(false)
}

/// Start at login, unless the user turned it off from the tray menu.
pub fn sync_autostart() {
    if is_dev_build() {
        return;
    }
    if !autostart_opted_out() {
        // Re-register every launch so the entry follows the app if it was moved
        if let Err(e) = set_autostart(true) {
            eprintln!("[autostart] {e}");
        }
    }
}

pub fn toggle_autostart(enable: bool) -> std::io::Result<()> {
    set_autostart(enable)?;
    let marker = config_dir().join("autostart-disabled");
    if enable {
        let _ = std::fs::remove_file(marker);
    } else {
        std::fs::create_dir_all(config_dir())?;
        std::fs::write(marker, b"")?;
    }
    Ok(())
}

fn autostart_opted_out() -> bool {
    config_dir().join("autostart-disabled").exists()
}

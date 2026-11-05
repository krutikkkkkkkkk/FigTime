use super::Window;
use std::{io, path::PathBuf, process::Command};

use core_foundation::{
    base::{CFType, TCFType},
    dictionary::{CFDictionary, CFDictionaryRef},
    number::CFNumber,
    string::{CFString, CFStringRef},
};
use core_graphics::{
    access::ScreenCaptureAccess,
    window::{
        copy_window_info, kCGNullWindowID, kCGWindowLayer, kCGWindowListExcludeDesktopElements,
        kCGWindowListOptionAll, kCGWindowListOptionOnScreenOnly, kCGWindowName, kCGWindowOwnerName,
        kCGWindowOwnerPID, CGWindowListOption,
    },
};

const AGENT_LABEL: &str = "com.infinitylinkage.figtime";

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
}
const COMBINED_SESSION_STATE: i32 = 0;
const ANY_INPUT_EVENT: u32 = u32::MAX;

/// Normal (layer 0) windows with their owner's pid, front to back.
fn window_list(option: CGWindowListOption) -> Vec<(i64, Window)> {
    let Some(array) = copy_window_info(option | kCGWindowListExcludeDesktopElements, kCGNullWindowID) else {
        return Vec::new();
    };
    let key = |k: CFStringRef| unsafe { CFString::wrap_under_get_rule(k) };
    let (k_layer, k_pid, k_owner, k_name) = unsafe {
        (key(kCGWindowLayer), key(kCGWindowOwnerPID), key(kCGWindowOwnerName), key(kCGWindowName))
    };

    array
        .iter()
        .filter_map(|item| {
            let dict: CFDictionary<CFString, CFType> =
                unsafe { CFDictionary::wrap_under_get_rule(*item as CFDictionaryRef) };
            let number = |k: &CFString| dict.find(k).and_then(|v| v.downcast::<CFNumber>()).and_then(|n| n.to_i64());
            let string = |k: &CFString| {
                dict.find(k).and_then(|v| v.downcast::<CFString>()).map(|s| s.to_string()).unwrap_or_default()
            };
            if number(&k_layer)? != 0 {
                return None;
            }
            // Titles are empty without the Screen Recording permission
            Some((number(&k_pid)?, Window { owner: string(&k_owner), title: string(&k_name) }))
        })
        .collect()
}

fn frontmost_pid() -> Option<i64> {
    use objc2_app_kit::NSWorkspace;
    #[allow(unused_unsafe)]
    unsafe {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        Some(app.processIdentifier() as i64)
    }
}

pub fn active_window() -> Option<Window> {
    let windows = window_list(kCGWindowListOptionOnScreenOnly);
    let pid = frontmost_pid();
    let mut windows = windows.into_iter();
    match pid {
        Some(pid) => windows.find(|(p, _)| *p == pid).map(|(_, w)| w),
        None => windows.next().map(|(_, w)| w), // Front-most window as a fallback
    }
}

pub fn open_windows() -> Vec<Window> {
    // All windows, including minimized ones and other Spaces
    window_list(kCGWindowListOptionAll).into_iter().map(|(_, w)| w).collect()
}

/// Seconds since the last keyboard/mouse input anywhere on the system.
pub fn idle_seconds() -> u64 {
    let secs = unsafe { CGEventSourceSecondsSinceLastEventType(COMBINED_SESSION_STATE, ANY_INPUT_EVENT) };
    if secs.is_finite() && secs > 0.0 { secs as u64 } else { 0 }
}

pub fn figma_app_running() -> bool {
    Command::new("pgrep").args(["-x", "Figma"]).output().map(|o| o.status.success()).unwrap_or(false)
}

/// Window titles (= Figma file names) need the Screen Recording permission.
/// Asks once; macOS remembers the answer.
pub fn request_permissions() {
    let access = ScreenCaptureAccess;
    if !access.preflight() {
        access.request();
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

pub fn config_dir() -> PathBuf {
    home().join("Library/Application Support/FigTime")
}

fn agent_path() -> PathBuf {
    home().join(format!("Library/LaunchAgents/{AGENT_LABEL}.plist"))
}

pub fn autostart_enabled() -> bool {
    agent_path().exists()
}

pub(super) fn set_autostart(enable: bool) -> io::Result<()> {
    let path = agent_path();
    if !enable {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }

    let exe = std::env::current_exe()?;
    let exe = exe.to_string_lossy();
    // Running from the .dmg or a quarantined Downloads copy: the path won't survive
    if exe.starts_with("/Volumes/") || exe.contains("/AppTranslocation/") {
        return Err(io::Error::other("move FigTime to Applications to start it at login"));
    }

    // Launch the .app bundle through `open` when we're inside one
    let args: Vec<String> = match exe.find(".app/Contents/MacOS/") {
        Some(i) => vec!["/usr/bin/open".into(), exe[..i + 4].into()],
        None => vec![exe.into_owned()],
    };
    let args_xml: String = args.iter().map(|a| format!("<string>{}</string>", xml_escape(a))).collect();

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{AGENT_LABEL}</string>
  <key>ProgramArguments</key><array>{args_xml}</array>
  <key>RunAtLoad</key><true/>
  <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
"#
    );
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, plist)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

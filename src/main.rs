// No console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! FigTime: shows what you're designing in Figma on your Discord profile.
//!
//! Main thread: tray icon + menu (event loop). Worker thread: checks Figma every
//! 10 seconds and updates Discord, sending status text back to the tray.

mod discord;
mod figma;
mod platform;

use std::{
    fs::File,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIconBuilder,
};

use discord::Discord;
use figma::{Tracker, Update};

/// Discord application ("FigTime") that owns the name and Figma artwork
const DEFAULT_CLIENT_ID: &str = "1396898295594483753";
const CHECK_INTERVAL: Duration = Duration::from_secs(10);
const TRAY_ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tray.rgba"));
const TRAY_ICON_SIZE: u32 = 32;

enum UserEvent {
    Status(String),
    Menu(MenuEvent),
}

fn main() {
    // Only one FigTime at a time
    let Some(_lock) = single_instance_lock() else {
        eprintln!("FigTime is already running");
        return;
    };

    platform::request_permissions();
    platform::sync_autostart();

    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    #[cfg(target_os = "macos")]
    {
        // Menu bar only: no Dock icon
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
    }

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let status_item = MenuItem::new("Starting…", false, None);
    let autostart_item = CheckMenuItem::new("Start at login", !platform::is_dev_build(), platform::autostart_enabled(), None);
    let quit_item = MenuItem::new("Quit FigTime", true, None);
    let mut tray = None;
    let proxy = event_loop.create_proxy();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            // macOS needs the tray created once the event loop is running
            Event::NewEvents(StartCause::Init) => {
                let menu = Menu::new();
                let _ = menu.append_items(&[
                    &status_item,
                    &PredefinedMenuItem::separator(),
                    &autostart_item,
                    &quit_item,
                ]);
                let icon = Icon::from_rgba(TRAY_ICON.to_vec(), TRAY_ICON_SIZE, TRAY_ICON_SIZE).expect("tray icon");
                tray = TrayIconBuilder::new()
                    .with_icon(icon)
                    .with_tooltip("FigTime")
                    .with_menu(Box::new(menu))
                    .build()
                    .map_err(|e| eprintln!("[tray] {e}"))
                    .ok();

                let proxy = proxy.clone();
                thread::spawn(move || worker(proxy));
            }
            Event::UserEvent(UserEvent::Status(text)) => {
                status_item.set_text(&text);
                if let Some(tray) = &tray {
                    let _ = tray.set_tooltip(Some(format!("FigTime – {text}")));
                }
            }
            Event::UserEvent(UserEvent::Menu(event)) => {
                if event.id == *quit_item.id() {
                    // Discord clears the status when our connection closes
                    std::process::exit(0);
                } else if event.id == *autostart_item.id() {
                    let enable = autostart_item.is_checked();
                    if let Err(e) = platform::toggle_autostart(enable) {
                        eprintln!("[autostart] {e}");
                        autostart_item.set_checked(platform::autostart_enabled());
                    }
                }
            }
            _ => {}
        }
    });
}

/// Checks Figma every few seconds and keeps Discord in sync.
fn worker(proxy: EventLoopProxy<UserEvent>) {
    let client_id: &'static str = match std::env::var("CLIENT_ID") {
        Ok(id) if !id.is_empty() => Box::leak(id.into_boxed_str()),
        _ => DEFAULT_CLIENT_ID,
    };
    let idle_after = std::env::var("IDLE_AFTER_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(figma::DEFAULT_IDLE_AFTER_SECS);

    let mut discord = Discord::new(client_id);
    let mut tracker = Tracker::new(idle_after);
    let mut last_sent: Option<String> = None; // None = nothing shown in Discord
    let mut last_status = String::new();

    loop {
        let figma = figma::find(tracker.project.as_deref());
        let update = tracker.update(figma, platform::idle_seconds(), now_ms());

        if !discord.is_connected() && discord.connect().is_ok() {
            eprintln!("✅ Connected to Discord");
            last_sent = None; // A fresh connection shows nothing yet
        }

        let status = if !discord.is_connected() {
            "Waiting for Discord…".to_owned()
        } else {
            match update {
                Update::Show { activity, key } => {
                    if last_sent.as_deref() != Some(key.as_str()) {
                        match discord.set_activity(Some(&activity)) {
                            Ok(()) => {
                                eprintln!("[✔] {} ({})", activity.details, activity.state);
                                last_sent = Some(key);
                            }
                            Err(e) => {
                                eprintln!("[discord] {e}");
                                last_sent = None;
                            }
                        }
                    }
                    if discord.is_connected() { activity.details } else { "Waiting for Discord…".into() }
                }
                Update::Clear => {
                    if last_sent.is_some() {
                        let _ = discord.set_activity(None);
                        eprintln!("[✔] Figma closed - status cleared");
                        last_sent = None;
                    }
                    "Figma isn't open".to_owned()
                }
            }
        };

        if status != last_status {
            let _ = proxy.send_event(UserEvent::Status(status.clone()));
            last_status = status;
        }
        thread::sleep(CHECK_INTERVAL);
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Holds an exclusive lock on a file in the temp dir for the lifetime of the process.
fn single_instance_lock() -> Option<File> {
    let name = if platform::is_dev_build() { "figtime-dev.lock" } else { "figtime.lock" };
    let file = File::create(std::env::temp_dir().join(name)).ok()?;
    file.try_lock().ok()?;
    Some(file)
}

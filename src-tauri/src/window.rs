use crate::state::AppState;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

static LAST_TOGGLE: Mutex<Option<Instant>> = Mutex::new(None);

pub fn parse_hotkey(hotkey: &str) -> Result<Shortcut, String> {
    let hotkey = hotkey.trim();
    hotkey.parse::<Shortcut>().map_err(|_| {
        format!("Could not use the hotkey `{hotkey}`. Use a form like Ctrl+Shift+Space.")
    })
}

pub fn register_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let shortcut = parse_hotkey(hotkey)?;
    let shortcuts = app.global_shortcut();
    if let Err(err) = shortcuts.unregister_all() {
        eprintln!("hotkey unregister failed: {err}");
    }
    shortcuts
        .register(shortcut)
        .map_err(|err| format!("Could not register {hotkey}. Another app may already own it. {err}"))
}

pub fn hotkey_is_active(app: &AppHandle, hotkey: &str) -> bool {
    match parse_hotkey(hotkey) {
        Ok(shortcut) => app.global_shortcut().is_registered(shortcut),
        Err(_) => false,
    }
}

pub fn toggle_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    match window.is_visible() {
        Ok(true) => {
            let _ = window.hide();
        }
        _ => present_main(app),
    }
}

pub fn present_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

pub fn open_settings(app: &AppHandle) {
    present_main(app);
    let _ = app.emit("open-settings", ());
}

pub fn install_close_hides(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let hidden = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = hidden.hide();
        }
    });
}

pub fn install_tray(app: &AppHandle) -> Result<(), String> {
    let show = MenuItem::with_id(app, "show", "Show / Hide", true, None::<&str>)
        .map_err(|err| err.to_string())?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)
        .map_err(|err| err.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).map_err(|err| err.to_string())?;
    let menu = Menu::with_items(app, &[&show, &settings, &quit]).map_err(|err| err.to_string())?;
    let mut builder = TrayIconBuilder::with_id("prompt-rewriter")
        .menu(&menu)
        .tooltip("Prompt Rewriter")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => toggle_main(app),
            "settings" => open_settings(app),
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
                toggle_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon((*icon).clone());
    }
    builder.build(app).map_err(|err| err.to_string())?;
    Ok(())
}

pub fn on_shortcut(app: &AppHandle, state: ShortcutState) {
    if state != ShortcutState::Pressed || !should_toggle() {
        return;
    }
    toggle_main(app);
}

fn should_toggle() -> bool {
    let mut last = LAST_TOGGLE.lock().unwrap_or_else(|err| err.into_inner());
    let now = Instant::now();
    if last.is_some_and(|previous| now.duration_since(previous) < Duration::from_millis(350)) {
        return false;
    }
    *last = Some(now);
    true
}

pub fn remember_hotkey_error(app: &AppHandle, error: Option<String>) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut slot) = state.hotkey_error.lock() {
            *slot = error;
        }
    }
}

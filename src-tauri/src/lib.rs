mod commands;
mod files;
mod secrets;
mod state;
mod window;

use state::AppState;
use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            window::present_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        window::on_shortcut(app, event.state);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let config_dir = handle
                .path()
                .app_config_dir()
                .map_err(|err| format!("Could not find the config folder: {err}"))?;
            let templates_dir = config_dir.join("templates");
            std::fs::create_dir_all(&config_dir)
                .map_err(|err| format!("Could not create the config folder: {err}"))?;
            files::seed_templates(&templates_dir)?;
            let settings_path = config_dir.join("settings.json");
            let history_path = config_dir.join("history.json");
            let secret_path = config_dir.join("openrouter.key");
            let settings = files::load_settings(&settings_path);
            let history = files::load_history(&history_path);
            let http = rewriter_core::http_client().map_err(|err| err.to_string())?;
            let hotkey = settings.hotkey.clone();
            app.manage(AppState {
                http,
                config_dir,
                templates_dir,
                settings_path,
                history_path,
                secret_path,
                settings: std::sync::Mutex::new(settings),
                history: std::sync::Mutex::new(history),
                hotkey_error: std::sync::Mutex::new(None),
            });
            if let Err(err) = window::register_hotkey(&handle, &hotkey) {
                eprintln!("hotkey unavailable: {err}");
                window::remember_hotkey_error(&handle, Some(err));
            }
            if let Err(err) = window::install_tray(&handle) {
                eprintln!("tray unavailable: {err}");
            }
            window::install_close_hides(&handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::set_openrouter_key,
            commands::clear_openrouter_key,
            commands::list_backend_models,
            commands::list_templates,
            commands::read_template,
            commands::save_template,
            commands::reset_template,
            commands::open_templates_dir,
            commands::hide_window,
            commands::show_window,
            commands::rewrite,
            commands::list_history,
            commands::delete_history_entry,
            commands::clear_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running Prompt Rewriter");
}

#[cfg(test)]
mod hotkey_tests {
    use super::window::parse_hotkey;

    #[test]
    fn documented_shortcuts_parse() {
        for sample in ["Ctrl+Shift+Space", "Ctrl+Shift+C", "CommandOrControl+Shift+Space", "Alt+Space"] {
            parse_hotkey(sample).unwrap_or_else(|err| panic!("{sample}: {err}"));
        }
    }
}

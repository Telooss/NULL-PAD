//! Cote Rust de NULL/CONTROL : commandes Tauri et relais des pipes du moteur.
pub mod native;
pub mod pipes;
pub mod store;

use engine::config::{default_config, Config};
use engine::ipc::Response;
use std::collections::BTreeMap;
use tauri::Manager;

#[tauri::command]
fn get_config() -> Result<Config, String> {
    let path = engine::paths::config_path();
    if path.exists() {
        store::load(&path)
    } else {
        Ok(default_config())
    }
}

#[tauri::command]
fn validate_config(json: String) -> Result<(), String> {
    store::validate(&json)
}

#[tauri::command]
fn save_config(json: String) -> Result<(), String> {
    store::save_atomic(&engine::paths::config_path(), &json)
}

#[tauri::command]
fn native_keys() -> BTreeMap<String, String> {
    native::native_keys(native::KEYMAP)
}

// async : jusqu'a 3 s d'attente, hors du thread de la fenetre.
#[tauri::command(async)]
fn engine_status() -> Result<Response, String> {
    pipes::control("status")
}

#[tauri::command(async)]
fn test_obs() -> Result<Response, String> {
    pipes::control("test_obs")
}

#[tauri::command(async)]
fn test_midi() -> Result<Response, String> {
    pipes::control("test_midi")
}

#[tauri::command]
fn start_engine() -> Result<(), String> {
    pipes::start_engine()
}

#[tauri::command]
fn config_path() -> String {
    engine::paths::config_path().display().to_string()
}

pub fn run() {
    tauri::Builder::default()
        // En premier : une seconde instance ramene la fenetre puis quitte.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            validate_config,
            save_config,
            native_keys,
            engine_status,
            test_obs,
            test_midi,
            start_engine,
            config_path
        ])
        .setup(|app| {
            pipes::spawn_events_relay(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erreur au lancement de nullctl");
}

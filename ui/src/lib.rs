//! Cote Rust de NULL/CONTROL : commandes Tauri et relais des pipes du moteur.
pub mod native;
pub mod pipes;
pub mod settings;
pub mod store;
pub(crate) mod studio;

use engine::config::{default_config, Config};
use engine::ipc::Response;
use std::collections::BTreeMap;
use tauri::Manager;

/// Dossier parent de `nullctl` (et de l'ancien `NULL-PAD`).
fn config_base() -> std::path::PathBuf {
    let p = engine::paths::config_path();
    p.parent().and_then(|p| p.parent()).map(Into::into).unwrap_or_default()
}

/// Jamais un defaut quand l'ancienne config existe encore : il pourrait ecraser celle de l'utilisateur.
fn read_config(base: &std::path::Path) -> Result<Config, String> {
    let path = base.join(engine::paths::APP_DIR).join("config.json");
    if path.exists() {
        return store::load(&path);
    }
    let old = base.join(engine::paths::OLD_APP_DIR).join("config.json");
    if old.exists() {
        return Err(format!("Ancienne config non migrée : {}", old.display()));
    }
    Ok(default_config())
}

#[tauri::command]
fn get_config() -> Result<Config, String> {
    read_config(&config_base())
}

#[tauri::command]
fn validate_config(json: String, pad: Option<studio::model::PadModel>) -> Result<(), String> {
    store::validate(&json, pad.as_ref())
}

#[tauri::command]
fn save_config(json: String, pad: Option<studio::model::PadModel>) -> Result<(), String> {
    store::save_atomic(&engine::paths::config_path(), &json, pad.as_ref())
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

// async : l'enumeration WASAPI peut prendre un moment.
#[tauri::command(async)]
fn list_audio_outputs() -> Vec<String> {
    engine::sound::devices::output_names()
}

#[tauri::command(async)]
fn cable_present() -> bool {
    engine::sound::devices::cable_present()
}

#[tauri::command]
fn get_autostart() -> bool {
    engine::autostart::is_autostart()
}

#[tauri::command]
fn set_autostart(enabled: bool) -> Result<(), String> {
    settings::set_autostart(enabled)
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
        .manage(studio::commands::PadState::default())
        .invoke_handler(tauri::generate_handler![
            get_config,
            validate_config,
            save_config,
            native_keys,
            engine_status,
            test_obs,
            test_midi,
            start_engine,
            list_audio_outputs,
            cable_present,
            get_autostart,
            set_autostart,
            config_path,
            studio::commands::pad_status,
            studio::commands::pad_read,
            studio::commands::pad_cancel_unlock,
            studio::commands::pad_cached,
            studio::commands::pad_write,
            studio::commands::pad_discard_foreign,
            studio::commands::hid_catalog,
            studio::commands::pad_default_layer
        ])
        .setup(|app| {
            // Avant toute lecture de la config (comme le moteur).
            if let Err(e) = engine::paths::migrate_app_dir(&config_base()) {
                tracing::warn!("migration du dossier de config: {e}");
            }
            pipes::spawn_events_relay(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("erreur au lancement de nullctl");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmigrated_old_config_is_an_error_not_a_default() {
        let base = std::env::temp_dir().join(format!("nullctl-ui-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        assert_eq!(read_config(&base).unwrap(), default_config());
        let old = base.join(engine::paths::OLD_APP_DIR);
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("config.json"), "{}").unwrap();
        let e = read_config(&base).unwrap_err();
        assert!(e.starts_with("Ancienne config non migrée : "), "{e}");
        let _ = std::fs::remove_dir_all(&base);
    }
}

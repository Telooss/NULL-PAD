//! Commandes Tauri du pad (fines : tout le travail est dans `service`).
//!
//! Contrat d'erreur : l'`Err(String)` de `pad_read`, `pad_write`, `pad_discard_foreign` et `pad_cached`
//! commence par un code stable, puis `: `, puis le message français. Codes : `locked`, `cancelled`,
//! `foreign-unsaved`, `changed-elsewhere`, `disconnected`, `timeout`, `absent` (pas de port pad),
//! `guard`, `rejected`, `busy` (une autre commande tient le port).
//! Le port n'est jamais gardé entre deux commandes : ZMK Studio doit rester utilisable.
use super::cache;
use super::client::Client;
use super::hid::{self, KeyChoice};
use super::model::{self, PadBinding, PadModel};
use super::port::{find_pad_port, open_pad, Port};
use super::service::{self, PadRead};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

/// État géré : un seul client à la fois, et le drapeau d'annulation de l'attente de déverrouillage.
#[derive(Default)]
pub struct PadState {
    busy: Mutex<()>,
    cancel: Arc<AtomicBool>,
}

#[derive(Serialize)]
pub struct PadStatus {
    present: bool,
    locked: Option<bool>,
    serial: Option<String>,
    cache_at: Option<u64>,
}

/// Ouvre le port sous le verrou, remet l'annulation à zéro, rend le port à la sortie.
fn with_client<T>(
    state: &PadState,
    f: impl FnOnce(&mut Client<Box<dyn Port>>) -> Result<T, String>,
) -> Result<T, String> {
    let _guard = state
        .busy
        .try_lock()
        .map_err(|_| "busy: Pad occupé".to_string())?;
    state.cancel.store(false, Ordering::Relaxed);
    let name = find_pad_port().ok_or("absent: Pad non branché")?;
    let port =
        open_pad(&name).map_err(|e| format!("disconnected: Impossible d'ouvrir le pad : {e}"))?;
    f(&mut Client::new(port))
}

fn notify(app: &AppHandle) -> impl FnOnce() + '_ {
    || {
        let _ = app.emit("pad-unlock-needed", serde_json::json!({}));
    }
}

#[tauri::command(async)]
pub fn pad_status(state: State<PadState>) -> PadStatus {
    let probe = with_client(&state, |c| Ok(service::status(c)))
        .ok()
        .flatten();
    let cache_at = probe
        .as_ref()
        .and_then(|(_, s)| cache::load(&cache::path(), Some(s)))
        .map(|c| c.read_at);
    PadStatus {
        present: probe.is_some(),
        locked: probe.as_ref().map(|(l, _)| *l),
        serial: probe.map(|(_, s)| s),
        cache_at,
    }
}

#[tauri::command(async)]
pub fn pad_read(
    app: AppHandle,
    state: State<PadState>,
    wait_unlock_secs: u64,
) -> Result<PadRead, String> {
    with_client(&state, |c| {
        service::read(
            c,
            &cache::path(),
            Duration::from_secs(wait_unlock_secs),
            &state.cancel,
            notify(&app),
        )
    })
}

#[tauri::command]
pub fn pad_cancel_unlock(state: State<PadState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

/// Cache du pad branché (même numéro de série) ou, sans pad, dernier cache.
#[tauri::command(async)]
pub fn pad_cached(state: State<PadState>) -> Option<PadRead> {
    let serial = with_client(&state, |c| Ok(service::status(c)))
        .ok()
        .flatten()
        .map(|(_, s)| s);
    cache::load(&cache::path(), serial.as_deref()).map(|c| PadRead {
        model: c.model,
        max_name_len: c.max_name_len,
    })
}

#[tauri::command(async)]
pub fn pad_write(
    app: AppHandle,
    state: State<PadState>,
    expected: PadModel,
    target: PadModel,
) -> Result<PadRead, String> {
    with_client(&state, |c| {
        service::write(
            c,
            &cache::path(),
            &expected,
            &target,
            Duration::from_secs(60),
            &state.cancel,
            notify(&app),
        )
    })
}

#[tauri::command(async)]
pub fn pad_discard_foreign(app: AppHandle, state: State<PadState>) -> Result<(), String> {
    with_client(&state, |c| {
        service::discard_foreign(c, Duration::from_secs(60), &state.cancel, notify(&app))
    })
}

#[tauri::command]
pub fn hid_catalog() -> Vec<KeyChoice> {
    hid::catalog()
}

#[tauri::command]
pub fn pad_default_layer(profile: u8) -> Result<Vec<PadBinding>, String> {
    if !(1..=8).contains(&profile) {
        return Err(format!("guard: profil {profile} hors de 1 à 8"));
    }
    Ok(model::default_layer(profile))
}

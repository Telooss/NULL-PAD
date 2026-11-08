//! Commandes Tauri du pad (fines : tout le travail est dans `service`).
//!
//! Contrat d'erreur : l'`Err(String)` de `pad_read`, `pad_write` et `pad_discard_foreign`
//! commence par un code stable, puis `: `, puis le message français. Codes : `locked`, `cancelled`,
//! `foreign-unsaved`, `changed-elsewhere`, `disconnected`, `timeout`, `absent` (pas de port pad),
//! `guard`, `rejected`, `busy` (une autre commande tient le port).
//! `pad_status` et `pad_cached` ne rendent jamais d'erreur : occupé = `busy: true` / `None`.
//! Le port n'est jamais gardé entre deux commandes : ZMK Studio doit rester utilisable.
use super::cache;
use super::client::Client;
use super::hid::{self, KeyChoice};
use super::model::{self, PadBinding, PadModel};
use super::port::{find_pad_port, open_pad, Port};
use super::service::{self, PadRead};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
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
    /// Une autre commande tient le port : le pad est là, on ne l'a pas sondé.
    busy: bool,
}

const MAX_WAIT_SECS: u64 = 600;

/// `None` si le port est tenu par une autre commande ; un verrou empoisonné est récupéré.
fn try_lock(state: &PadState) -> Option<MutexGuard<'_, ()>> {
    match state.busy.try_lock() {
        Ok(g) => Some(g),
        Err(TryLockError::Poisoned(p)) => Some(p.into_inner()),
        Err(TryLockError::WouldBlock) => None,
    }
}

/// Ouvre le port sous le verrou, remet l'annulation à zéro, rend le port à la sortie.
fn with_client<T>(
    state: &PadState,
    f: impl FnOnce(&mut Client<Box<dyn Port>>) -> Result<T, String>,
) -> Result<T, String> {
    let _guard = try_lock(state).ok_or("busy: Pad occupé")?;
    state.cancel.store(false, Ordering::Relaxed);
    let name = find_pad_port().ok_or("absent: Pad non branché")?;
    let port =
        open_pad(&name).map_err(|e| format!("disconnected: Impossible d'ouvrir le pad : {e}"))?;
    let mut c = Client::new(port); // déclaré après le verrou : le port est rendu avant lui
    f(&mut c)
}

fn notify(app: &AppHandle) -> impl FnOnce() + '_ {
    || {
        let _ = app.emit("pad-unlock-needed", serde_json::json!({}));
    }
}

#[tauri::command(async)]
pub fn pad_status(state: State<PadState>) -> PadStatus {
    status_with(&state, || {
        with_client(&state, |c| Ok(service::status(c)))
            .ok()
            .flatten()
    })
}

/// `probe` n'est appelé que si le port est libre.
fn status_with(state: &PadState, probe: impl FnOnce() -> Option<(bool, String)>) -> PadStatus {
    if try_lock(state).is_none() {
        return PadStatus {
            present: true,
            locked: None,
            serial: None,
            cache_at: None,
            busy: true,
        };
    }
    let probe = probe();
    let cache_at = probe
        .as_ref()
        .and_then(|(_, s)| cache::load(&cache::path(), Some(s)))
        .map(|c| c.read_at);
    PadStatus {
        present: probe.is_some(),
        locked: probe.as_ref().map(|(l, _)| *l),
        serial: probe.map(|(_, s)| s),
        cache_at,
        busy: false,
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
            Duration::from_secs(wait_unlock_secs.min(MAX_WAIT_SECS)),
            &state.cancel,
            notify(&app),
        )
    })
}

#[tauri::command]
pub fn pad_cancel_unlock(state: State<PadState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

/// Cache du pad branché (même numéro de série) ou, sans pad, dernier cache. Occupé : `None`.
#[tauri::command(async)]
pub fn pad_cached(state: State<PadState>) -> Option<PadRead> {
    cached_with(&state, &cache::path(), || {
        with_client(&state, |c| Ok(service::status(c)))
            .ok()
            .flatten()
    })
}

fn cached_with(
    state: &PadState,
    path: &std::path::Path,
    probe: impl FnOnce() -> Option<(bool, String)>,
) -> Option<PadRead> {
    // Occupé : le numéro de série est inconnu, donc pas de cache (jamais sans contrôle du série).
    let _guard = try_lock(state)?;
    let serial = probe().map(|(_, s)| s);
    cache::load(path, serial.as_deref()).map(|c| PadRead {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::cache::Cache;

    #[test]
    fn busy_lock_means_present_and_unprobed() {
        let state = PadState::default();
        let _held = try_lock(&state).unwrap();
        let s = status_with(&state, || panic!("ne sonde pas un port occupé"));
        assert!(s.present && s.busy && s.locked.is_none() && s.serial.is_none());
        let dir = std::env::temp_dir().join(format!("nullctl-cmd-{}", std::process::id()));
        let path = dir.join("pad-cache.json");
        let c = Cache {
            serial: "aa".into(),
            read_at: 1,
            model: PadModel { layers: vec![] },
            max_name_len: 20,
        };
        cache::save(&path, &c).unwrap();
        assert!(cached_with(&state, &path, || panic!("occupé")).is_none());
        drop(_held);
        assert!(cached_with(&state, &path, || Some((false, "bb".into()))).is_none());
        assert!(cached_with(&state, &path, || Some((false, "aa".into()))).is_some());
        let s = status_with(&state, || Some((true, "aa".into())));
        assert!(s.present && !s.busy && s.locked == Some(true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn poisoned_lock_is_recovered_not_busy() {
        let state = Arc::new(PadState::default());
        let s2 = state.clone();
        let _ = std::thread::spawn(move || {
            let _g = s2.busy.lock().unwrap();
            panic!("empoisonne");
        })
        .join();
        assert!(try_lock(&state).is_some());
    }
}

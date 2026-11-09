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

/// Sondé toutes les 5 s par l'interface : énumération des ports seulement, le port n'est jamais ouvert
/// (une requête RPC relancerait le minuteur de reverrouillage et gênerait ZMK Studio).
/// `locked` et `serial` restent `None` : seules les commandes qui ouvrent le port les connaissent.
#[derive(Serialize)]
pub struct PadStatus {
    present: bool,
    locked: Option<bool>,
    serial: Option<String>,
    /// Une autre commande tient le port : le pad est là.
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
    run_on_port(f)
}

/// Ouvre le port et exécute `f`, SANS toucher au verrou : l'appelant le tient déjà.
fn run_on_port<T>(
    f: impl FnOnce(&mut Client<Box<dyn Port>>) -> Result<T, String>,
) -> Result<T, String> {
    let name = find_pad_port().ok_or("absent: Pad non branché")?;
    let port =
        open_pad(&name).map_err(|e| format!("disconnected: Impossible d'ouvrir le pad : {e}"))?;
    let mut c = Client::new(port); // déclaré après le verrou : le port est rendu avant lui
    f(&mut c)
}

/// `service::UNLOCK_NEEDED` / `UNLOCKED` -> événement Tauri du même nom.
fn notify(app: &AppHandle) -> impl FnMut(&'static str) + '_ {
    |event| {
        let _ = app.emit(event, serde_json::json!({}));
    }
}

#[tauri::command(async)] // l'énumération des ports peut prendre quelques dizaines de ms
pub fn pad_status(state: State<PadState>) -> PadStatus {
    status_with(&state, || find_pad_port().is_some())
}

/// Occupé (une commande tient le port) : présent. Sinon, `present` vient de l'énumération des ports.
fn status_with(state: &PadState, present: impl FnOnce() -> bool) -> PadStatus {
    let busy = matches!(state.busy.try_lock(), Err(TryLockError::WouldBlock));
    PadStatus {
        present: busy || present(),
        locked: None,
        serial: None,
        busy,
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
        run_on_port(|c| Ok(service::status(c))).ok().flatten()
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
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Le statut n'ouvre jamais le port : présence = énumération, ni verrou ni numéro de série.
    #[test]
    fn status_only_enumerates() {
        let state = PadState::default();
        let s = status_with(&state, || true);
        assert!(s.present && !s.busy && s.locked.is_none() && s.serial.is_none());
        assert!(!status_with(&state, || false).present);
        assert!(try_lock(&state).is_some()); // le verrou n'est pas garde
    }

    /// Régression : le verrou est tenu pendant la sonde de `pad_cached` (une sonde qui le reprendrait verrait
    /// WouldBlock et perdrait le numéro de série) ; la sonde réelle (`run_on_port`) ne le touche pas.
    #[test]
    fn lock_is_held_once_during_the_probe() {
        let state = PadState::default();
        let held = |state: &PadState| {
            assert!(
                matches!(state.busy.try_lock(), Err(TryLockError::WouldBlock)),
                "verrou non tenu pendant la sonde"
            )
        };
        let path = std::env::temp_dir().join("nullctl-cmd-absent-xyz.json");
        let probed = std::cell::Cell::new(false);
        assert!(cached_with(&state, &path, || {
            held(&state);
            probed.set(true);
            None
        })
        .is_none());
        assert!(probed.get());
        assert!(try_lock(&state).is_some()); // libéré ensuite
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

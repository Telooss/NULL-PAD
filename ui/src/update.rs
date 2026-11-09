//! Mise a jour de nullctl (tauri-plugin-updater), pilotee depuis Rust : la fenetre n'appelle que
//! `update_check` et `update_install`, jamais l'API JS du plugin (aucune permission `updater:*`).
//!
//! Config : `plugins.updater` de la config Tauri fusionnee, lue par le plugin au demarrage.
//! `tauri.conf.json` porte l'endpoint et une pubkey vide (le plugin exige le champ, sinon l'appli ne
//! demarre pas) ; la vraie cle arrive par `tauri.bundle.json` (Task 3). Pubkey vide = build de
//! developpement : `update_check` renvoie `None` sans rien demander au reseau.
//!
//! Sous Windows, `Update::install` lance l'installeur NSIS puis quitte le processus ; l'installeur
//! relance nullctl, et ses hooks (Task 3) relancent le moteur arrete ici.

use crate::pipes;
use serde::Serialize;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::{Error, Update, UpdaterExt};

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    version: String,
    notes: Option<String>,
    date: Option<String>,
}

/// `network`, `config` ou `signature` selon l'erreur du plugin.
fn prefix(e: &Error) -> &'static str {
    match e {
        Error::EmptyEndpoints
        | Error::UnsupportedArch
        | Error::UnsupportedOs
        | Error::UrlParse(_)
        | Error::InsecureTransportProtocol
        | Error::TargetNotFound(_)
        | Error::TargetsNotFound(_) => "config",
        Error::Minisign(_)
        | Error::Base64(_)
        | Error::SignatureUtf8(_)
        | Error::SignedVersionMismatch { .. }
        | Error::MissingSignedVersion => "signature",
        _ => "network",
    }
}

fn prefixed(e: Error) -> String {
    format!("{}: {e}", prefix(&e))
}

/// Mise a jour plus recente que cette version, ou `None` (rien de neuf, aucune release, build de dev).
async fn find(app: &AppHandle) -> Result<Option<Update>, String> {
    let pubkey = app
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u["pubkey"].as_str());
    if pubkey.unwrap_or_default().is_empty() {
        tracing::info!("mise a jour: pas de cle publique (build de developpement)");
        return Ok(None);
    }
    match app.updater().map_err(prefixed)?.check().await {
        // latest.json en 404 (aucune release publiee) : pas une panne.
        Err(Error::ReleaseNotFound) => {
            tracing::info!("mise a jour: aucune release publiee");
            Ok(None)
        }
        r => r.map_err(prefixed),
    }
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    Ok(find(&app).await?.map(|u| UpdateInfo {
        date: u
            .raw_json
            .get("pub_date")
            .and_then(|d| d.as_str())
            .map(String::from),
        notes: u.body,
        version: u.version,
    }))
}

/// Telecharge (signature verifiee par le plugin), arrete le moteur, installe et relance l'appli.
/// Le moteur ne s'arrete qu'une fois le fichier telecharge et verifie.
#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), String> {
    let update = find(&app)
        .await?
        .ok_or("network: mise a jour introuvable")?;
    let mut downloaded = 0u64;
    let bytes = update
        .download(
            |n, total| {
                downloaded += n as u64;
                let _ = app.emit(
                    "update-progress",
                    serde_json::json!({ "downloaded": downloaded, "total": total }),
                );
            },
            || {},
        )
        .await
        .map_err(prefixed)?;
    // Bloquant (pipe, attente de 5 s, installeur) : hors du runtime async.
    tauri::async_runtime::spawn_blocking(move || {
        stop_engine_then(
            || {
                update.install(&bytes).map_err(|e| match prefix(&e) {
                    "signature" => prefixed(e),
                    _ => format!("install: {e}"),
                })
            },
            &mut Engine,
        )
    })
    .await
    .map_err(|e| format!("install: {e}"))??;
    app.restart()
}

/// Arret et relance du moteur, separes pour tester l'ordre avec un faux moteur.
pub trait EngineCtl {
    /// Arrete le moteur : `Ok(true)` s'il tournait, `Ok(false)` s'il etait deja arrete.
    fn stop(&mut self) -> Result<bool, String>;
    fn start(&mut self) -> Result<(), String>;
}

/// Arrete le moteur puis installe ; si l'installation echoue, relance le moteur s'il tournait.
/// Un moteur qui refuse de s'arreter annule l'installation (`engine:`).
pub fn stop_engine_then(
    install: impl FnOnce() -> Result<(), String>,
    engine: &mut impl EngineCtl,
) -> Result<(), String> {
    let was_running = engine.stop().map_err(|e| format!("engine: {e}"))?;
    let r = install();
    if r.is_err() && was_running {
        if let Err(e) = engine.start() {
            tracing::warn!("relance du moteur apres l'echec de la mise a jour: {e}");
        }
    }
    r
}

struct Engine;

impl EngineCtl for Engine {
    fn stop(&mut self) -> Result<bool, String> {
        match pipes::control("quit") {
            Err(e) if e == pipes::ENGINE_DOWN => return Ok(false),
            Err(e) => return Err(e),
            Ok(r) if !r.ok => return Err(r.error.unwrap_or_else(|| "quit refuse".into())),
            Ok(_) => {}
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while !pipes::engine_gone() {
            if Instant::now() >= deadline {
                return Err("le moteur ne s'est pas arrete".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(true)
    }

    fn start(&mut self) -> Result<(), String> {
        pipes::start_engine()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Faux moteur ; `log` est partage avec l'installation pour verifier l'ordre des appels.
    struct Fake<'a> {
        running: bool,
        refuse: bool,
        log: &'a RefCell<Vec<&'static str>>,
    }

    impl EngineCtl for Fake<'_> {
        fn stop(&mut self) -> Result<bool, String> {
            self.log.borrow_mut().push("stop");
            if self.refuse {
                return Err("le moteur ne s'est pas arrete".into());
            }
            Ok(std::mem::replace(&mut self.running, false))
        }
        fn start(&mut self) -> Result<(), String> {
            self.log.borrow_mut().push("start");
            self.running = true;
            Ok(())
        }
    }

    /// Lance stop_engine_then avec une installation qui reussit ou non -> (resultat, appels).
    fn run(running: bool, refuse: bool, ok: bool) -> (Result<(), String>, Vec<&'static str>) {
        let log = RefCell::new(Vec::new());
        let mut e = Fake {
            running,
            refuse,
            log: &log,
        };
        let r = stop_engine_then(
            || {
                log.borrow_mut().push("install");
                if ok {
                    Ok(())
                } else {
                    Err("install: refuse".into())
                }
            },
            &mut e,
        );
        (r, log.into_inner())
    }

    #[test]
    fn running_engine_is_stopped_before_install() {
        assert_eq!(run(true, false, true), (Ok(()), vec!["stop", "install"]));
    }

    #[test]
    fn failed_install_restarts_a_running_engine() {
        assert_eq!(
            run(true, false, false),
            (
                Err("install: refuse".into()),
                vec!["stop", "install", "start"]
            )
        );
    }

    #[test]
    fn stopped_engine_is_never_started() {
        assert_eq!(run(false, false, true), (Ok(()), vec!["stop", "install"]));
        assert_eq!(
            run(false, false, false),
            (Err("install: refuse".into()), vec!["stop", "install"])
        );
    }

    #[test]
    fn engine_refusing_to_stop_cancels_install() {
        let (r, log) = run(true, true, true);
        assert!(r.as_ref().unwrap_err().starts_with("engine: "), "{r:?}");
        assert_eq!(log, vec!["stop"]);
    }

    #[test]
    fn plugin_errors_get_a_prefix() {
        assert!(prefixed(Error::EmptyEndpoints).starts_with("config: "));
        assert!(prefixed(Error::MissingSignedVersion).starts_with("signature: "));
        assert!(prefixed(Error::Network("coupe".into())).starts_with("network: "));
    }
}

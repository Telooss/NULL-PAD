//! Mise a jour de nullctl (tauri-plugin-updater), pilotee depuis Rust : la fenetre n'appelle que
//! `update_check` et `update_install`, jamais l'API JS du plugin (aucune permission `updater:*`).
//!
//! Config : `plugins.updater` de la config Tauri fusionnee, lue par le plugin au demarrage.
//! `tauri.conf.json` porte l'endpoint et une pubkey vide (le plugin exige le champ, sinon l'appli ne
//! demarre pas) ; la vraie cle arrive par `tauri.bundle.json` (Task 3). Pubkey vide = build de
//! developpement : `update_check` renvoie `None` sans rien demander au reseau.
//!
//! Sous Windows, `Update::install` lance l'installeur NSIS puis quitte le processus (`app.restart()`
//! n'est pas atteint) ; l'installeur relance nullctl. Le moteur arrete ici est relance par la nouvelle
//! version au demarrage, grace au marqueur `engine-restart` pose juste avant l'installation.

use crate::pipes;
use serde::Serialize;
use std::path::{Path, PathBuf};
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
    // Delais de connexion et de lecture (verification et telechargement) : un reseau bloque echoue.
    let updater = app
        .updater_builder()
        .configure_client(|c| {
            c.connect_timeout(Duration::from_secs(15))
                .read_timeout(Duration::from_secs(30))
        })
        .build()
        .map_err(prefixed)?;
    match updater.check().await {
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

/// Pas de progression : le pourcentage entier, ou les Mo sans taille connue.
fn progress_step(downloaded: u64, total: Option<u64>) -> u64 {
    match total {
        Some(t) if t > 0 => downloaded * 100 / t,
        _ => downloaded / 1_000_000,
    }
}

/// Telecharge (signature verifiee par le plugin), arrete le moteur, installe et relance l'appli.
/// Le moteur ne s'arrete qu'une fois le fichier telecharge et verifie. `expected_version` : la
/// version affichee ; si latest.json a change entre-temps, rien n'est installe.
#[tauri::command]
pub async fn update_install(app: AppHandle, expected_version: String) -> Result<(), String> {
    let update = find(&app)
        .await?
        .ok_or("network: mise a jour introuvable")?;
    if update.version != expected_version {
        return Err("install: version changée".into());
    }
    let mut downloaded = 0u64;
    let mut last = None;
    let bytes = update
        .download(
            |n, total| {
                downloaded += n as u64;
                let step = Some(progress_step(downloaded, total));
                if step != last {
                    last = step;
                    let _ = app.emit(
                        "update-progress",
                        serde_json::json!({ "downloaded": downloaded, "total": total }),
                    );
                }
            },
            || {},
        )
        .await
        .map_err(prefixed)?;
    let dir = marker_dir();
    // Bloquant (pipe, attentes, installeur) : hors du runtime async.
    tauri::async_runtime::spawn_blocking(move || {
        stop_engine_then(
            |was_running| {
                if was_running {
                    mark_engine_restart(&dir)
                        .map_err(|e| format!("install: marqueur de relance: {e}"))?;
                }
                let r = update.install(&bytes).map_err(|e| match prefix(&e) {
                    "signature" => prefixed(e),
                    _ => format!("install: {e}"),
                });
                if r.is_err() {
                    take_engine_restart(&dir); // relance faite par stop_engine_then
                }
                r
            },
            &mut Engine,
        )
    })
    .await
    .map_err(|e| format!("install: {e}"))??;
    app.restart()
}

const MARKER: &str = "engine-restart";

/// Dossier du marqueur : celui de config.json.
pub fn marker_dir() -> PathBuf {
    let p = engine::paths::config_path();
    p.parent().map(Into::into).unwrap_or_default()
}

/// Pose le marqueur : le moteur tournait avant l'installation.
pub fn mark_engine_restart(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join(MARKER), b"")
}

/// Retire le marqueur ; true s'il existait (le moteur est alors a relancer).
pub fn take_engine_restart(dir: &Path) -> bool {
    std::fs::remove_file(dir.join(MARKER)).is_ok()
}

/// Echec de l'arret du moteur.
pub enum Stop {
    /// `quit` refuse ou serveur inconnu : le moteur tourne toujours.
    Refused(String),
    /// `quit` envoye, mais pas de sortie a temps : le moteur peut encore s'arreter.
    Partial(String),
}

/// Attente supplementaire apres un arret partiel, avant de relancer le moteur.
const PARTIAL_WAIT: Duration = Duration::from_secs(15);

/// Arret et relance du moteur, separes pour tester l'ordre avec un faux moteur.
pub trait EngineCtl {
    /// Arrete le moteur : `Ok(true)` s'il tournait, `Ok(false)` s'il etait deja arrete.
    fn stop(&mut self) -> Result<bool, Stop>;
    /// Attend la sortie du moteur (`max` au plus).
    fn wait_gone(&mut self, max: Duration);
    fn start(&mut self) -> Result<(), String>;
}

fn restart(engine: &mut impl EngineCtl) {
    if let Err(e) = engine.start() {
        tracing::warn!("relance du moteur apres l'echec de la mise a jour: {e}");
    }
}

/// Arrete le moteur puis installe (`install(etait_lance)`) ; si l'installation echoue, relance le
/// moteur s'il tournait. Un moteur qui ne s'arrete pas annule l'installation (`engine:`) ; s'il a
/// recu `quit`, il est relance une fois sorti, pour ne jamais rester arrete.
pub fn stop_engine_then(
    install: impl FnOnce(bool) -> Result<(), String>,
    engine: &mut impl EngineCtl,
) -> Result<(), String> {
    let was_running = match engine.stop() {
        Ok(r) => r,
        Err(Stop::Refused(e)) => return Err(format!("engine: {e}")),
        Err(Stop::Partial(e)) => {
            engine.wait_gone(PARTIAL_WAIT);
            // Toujours la apres l'attente : le mutex du moteur fait sortir la nouvelle instance.
            restart(engine);
            return Err(format!("engine: {e}"));
        }
    };
    let r = install(was_running);
    if r.is_err() && was_running {
        restart(engine);
    }
    r
}

struct Engine;

impl EngineCtl for Engine {
    fn stop(&mut self) -> Result<bool, Stop> {
        match pipes::control("quit") {
            Err(e) if e == pipes::ENGINE_DOWN => return Ok(false),
            Err(e) if e == "serveur du pipe non reconnu" => return Err(Stop::Refused(e)),
            // Delai ou pipe coupe : le moteur a peut-etre recu `quit`.
            Err(e) => return Err(Stop::Partial(e)),
            Ok(r) if !r.ok => {
                return Err(Stop::Refused(
                    r.error.unwrap_or_else(|| "quit refuse".into()),
                ))
            }
            Ok(_) => {}
        }
        self.wait_gone(Duration::from_secs(5));
        if pipes::engine_gone() {
            Ok(true)
        } else {
            Err(Stop::Partial("le moteur ne s'est pas arrete".into()))
        }
    }

    fn wait_gone(&mut self, max: Duration) {
        let deadline = Instant::now() + max;
        while !pipes::engine_gone() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn start(&mut self) -> Result<(), String> {
        pipes::start_engine()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Clone, Copy)]
    enum Mode {
        Stops,
        Refuses,
        StopsHalfway,
    }

    /// Faux moteur ; `log` est partage avec l'installation pour verifier l'ordre des appels.
    struct Fake<'a> {
        running: bool,
        mode: Mode,
        log: &'a RefCell<Vec<&'static str>>,
    }

    impl EngineCtl for Fake<'_> {
        fn stop(&mut self) -> Result<bool, Stop> {
            self.log.borrow_mut().push("stop");
            match self.mode {
                Mode::Stops => Ok(std::mem::replace(&mut self.running, false)),
                Mode::Refuses => Err(Stop::Refused("quit refuse".into())),
                Mode::StopsHalfway => Err(Stop::Partial("le moteur ne s'est pas arrete".into())),
            }
        }
        fn wait_gone(&mut self, _: Duration) {
            self.log.borrow_mut().push("wait");
            self.running = false;
        }
        fn start(&mut self) -> Result<(), String> {
            self.log.borrow_mut().push("start");
            self.running = true;
            Ok(())
        }
    }

    /// Lance stop_engine_then avec une installation qui reussit ou non -> (resultat, appels).
    /// L'installation note si le moteur tournait (marqueur de relance).
    fn run(running: bool, mode: Mode, ok: bool) -> (Result<(), String>, Vec<&'static str>) {
        let log = RefCell::new(Vec::new());
        let mut e = Fake {
            running,
            mode,
            log: &log,
        };
        let r = stop_engine_then(
            |was_running| {
                log.borrow_mut().push(if was_running {
                    "install+mark"
                } else {
                    "install"
                });
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
        assert_eq!(
            run(true, Mode::Stops, true),
            (Ok(()), vec!["stop", "install+mark"])
        );
    }

    #[test]
    fn failed_install_restarts_a_running_engine() {
        assert_eq!(
            run(true, Mode::Stops, false),
            (
                Err("install: refuse".into()),
                vec!["stop", "install+mark", "start"]
            )
        );
    }

    #[test]
    fn stopped_engine_is_never_started() {
        assert_eq!(
            run(false, Mode::Stops, true),
            (Ok(()), vec!["stop", "install"])
        );
        assert_eq!(
            run(false, Mode::Stops, false),
            (Err("install: refuse".into()), vec!["stop", "install"])
        );
    }

    #[test]
    fn engine_refusing_to_stop_cancels_install() {
        let (r, log) = run(true, Mode::Refuses, true);
        assert!(r.as_ref().unwrap_err().starts_with("engine: "), "{r:?}");
        assert_eq!(log, vec!["stop"]);
    }

    #[test]
    fn half_stopped_engine_is_restarted_without_install() {
        let (r, log) = run(true, Mode::StopsHalfway, true);
        assert!(r.as_ref().unwrap_err().starts_with("engine: "), "{r:?}");
        assert_eq!(log, vec!["stop", "wait", "start"]);
    }

    #[test]
    fn restart_marker_roundtrip() {
        let dir = std::env::temp_dir()
            .join(format!("nullctl-marker-{}", std::process::id()))
            .join("nullctl");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!take_engine_restart(&dir));
        mark_engine_restart(&dir).unwrap(); // cree le dossier manquant
        assert!(take_engine_restart(&dir));
        assert!(!take_engine_restart(&dir)); // une seule relance
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn progress_step_changes_per_whole_percent() {
        assert_eq!(progress_step(0, Some(1000)), 0);
        assert_eq!(progress_step(9, Some(1000)), 0);
        assert_eq!(progress_step(10, Some(1000)), 1);
        assert_eq!(progress_step(1000, Some(1000)), 100);
        // Taille inconnue : un pas par Mo.
        assert_eq!(progress_step(2_500_000, None), 2);
        assert_eq!(progress_step(5, Some(0)), 0);
    }

    #[test]
    fn plugin_errors_get_a_prefix() {
        assert!(prefixed(Error::EmptyEndpoints).starts_with("config: "));
        assert!(prefixed(Error::MissingSignedVersion).starts_with("signature: "));
        assert!(prefixed(Error::Network("coupe".into())).starts_with("network: "));
    }
}

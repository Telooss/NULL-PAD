//! Service pad : lecture, écriture transactionnelle et cache, sans Tauri (testable avec `FakePad`).
use super::cache::{self, Cache};
use super::client::{Client, StudioError};
use super::model::{self, PadModel};
use super::port::Port;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct PadRead {
    pub model: PadModel,
    pub max_name_len: usize,
}

/// Code stable + message français : `locked: …`.
pub fn err(e: StudioError) -> String {
    let code = match e {
        StudioError::Locked => "locked",
        StudioError::Cancelled => "cancelled",
        StudioError::Disconnected => "disconnected",
        StudioError::Timeout => "timeout",
        StudioError::Rejected(_) | StudioError::Decode(_) => "rejected",
    };
    format!("{code}: {e}")
}

fn ensure_unlocked<P: Port>(
    c: &mut Client<P>,
    wait: Duration,
    cancel: &AtomicBool,
    on_unlock_needed: impl FnOnce(),
) -> Result<(), String> {
    if c.lock_state().map_err(err)? {
        on_unlock_needed();
        c.wait_unlocked(wait, cancel).map_err(err)?;
    }
    Ok(())
}

fn snapshot<P: Port>(c: &mut Client<P>) -> Result<PadRead, String> {
    let behaviors = c.behaviors().map_err(err)?;
    let km = c.keymap().map_err(err)?;
    Ok(PadRead {
        model: model::decode(&km, &behaviors),
        max_name_len: km.max_layer_name_length as usize,
    })
}

fn serial<P: Port>(c: &mut Client<P>) -> Result<String, String> {
    let info = c.device_info().map_err(err)?;
    Ok(info
        .serial_number
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// `(verrouillé, numéro de série)` sans jamais demander de déverrouillage.
pub fn status<P: Port>(c: &mut Client<P>) -> Option<(bool, String)> {
    let serial = serial(c).ok()?;
    Some((c.lock_state().ok()?, serial))
}

/// Déverrouille au besoin, refuse les changements non enregistrés d'un autre outil, lit et met le cache à jour.
pub fn read<P: Port>(
    c: &mut Client<P>,
    cache_path: &Path,
    wait: Duration,
    cancel: &AtomicBool,
    on_unlock_needed: impl FnOnce(),
) -> Result<PadRead, String> {
    let serial = serial(c)?;
    ensure_unlocked(c, wait, cancel, on_unlock_needed)?;
    check_clean(c)?;
    let r = snapshot(c)?;
    store(cache_path, &serial, &r);
    Ok(r)
}

fn check_clean<P: Port>(c: &mut Client<P>) -> Result<(), String> {
    if c.check_unsaved().map_err(err)? {
        return Err(
            "foreign-unsaved: Le pad a des changements non enregistrés venus d'un autre outil"
                .into(),
        );
    }
    Ok(())
}

fn store(path: &Path, serial: &str, r: &PadRead) {
    let read_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let cache = Cache {
        serial: serial.into(),
        read_at,
        model: r.model.clone(),
        max_name_len: r.max_name_len,
    };
    if let Err(e) = cache::save(path, &cache) {
        tracing::warn!("cache du pad: {e}");
    }
}

/// Écrit `target` : vérifie que le pad n'a pas bougé depuis `expected`, applique le diff, enregistre.
/// Au premier échec après un `set_*`, annule tout (`discard`).
pub fn write<P: Port>(
    c: &mut Client<P>,
    cache_path: &Path,
    expected: &PadModel,
    target: &PadModel,
    wait: Duration,
    cancel: &AtomicBool,
    on_unlock_needed: impl FnOnce(),
) -> Result<PadRead, String> {
    let serial = serial(c)?;
    ensure_unlocked(c, wait, cancel, on_unlock_needed)?;
    check_clean(c)?;
    let behaviors = c.behaviors().map_err(err)?;
    let km = c.keymap().map_err(err)?;
    let fresh = model::decode(&km, &behaviors);
    if &fresh != expected {
        return Err("changed-elsewhere: Le pad a changé depuis ta dernière lecture".into());
    }
    model::guard(target, km.max_layer_name_length as usize).map_err(|m| format!("guard: {m}"))?;
    // Tout encoder avant le premier set_* : un binding invalide n'écrit rien.
    let mut ops = Vec::new();
    for ch in model::diff(&fresh, target) {
        let op = match (&ch.name, &ch.binding, ch.pos) {
            (Some(n), _, _) => Op::Name(ch.layer_id, n.clone()),
            (_, Some(b), Some(pos)) => Op::Bind(
                ch.layer_id,
                pos as i32,
                model::encode(b, &behaviors).map_err(|m| format!("guard: {m}"))?,
            ),
            _ => continue,
        };
        ops.push(op);
    }
    let apply = |c: &mut Client<P>| -> Result<(), StudioError> {
        for op in ops {
            match op {
                Op::Name(id, n) => c.set_layer_name(id, &n)?,
                Op::Bind(id, pos, b) => c.set_binding(id, pos, b)?,
            }
        }
        c.save()
    };
    if let Err(e) = apply(c) {
        let _ = c.discard();
        return Err(err(e));
    }
    let r = snapshot(c)?;
    store(cache_path, &serial, &r);
    Ok(r)
}

enum Op {
    Name(u32, String),
    Bind(u32, i32, super::client::BehaviorBinding),
}

/// Jette les changements non enregistrés d'un autre outil (après confirmation côté interface).
pub fn discard_foreign<P: Port>(
    c: &mut Client<P>,
    wait: Duration,
    cancel: &AtomicBool,
    on_unlock_needed: impl FnOnce(),
) -> Result<(), String> {
    ensure_unlocked(c, wait, cancel, on_unlock_needed)?;
    c.discard().map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::fake::FakePad;
    use crate::studio::model::PadBinding;
    use std::io::{Read, Write};
    use std::sync::atomic::AtomicBool;

    const W: Duration = Duration::from_secs(5);

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("nullctl-svc-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d.join("pad-cache.json")
    }

    fn client(f: FakePad) -> Client<FakePad> {
        Client::new(f)
    }

    fn read_ok(c: &mut Client<FakePad>, p: &Path) -> PadRead {
        read(c, p, W, &AtomicBool::new(false), || {}).unwrap()
    }

    fn do_write(
        c: &mut Client<FakePad>,
        p: &Path,
        e: &PadModel,
        t: &PadModel,
    ) -> Result<PadRead, String> {
        write(c, p, e, t, W, &AtomicBool::new(false), || {})
    }

    #[test]
    fn locked_pad_is_unlocked_with_one_callback() {
        let mut pad = FakePad::new();
        pad.locked = true;
        pad.unlock_after = Some((3, false));
        let mut c = client(pad);
        let p = tmp("unlock");
        let mut calls = 0;
        let r = read(&mut c, &p, W, &AtomicBool::new(false), || calls += 1).unwrap();
        assert_eq!(calls, 1);
        assert_eq!(r.model.layers.len(), 8);
        assert_eq!(r.max_name_len, 20);
        let cached = cache::load(&p, Some("deadbeef")).unwrap();
        assert_eq!(cached.model, r.model);
        assert!(cache::load(&p, Some("00")).is_none());
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn locked_pad_can_be_cancelled() {
        let mut pad = FakePad::new();
        pad.locked = true;
        let mut c = client(pad);
        let cancel = AtomicBool::new(true);
        let e = read(&mut c, &tmp("cancel"), W, &cancel, || {}).unwrap_err();
        assert!(e.starts_with("cancelled: "), "{e}");
    }

    #[test]
    fn foreign_unsaved_refuses_read_and_write() {
        let mut pad = FakePad::new();
        pad.unsaved = true;
        let mut c = client(pad);
        let p = tmp("foreign");
        let e = read(&mut c, &p, W, &AtomicBool::new(false), || {}).unwrap_err();
        assert!(e.starts_with("foreign-unsaved: "), "{e}");
        let m = model::decode(&c.keymap().unwrap(), &c.behaviors().unwrap());
        let e = do_write(&mut c, &p, &m, &m).unwrap_err();
        assert!(e.starts_with("foreign-unsaved: "), "{e}");
        discard_foreign(&mut c, W, &AtomicBool::new(false), || {}).unwrap();
        assert!(!c.port_mut().unsaved);
    }

    #[test]
    fn changed_elsewhere_is_detected() {
        let mut c = client(FakePad::new());
        let p = tmp("elsewhere");
        let r = read_ok(&mut c, &p);
        c.port_mut().keymap.layers[0].name = "XYZ".into();
        let e = do_write(&mut c, &p, &r.model, &r.model).unwrap_err();
        assert!(e.starts_with("changed-elsewhere: "), "{e}");
        assert!(!c.port_mut().unsaved);
    }

    #[test]
    fn write_applies_diff_and_refreshes_cache() {
        let mut c = client(FakePad::new());
        let p = tmp("write");
        let r = read_ok(&mut c, &p);
        let mut t = r.model.clone();
        t.layers[0].bindings[4] = PadBinding::Action {
            profile: 1,
            input: 4,
        };
        t.layers[1].name = "ABC".into();
        let out = do_write(&mut c, &p, &r.model, &t).unwrap();
        assert_eq!(out.model, t);
        assert_eq!(cache::load(&p, None).unwrap().model, t);
        let pad = c.port_mut();
        assert!(!pad.unsaved);
        pad.keymap = pad.keymap.clone(); // état courant == état gelé
        let m = model::decode(
            &pad.keymap,
            &crate::studio::fake::behavior_names()
                .into_iter()
                .map(|(i, n)| (i as u32, n))
                .collect(),
        );
        assert_eq!(m, t);
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }

    #[test]
    fn guard_refuses_before_any_set() {
        let mut c = client(FakePad::new());
        let p = tmp("guard");
        let r = read_ok(&mut c, &p);
        let mut t = r.model.clone();
        t.layers[0].bindings[4] = PadBinding::Action {
            profile: 1,
            input: 4,
        };
        t.layers[1].name = "N".repeat(21);
        let e = do_write(&mut c, &p, &r.model, &t).unwrap_err();
        assert!(e.starts_with("guard: "), "{e}");
        assert!(!c.port_mut().unsaved);
    }

    /// Fait échouer l'écriture suivante dès que le pad a des changements non enregistrés (une fois).
    struct Flaky(FakePad, bool);
    impl Read for Flaky {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(b)
        }
    }
    impl Write for Flaky {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if self.1 && self.0.unsaved {
                self.1 = false;
                return Err(std::io::ErrorKind::BrokenPipe.into());
            }
            self.0.write(b)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0.flush()
        }
    }

    #[test]
    fn failure_mid_write_discards() {
        let mut c = Client::new(Flaky(FakePad::new(), true));
        let p = tmp("mid");
        let r = read(&mut c, &p, W, &AtomicBool::new(false), || {}).unwrap();
        let before = c.port_mut().0.keymap.clone();
        let mut t = r.model.clone();
        t.layers[0].bindings[4] = PadBinding::Action {
            profile: 1,
            input: 4,
        };
        t.layers[0].bindings[5] = PadBinding::Action {
            profile: 1,
            input: 5,
        };
        let e = write(&mut c, &p, &r.model, &t, W, &AtomicBool::new(false), || {}).unwrap_err();
        assert!(e.starts_with("disconnected: "), "{e}");
        let pad = &c.port_mut().0;
        assert!(!pad.unsaved);
        assert_eq!(pad.keymap, before);
    }

    #[test]
    fn cache_ignores_other_serial_and_corrupt_file() {
        let p = tmp("cache");
        let model = PadModel { layers: vec![] };
        let c = Cache {
            serial: "aa".into(),
            read_at: 1,
            model,
            max_name_len: 20,
        };
        cache::save(&p, &c).unwrap();
        assert_eq!(cache::load(&p, Some("aa")), Some(c.clone()));
        assert_eq!(cache::load(&p, None), Some(c));
        assert!(cache::load(&p, Some("bb")).is_none());
        std::fs::write(&p, "{pas du json").unwrap();
        assert!(cache::load(&p, None).is_none());
        assert!(cache::load(&p.with_file_name("absent.json"), None).is_none());
        let _ = std::fs::remove_dir_all(p.parent().unwrap());
    }
}

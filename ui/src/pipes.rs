//! Clients des pipes du moteur : evenements (relayes a la fenetre) et controle (requete/reponse).
use engine::ipc::{Response, CONTROL_PIPE, EVENTS_PIPE};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::Emitter;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, HANDLE};
use windows::Win32::Storage::FileSystem::SECURITY_IDENTIFICATION;
use windows::Win32::System::Pipes::GetNamedPipeServerProcessId;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, DETACHED_PROCESS, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};

pub const ENGINE_EXE: &str = "nullctl-engine.exe";
const TIMEOUT: Duration = Duration::from_secs(3);
// TestObs : jusqu'a 2 s par etape cote moteur.
const TIMEOUT_OBS: Duration = Duration::from_secs(8);

/// Refuse un pipe dont le serveur n'est pas `allowed` (nom de fichier de l'exe, sans casse).
pub fn verify_server_named(handle: HANDLE, allowed: &str) -> Result<(), String> {
    let refused = || "serveur du pipe non reconnu".to_string();
    let mut pid = 0u32;
    unsafe { GetNamedPipeServerProcessId(handle, &mut pid) }.map_err(|_| refused())?;
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
        .map_err(|_| refused())?;
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let res = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    let _ = unsafe { CloseHandle(process) };
    res.map_err(|_| refused())?;
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    let file = std::path::Path::new(&path).file_name().unwrap_or_default();
    if file.to_string_lossy().eq_ignore_ascii_case(allowed) {
        Ok(())
    } else {
        Err(refused())
    }
}

/// Ouvre le pipe (reessaie tant qu'il est occupe, jusqu'a `deadline`) et verifie son serveur.
fn open(name: &str, allowed: &str, write: bool, deadline: Instant) -> Result<File, String> {
    let f = loop {
        // Identification seulement : un faux serveur ne peut pas usurper notre jeton.
        let res = File::options()
            .read(true)
            .write(write)
            .security_qos_flags(SECURITY_IDENTIFICATION.0)
            .open(name);
        match res {
            Ok(f) => break f,
            Err(e)
                if e.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32)
                    && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(20))
            }
            Err(e) if e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND.0 as i32) => {
                return Err("moteur arrete".into())
            }
            Err(e) => return Err(format!("pipe du moteur: {e}")),
        }
    };
    verify_server_named(HANDLE(f.as_raw_handle()), allowed)?;
    Ok(f)
}

/// Une requete `{"cmd": cmd}` sur le pipe `name`, reponse en `timeout` au plus.
pub fn control_at(
    name: &str,
    allowed: &str,
    cmd: &str,
    timeout: Duration,
) -> Result<Response, String> {
    let deadline = Instant::now() + timeout;
    let (tx, rx) = mpsc::channel();
    let (name, allowed, cmd) = (name.to_string(), allowed.to_string(), cmd.to_string());
    // shortcut: un moteur muet garde ce thread bloque dans ReadFile jusqu'a sa fermeture ; acceptable pour une appli de bureau.
    thread::spawn(move || {
        let ask = || -> Result<Response, String> {
            let mut f = open(&name, &allowed, true, deadline)?;
            let line = format!("{}
", serde_json::json!({ "cmd": cmd }));
            f.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
            let mut resp = String::new();
            BufReader::new(f)
                .read_line(&mut resp)
                .map_err(|e| e.to_string())?;
            serde_json::from_str(&resp).map_err(|e| format!("reponse illisible: {e}"))
        };
        let _ = tx.send(ask());
    });
    rx.recv_timeout(timeout)
        .unwrap_or_else(|_| Err("le moteur ne repond pas".into()))
}

pub fn control(cmd: &str) -> Result<Response, String> {
    let t = if cmd == "test_obs" { TIMEOUT_OBS } else { TIMEOUT };
    control_at(CONTROL_PIPE, ENGINE_EXE, cmd, t)
}

/// Relaie le pipe d'evenements `name` vers `emit`, pour toujours (reconnexion toutes les secondes).
pub fn relay(name: &str, allowed: &str, emit: &mut dyn FnMut(&str, Value)) {
    let mut up = None;
    loop {
        match open(
            name,
            allowed,
            false,
            Instant::now() + Duration::from_secs(1),
        ) {
            Ok(f) => {
                emit("engine-up", Value::Null);
                up = Some(true);
                for line in BufReader::new(f).lines() {
                    let Ok(line) = line else { break };
                    if let Ok(v) = serde_json::from_str(&line) {
                        emit("engine-event", v);
                    }
                }
            }
            Err(e) if e == "serveur du pipe non reconnu" => {
                tracing::warn!("pipe d'evenements: {e}")
            }
            Err(_) => {}
        }
        if up != Some(false) {
            emit("engine-down", Value::Null);
            up = Some(false);
        }
        thread::sleep(Duration::from_secs(1));
    }
}

pub fn spawn_events_relay(app: tauri::AppHandle) {
    thread::spawn(move || {
        relay(EVENTS_PIPE, ENGINE_EXE, &mut |kind, v| {
            let _ = app.emit(kind, v);
        })
    });
}

/// Lance `nullctl-engine.exe`, a cote de cet executable, sans console.
pub fn start_engine() -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name(ENGINE_EXE);
    if !exe.exists() {
        return Err(format!(
            "{ENGINE_EXE} introuvable dans {}",
            exe.parent().unwrap_or(&exe).display()
        ));
    }
    std::process::Command::new(&exe)
        .creation_flags(DETACHED_PROCESS.0)
        .spawn()
        .map(drop)
        .map_err(|e| format!("lancement de {ENGINE_EXE}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::ipc::{serve_control, serve_events, Event, Request};
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    const WAIT: Duration = Duration::from_secs(5);

    fn pipe_name() -> String {
        static N: AtomicUsize = AtomicUsize::new(0);
        format!(
            r"\\.\pipe\nullctl-ui-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        )
    }

    /// Le serveur des tests est ce binaire de test.
    fn me() -> String {
        std::env::current_exe()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    fn dummy(r: Request) -> Response {
        match r {
            Request::Status => Response::ok(Some(json!({"profile": 2}))),
            _ => Response::err("non"),
        }
    }

    #[test]
    fn control_roundtrip() {
        let name = pipe_name();
        serve_control(&name, dummy).unwrap();
        let r = control_at(&name, &me(), "status", TIMEOUT).unwrap();
        assert_eq!(r, Response::ok(Some(json!({"profile": 2}))));
        let r = control_at(&name, &me(), "test_obs", TIMEOUT).unwrap();
        assert_eq!(r, Response::err("non"));
    }

    #[test]
    fn control_refuses_unknown_server() {
        let name = pipe_name();
        serve_control(&name, dummy).unwrap();
        let e = control_at(&name, "nullctl-engine.exe", "status", TIMEOUT).unwrap_err();
        assert_eq!(e, "serveur du pipe non reconnu");
    }

    #[test]
    fn control_without_server_fails_fast() {
        let t0 = Instant::now();
        assert!(control_at(&pipe_name(), &me(), "status", TIMEOUT).is_err());
        assert!(t0.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn relay_forwards_events() {
        let name = pipe_name();
        let ev = serve_events(&name).unwrap();
        let (tx, rx) = mpsc::channel();
        let n = name.clone();
        thread::spawn(move || {
            relay(&n, &me(), &mut |kind, v| {
                let _ = tx.send((kind.to_string(), v));
            })
        });
        assert_eq!(rx.recv_timeout(WAIT).unwrap().0, "engine-up");
        let end = Instant::now() + WAIT;
        while ev.clients() < 1 {
            assert!(Instant::now() < end, "client jamais enregistre");
            thread::sleep(Duration::from_millis(5));
        }
        ev.emit(&Event::Profile { profile: 3 });
        assert_eq!(
            rx.recv_timeout(WAIT).unwrap(),
            (
                "engine-event".to_string(),
                json!({"type": "profile", "profile": 3})
            )
        );
    }

    #[test]
    fn relay_treats_unknown_server_as_down() {
        let name = pipe_name();
        let _ev = serve_events(&name).unwrap();
        let (tx, rx) = mpsc::channel();
        let n = name.clone();
        thread::spawn(move || {
            relay(&n, "nullctl-engine.exe", &mut |kind, v| {
                let _ = tx.send((kind.to_string(), v));
            })
        });
        assert_eq!(rx.recv_timeout(WAIT).unwrap().0, "engine-down");
        // Jamais « up » ensuite.
        assert!(rx.recv_timeout(Duration::from_millis(1500)).is_err());
    }
}

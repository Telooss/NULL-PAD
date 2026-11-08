//! Pipe d'evenements vers l'interface : une ligne JSON par evenement, sans jamais bloquer le moteur.
use crate::triggers::Trigger;
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows::core::{HSTRING, PWSTR};
use windows::Win32::Foundation::{CloseHandle, LocalFree, ERROR_PIPE_CONNECTED, HANDLE, HLOCAL};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{
    GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
    TOKEN_USER,
};
use windows::Win32::Storage::FileSystem::{
    FlushFileBuffers, ReadFile, WriteFile, FILE_FLAGS_AND_ATTRIBUTES,
    FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX, PIPE_ACCESS_OUTBOUND,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

pub const EVENTS_PIPE: &str = r"\\.\pipe\nullctl-events";
pub const CONTROL_PIPE: &str = r"\\.\pipe\nullctl-control";
/// Taille maximale d'une ligne de requete.
const MAX_LINE: usize = 4096;
/// Lignes en attente par client ; au-dela, le client perd les suivantes.
const QUEUE: usize = 256;

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Link {
    Usb,
    Ble,
    #[default]
    None,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Key {
        input: String,
        down: bool,
        profile: u8,
    },
    Action {
        input: String,
        trigger: Trigger,
        kind: String,
        ok: bool,
        error: Option<String>,
    },
    Profile {
        profile: u8,
    },
    Link {
        link: Link,
    },
    Config {
        ok: bool,
        error: Option<String>,
    },
}

#[derive(Default)]
struct Inner {
    clients: Mutex<Vec<SyncSender<String>>>,
    profile: Mutex<Option<u8>>,
    link: Mutex<Link>,
}

#[derive(Clone, Default)]
pub struct Events(Arc<Inner>);

impl Events {
    /// Sans serveur : n'emet rien (garde quand meme le dernier profil et le dernier lien).
    pub fn none() -> Self {
        Self::default()
    }

    /// Aide de test : un faux client qui recoit chaque ligne sur le Receiver.
    #[doc(hidden)]
    pub fn recording() -> (Self, Receiver<String>) {
        let ev = Self::default();
        let (tx, rx) = sync_channel(4096);
        ev.0.clients.lock().unwrap().push(tx);
        (ev, rx)
    }

    pub fn emit(&self, e: &Event) {
        match e {
            Event::Profile { profile } => *self.0.profile.lock().unwrap() = Some(*profile),
            Event::Link { link } => *self.0.link.lock().unwrap() = *link,
            _ => {}
        }
        let mut clients = self.0.clients.lock().unwrap();
        if clients.is_empty() {
            return;
        }
        let Ok(mut line) = serde_json::to_string(e) else {
            return;
        };
        line.push('\n');
        // Plein : la ligne est perdue pour ce client. Deconnecte : le client est retire.
        clients.retain(|c| !matches!(c.try_send(line.clone()), Err(TrySendError::Disconnected(_))));
    }

    pub fn clients(&self) -> usize {
        self.0.clients.lock().unwrap().len()
    }

    pub fn last_profile(&self) -> Option<u8> {
        *self.0.profile.lock().unwrap()
    }

    pub fn last_link(&self) -> Link {
        *self.0.link.lock().unwrap()
    }
}

/// Lit une chaine allouee par Windows, puis la libere avec LocalFree.
fn take_local_wstr(s: PWSTR) -> io::Result<String> {
    let out = unsafe { s.to_string() }.map_err(io::Error::other);
    unsafe { LocalFree(Some(HLOCAL(s.0.cast()))) };
    out
}

/// SID texte (S-1-5-21-...) de l'utilisateur du processus, lu dans son jeton.
pub fn current_user_sid() -> io::Result<String> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;
        let mut len = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut len);
        // Vec<u64> : tampon aligne pour TOKEN_USER.
        let mut buf = vec![0u64; (len as usize).div_ceil(8)];
        let res = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            len,
            &mut len,
        );
        let _ = CloseHandle(token);
        res?;
        let user = &*(buf.as_ptr() as *const TOKEN_USER);
        let mut s = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut s)?;
        take_local_wstr(s)
    }
}

/// SECURITY_ATTRIBUTES dont le DACL protege n'autorise que l'utilisateur courant (pipes du moteur).
pub struct OwnedSa(SECURITY_ATTRIBUTES);
// SAFETY : le descripteur est immuable et libere une seule fois, au drop.
unsafe impl Send for OwnedSa {}
unsafe impl Sync for OwnedSa {}
impl OwnedSa {
    pub fn as_ptr(&self) -> *const SECURITY_ATTRIBUTES {
        &self.0
    }
}
impl Drop for OwnedSa {
    fn drop(&mut self) {
        unsafe { LocalFree(Some(HLOCAL(self.0.lpSecurityDescriptor))) };
    }
}

/// Sans cela le DACL par defaut laisse Tout le monde et Anonyme lire le pipe.
pub fn current_user_sa() -> io::Result<OwnedSa> {
    let sddl = HSTRING::from(format!("D:P(A;;GA;;;{})", current_user_sid()?));
    let mut sd = PSECURITY_DESCRIPTOR(std::ptr::null_mut());
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(&sddl, SDDL_REVISION_1, &mut sd, None)?
    };
    Ok(OwnedSa(SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd.0,
        bInheritHandle: false.into(),
    }))
}

/// Handle de pipe ferme au drop ; deplace dans le thread d'ecriture du client.
struct Pipe(HANDLE);
// SAFETY : un handle de pipe peut etre utilise depuis n'importe quel thread.
unsafe impl Send for Pipe {}
impl Drop for Pipe {
    fn drop(&mut self) {
        unsafe {
            let _ = DisconnectNamedPipe(self.0);
            let _ = CloseHandle(self.0);
        }
    }
}

fn create(name: &HSTRING, first: bool, sa: &OwnedSa, duplex: bool) -> io::Result<Pipe> {
    let access = if duplex {
        PIPE_ACCESS_DUPLEX
    } else {
        PIPE_ACCESS_OUTBOUND
    };
    let mode = if first {
        access | FILE_FLAG_FIRST_PIPE_INSTANCE
    } else {
        access | FILE_FLAGS_AND_ATTRIBUTES(0)
    };
    let h = unsafe {
        CreateNamedPipeW(
            name,
            mode,
            PIPE_TYPE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            64 * 1024,
            0,
            0,
            Some(sa.as_ptr()),
        )
    };
    if h.is_invalid() {
        Err(io::Error::last_os_error())
    } else {
        Ok(Pipe(h))
    }
}

/// Cree le pipe (echoue si le nom est deja pris) et accepte les clients dans un thread.
pub fn serve_events(name: &str) -> io::Result<Events> {
    let name = HSTRING::from(name);
    let sa = current_user_sa()?;
    let mut pipe = create(&name, true, &sa, false)?;
    let events = Events::default();
    let ev = events.clone();
    thread::spawn(move || loop {
        let connected = match unsafe { ConnectNamedPipe(pipe.0, None) } {
            Ok(()) => true,
            // Le client s'est connecte entre la creation et ConnectNamedPipe.
            Err(e) => e.code() == ERROR_PIPE_CONNECTED.to_hresult(),
        };
        if connected {
            let (tx, rx) = sync_channel::<String>(QUEUE);
            ev.0.clients.lock().unwrap().push(tx);
            // Un client qui ne lit jamais garde ce thread et ses tampons bornes jusqu'a sa deconnexion (accepte).
            thread::spawn(move || {
                let pipe = pipe; // capture tout le Pipe (Send), pas seulement son HANDLE
                let mut written = 0u32; // obligatoire sans OVERLAPPED
                for line in rx {
                    let res = unsafe {
                        WriteFile(pipe.0, Some(line.as_bytes()), Some(&mut written), None)
                    };
                    if res.is_err() {
                        break;
                    }
                }
            });
        }
        pipe = loop {
            match create(&name, false, &sa, false) {
                Ok(p) => break p,
                Err(e) => {
                    tracing::warn!("pipe d'evenements: {e}");
                    thread::sleep(Duration::from_secs(1));
                }
            }
        };
    });
    Ok(events)
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    Status,
    TestObs,
    TestMidi,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Response {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data: Option<serde_json::Value>,
}

impl Response {
    pub fn ok(data: Option<serde_json::Value>) -> Self {
        Self {
            ok: true,
            error: None,
            data,
        }
    }
    pub fn err(e: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(e.into()),
            data: None,
        }
    }
}

fn write_all(pipe: &Pipe, line: &str) -> bool {
    let mut buf = line.as_bytes();
    while !buf.is_empty() {
        let mut n = 0u32;
        if unsafe { WriteFile(pipe.0, Some(buf), Some(&mut n), None) }.is_err() || n == 0 {
            return false;
        }
        buf = &buf[n as usize..];
    }
    true
}

/// Boucle d'un client : une ligne lue -> une ligne ecrite, jusqu'a la deconnexion.
fn serve_client(pipe: Pipe, handler: &(dyn Fn(Request) -> Response + Send + Sync)) {
    let mut pending: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        while let Some(i) = pending.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = pending.drain(..=i).collect();
            let resp = if line.len() > MAX_LINE {
                Response::err("requete invalide")
            } else {
                match serde_json::from_slice::<Request>(&line) {
                    Ok(r) => handler(r),
                    Err(_) => Response::err("requete invalide"),
                }
            };
            let mut out = serde_json::to_string(&resp).unwrap_or_default();
            out.push('\n');
            if !write_all(&pipe, &out) {
                return;
            }
            if line.len() > MAX_LINE {
                let _ = unsafe { FlushFileBuffers(pipe.0) }; // sinon Disconnect jette la reponse
                return;
            }
        }
        if pending.len() > MAX_LINE {
            let _ = write_all(&pipe, "{\"ok\":false,\"error\":\"requete invalide\"}\n");
            let _ = unsafe { FlushFileBuffers(pipe.0) };
            return;
        }
        let mut n = 0u32;
        if unsafe { ReadFile(pipe.0, Some(&mut chunk), Some(&mut n), None) }.is_err() || n == 0 {
            return;
        }
        pending.extend_from_slice(&chunk[..n as usize]);
    }
}

/// Pipe duplex de controle : un thread par client, la requete est traitee sur ce thread.
/// Echoue si le nom est deja pris.
pub fn serve_control(
    name: &str,
    handler: impl Fn(Request) -> Response + Send + Sync + 'static,
) -> io::Result<()> {
    let name = HSTRING::from(name);
    let sa = current_user_sa()?;
    let mut pipe = create(&name, true, &sa, true)?;
    let handler: Arc<dyn Fn(Request) -> Response + Send + Sync> = Arc::new(handler);
    thread::spawn(move || loop {
        let connected = match unsafe { ConnectNamedPipe(pipe.0, None) } {
            Ok(()) => true,
            Err(e) => e.code() == ERROR_PIPE_CONNECTED.to_hresult(),
        };
        if connected {
            let h = handler.clone();
            thread::spawn(move || {
                let pipe = pipe;
                serve_client(pipe, &*h);
            });
        }
        pipe = loop {
            match create(&name, false, &sa, true) {
                Ok(p) => break p,
                Err(e) => {
                    tracing::warn!("pipe de controle: {e}");
                    thread::sleep(Duration::from_secs(1));
                }
            }
        };
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(e: Event) -> String {
        serde_json::to_string(&e).unwrap()
    }

    /// DACL du pipe en SDDL, lu sur le handle serveur.
    fn dacl_sddl(h: HANDLE) -> String {
        use windows::Win32::Foundation::ERROR_SUCCESS;
        use windows::Win32::Security::Authorization::{
            ConvertSecurityDescriptorToStringSecurityDescriptorW, GetSecurityInfo, SE_KERNEL_OBJECT,
        };
        use windows::Win32::Security::DACL_SECURITY_INFORMATION;
        let mut sd = PSECURITY_DESCRIPTOR(std::ptr::null_mut());
        let err = unsafe {
            GetSecurityInfo(
                h,
                SE_KERNEL_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                None,
                None,
                Some(&mut sd),
            )
        };
        assert_eq!(err, ERROR_SUCCESS);
        let mut s = PWSTR::null();
        let res = unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                sd,
                SDDL_REVISION_1,
                DACL_SECURITY_INFORMATION,
                &mut s,
                None,
            )
        };
        unsafe { LocalFree(Some(HLOCAL(sd.0))) };
        res.unwrap();
        take_local_wstr(s).unwrap()
    }

    #[test]
    fn pipe_dacl_grants_only_current_user() {
        let sid = current_user_sid().unwrap();
        assert!(sid.starts_with("S-1-"), "{sid}");
        let name = HSTRING::from(format!(r"\\.\pipe\nullctl-dacl-{}", std::process::id()));
        let name2 = HSTRING::from(format!(r"\\.\pipe\nullctl-dacl2-{}", std::process::id()));
        let sa = current_user_sa().unwrap();
        let mut keep = Vec::new(); // garde l'instance : la suivante n'est pas la premiere
        for (first, duplex) in [(true, false), (false, false), (true, true), (false, true)] {
            let pipe = create(if duplex { &name2 } else { &name }, first, &sa, duplex).unwrap();
            let dacl = dacl_sddl(pipe.0);
            // Protege (P), une seule ACE, qui autorise ce SID et personne d'autre.
            assert!(dacl.starts_with("D:P("), "{dacl}");
            assert_eq!(dacl.matches('(').count(), 1, "{dacl}");
            assert!(dacl.starts_with("D:P(A;;"), "{dacl}");
            assert!(dacl.ends_with(&format!(";;;{sid})")), "{dacl}");
            keep.push(pipe);
        }
    }

    #[test]
    fn every_variant_serializes_exactly() {
        assert_eq!(
            json(Event::Key {
                input: "K05".into(),
                down: true,
                profile: 2
            }),
            r#"{"type":"key","input":"K05","down":true,"profile":2}"#
        );
        assert_eq!(
            json(Event::Action {
                input: "K05".into(),
                trigger: Trigger::DoubleTap,
                kind: "ObsScene".into(),
                ok: false,
                error: Some("obs absent".into())
            }),
            r#"{"type":"action","input":"K05","trigger":"DoubleTap","kind":"ObsScene","ok":false,"error":"obs absent"}"#
        );
        assert_eq!(
            json(Event::Profile { profile: 2 }),
            r#"{"type":"profile","profile":2}"#
        );
        assert_eq!(
            json(Event::Link { link: Link::Usb }),
            r#"{"type":"link","link":"usb"}"#
        );
        assert_eq!(
            json(Event::Link { link: Link::Ble }),
            r#"{"type":"link","link":"ble"}"#
        );
        assert_eq!(
            json(Event::Link { link: Link::None }),
            r#"{"type":"link","link":"none"}"#
        );
        assert_eq!(
            json(Event::Config {
                ok: true,
                error: None
            }),
            r#"{"type":"config","ok":true,"error":null}"#
        );
    }

    #[test]
    fn keeps_last_profile_and_link() {
        let ev = Events::none();
        assert_eq!((ev.last_profile(), ev.last_link()), (None, Link::None));
        ev.emit(&Event::Profile { profile: 4 });
        ev.emit(&Event::Link { link: Link::Ble });
        assert_eq!((ev.last_profile(), ev.last_link()), (Some(4), Link::Ble));
    }

    #[test]
    fn recording_receives_lines() {
        let (ev, rx) = Events::recording();
        ev.emit(&Event::Profile { profile: 1 });
        assert_eq!(
            rx.try_recv().unwrap(),
            "{\"type\":\"profile\",\"profile\":1}\n"
        );
    }
}

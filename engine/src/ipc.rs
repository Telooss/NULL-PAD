//! Pipe d'evenements vers l'interface : une ligne JSON par evenement, sans jamais bloquer le moteur.
use crate::triggers::Trigger;
use serde::Serialize;
use std::io;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows::core::HSTRING;
use windows::Win32::Foundation::{CloseHandle, ERROR_PIPE_CONNECTED, HANDLE};
use windows::Win32::Storage::FileSystem::{
    WriteFile, FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_OUTBOUND,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

pub const EVENTS_PIPE: &str = r"\\.\pipe\nullctl-events";
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

    /// Pour les tests : un faux client qui recoit chaque ligne sur le Receiver.
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

fn create(name: &HSTRING, first: bool) -> io::Result<Pipe> {
    let mode = if first {
        PIPE_ACCESS_OUTBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE
    } else {
        PIPE_ACCESS_OUTBOUND | FILE_FLAGS_AND_ATTRIBUTES(0)
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
            None,
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
    let mut pipe = create(&name, true)?;
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
            match create(&name, false) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn json(e: Event) -> String {
        serde_json::to_string(&e).unwrap()
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

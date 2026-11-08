//! Client RPC ZMK Studio : une requête à la fois sur un port série (ou un faux pad).
use super::framing::{encode, Decoder};
use super::pb::zmk::{behaviors, core, keymap, meta, studio};
use super::port::Port;
use prost::Message;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::io::{self, ErrorKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub use keymap::BehaviorBinding;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudioError {
    Timeout,
    Locked,
    Disconnected,
    Cancelled,
    Rejected(String),
    Decode(String),
}

impl fmt::Display for StudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => f.write_str("Le pad ne répond pas"),
            Self::Locked => f.write_str("Pad verrouillé : appuie sur M1 + M3"),
            Self::Disconnected => f.write_str("Pad débranché"),
            Self::Cancelled => f.write_str("Annulé"),
            Self::Rejected(m) => write!(f, "Le pad refuse : {m}"),
            Self::Decode(m) => write!(f, "Réponse illisible : {m}"),
        }
    }
}

impl std::error::Error for StudioError {}

type Result<T> = std::result::Result<T, StudioError>;

fn unexpected<T>() -> Result<T> {
    Err(StudioError::Rejected("réponse inattendue".into()))
}

fn is_unlock_note(n: &studio::Notification) -> bool {
    matches!(
        n.subsystem,
        Some(studio::notification::Subsystem::Core(core::Notification {
            notification_type: Some(core::notification::NotificationType::LockStateChanged(1)),
        }))
    )
}

pub struct Client<P: Port> {
    port: P,
    decoder: Decoder,
    next_id: u32,
    queue: VecDeque<studio::Notification>,
    /// Délai maximal d'un appel.
    pub timeout: Duration,
}

impl<P: Port> Client<P> {
    pub fn new(port: P) -> Self {
        Client {
            port,
            decoder: Decoder::new(),
            next_id: 1,
            queue: VecDeque::new(),
            timeout: Duration::from_secs(3),
        }
    }

    #[cfg(test)]
    pub(crate) fn port_mut(&mut self) -> &mut P {
        &mut self.port
    }

    /// Notifications reçues pendant les appels, vidées à la lecture.
    pub fn notifications(&mut self) -> Vec<studio::Notification> {
        self.queue.drain(..).collect()
    }

    /// Une lecture : met les notifications en file, rend les réponses décodées.
    fn pump(&mut self) -> Result<Vec<studio::RequestResponse>> {
        let mut buf = [0u8; 512];
        let n = match self.port.read(&mut buf) {
            Ok(n) => n,
            Err(e) if matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) => 0,
            Err(_) => return Err(StudioError::Disconnected),
        };
        let mut out = Vec::new();
        for frame in self.decoder.push(&buf[..n]) {
            let r = studio::Response::decode(frame.as_slice())
                .map_err(|e| StudioError::Decode(e.to_string()))?;
            match r.r#type {
                Some(studio::response::Type::RequestResponse(rr)) => out.push(rr),
                Some(studio::response::Type::Notification(n)) => self.queue.push_back(n),
                None => {}
            }
        }
        Ok(out)
    }

    /// Envoie une requête et attend la réponse du même id ; les erreurs méta deviennent des `Err`.
    pub fn call(
        &mut self,
        subsystem: studio::request::Subsystem,
    ) -> Result<studio::RequestResponse> {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let req = studio::Request {
            request_id: id,
            subsystem: Some(subsystem),
        };
        let io_err = |_: io::Error| StudioError::Disconnected;
        self.port
            .write_all(&encode(&req.encode_to_vec()))
            .map_err(io_err)?;
        self.port.flush().map_err(io_err)?;
        let deadline = Instant::now() + self.timeout;
        loop {
            if let Some(rr) = self.pump()?.into_iter().find(|r| r.request_id == id) {
                return match rr.subsystem {
                    Some(studio::request_response::Subsystem::Meta(m)) => {
                        Err(match m.response_type {
                            Some(meta::response::ResponseType::SimpleError(e))
                                if e == meta::ErrorConditions::UnlockRequired as i32 =>
                            {
                                StudioError::Locked
                            }
                            other => StudioError::Rejected(format!("{other:?}")),
                        })
                    }
                    _ => Ok(rr),
                };
            }
            if Instant::now() >= deadline {
                return Err(StudioError::Timeout);
            }
        }
    }

    fn rpc_core(&mut self, r: core::request::RequestType) -> Result<core::response::ResponseType> {
        let req = studio::request::Subsystem::Core(core::Request {
            request_type: Some(r),
        });
        match self.call(req)?.subsystem {
            Some(studio::request_response::Subsystem::Core(core::Response {
                response_type: Some(t),
            })) => Ok(t),
            _ => unexpected(),
        }
    }

    fn rpc_behaviors(
        &mut self,
        r: behaviors::request::RequestType,
    ) -> Result<behaviors::response::ResponseType> {
        let req = studio::request::Subsystem::Behaviors(behaviors::Request {
            request_type: Some(r),
        });
        match self.call(req)?.subsystem {
            Some(studio::request_response::Subsystem::Behaviors(behaviors::Response {
                response_type: Some(t),
            })) => Ok(t),
            _ => unexpected(),
        }
    }

    fn rpc_keymap(
        &mut self,
        r: keymap::request::RequestType,
    ) -> Result<keymap::response::ResponseType> {
        let req = studio::request::Subsystem::Keymap(keymap::Request {
            request_type: Some(r),
        });
        match self.call(req)?.subsystem {
            Some(studio::request_response::Subsystem::Keymap(keymap::Response {
                response_type: Some(t),
            })) => Ok(t),
            _ => unexpected(),
        }
    }

    pub fn device_info(&mut self) -> Result<core::GetDeviceInfoResponse> {
        match self.rpc_core(core::request::RequestType::GetDeviceInfo(true))? {
            core::response::ResponseType::GetDeviceInfo(i) => Ok(i),
            _ => unexpected(),
        }
    }

    /// `true` si le pad est verrouillé.
    pub fn lock_state(&mut self) -> Result<bool> {
        match self.rpc_core(core::request::RequestType::GetLockState(true))? {
            core::response::ResponseType::GetLockState(s) => {
                Ok(s != core::LockState::ZmkStudioCoreLockStateUnlocked as i32)
            }
            _ => unexpected(),
        }
    }

    /// Attend le déverrouillage (sondage toutes les 500 ms, ou notification), jusqu'à `timeout`.
    pub fn wait_unlocked(&mut self, timeout: Duration, cancel: &AtomicBool) -> Result<()> {
        self.queue.retain(|n| !is_unlock_note(n));
        let deadline = Instant::now() + timeout;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(StudioError::Cancelled);
            }
            if !self.lock_state()? || self.queue.iter().any(is_unlock_note) {
                return Ok(());
            }
            let until = (Instant::now() + Duration::from_millis(500)).min(deadline);
            while Instant::now() < until {
                if cancel.load(Ordering::Relaxed) {
                    return Err(StudioError::Cancelled);
                }
                self.pump()?;
                if self.queue.iter().any(is_unlock_note) {
                    return Ok(());
                }
            }
            if Instant::now() >= deadline {
                return Err(StudioError::Locked);
            }
        }
    }

    /// id de comportement -> nom affiché.
    pub fn behaviors(&mut self) -> Result<HashMap<u32, String>> {
        let ids =
            match self.rpc_behaviors(behaviors::request::RequestType::ListAllBehaviors(true))? {
                behaviors::response::ResponseType::ListAllBehaviors(l) => l.behaviors,
                _ => return unexpected(),
            };
        let mut map = HashMap::new();
        for id in ids {
            let details = behaviors::request::RequestType::GetBehaviorDetails(
                behaviors::GetBehaviorDetailsRequest { behavior_id: id },
            );
            match self.rpc_behaviors(details)? {
                behaviors::response::ResponseType::GetBehaviorDetails(d) => {
                    map.insert(id, d.display_name);
                }
                _ => return unexpected(),
            }
        }
        Ok(map)
    }

    pub fn keymap(&mut self) -> Result<keymap::Keymap> {
        match self.rpc_keymap(keymap::request::RequestType::GetKeymap(true))? {
            keymap::response::ResponseType::GetKeymap(k) => Ok(k),
            _ => unexpected(),
        }
    }

    pub fn set_binding(&mut self, layer_id: u32, pos: i32, binding: BehaviorBinding) -> Result<()> {
        let r = keymap::request::RequestType::SetLayerBinding(keymap::SetLayerBindingRequest {
            layer_id,
            key_position: pos,
            binding: Some(binding),
        });
        match self.rpc_keymap(r)? {
            keymap::response::ResponseType::SetLayerBinding(c)
                if c == keymap::SetLayerBindingResponse::SetLayerBindingRespOk as i32 =>
            {
                Ok(())
            }
            keymap::response::ResponseType::SetLayerBinding(c) => Err(StudioError::Rejected(
                keymap::SetLayerBindingResponse::try_from(c)
                    .map_or_else(|_| c.to_string(), |e| format!("{e:?}")),
            )),
            _ => unexpected(),
        }
    }

    pub fn set_layer_name(&mut self, layer_id: u32, name: &str) -> Result<()> {
        let r = keymap::request::RequestType::SetLayerProps(keymap::SetLayerPropsRequest {
            layer_id,
            name: name.to_string(),
        });
        match self.rpc_keymap(r)? {
            keymap::response::ResponseType::SetLayerProps(c)
                if c == keymap::SetLayerPropsResponse::SetLayerPropsRespOk as i32 =>
            {
                Ok(())
            }
            keymap::response::ResponseType::SetLayerProps(c) => Err(StudioError::Rejected(
                keymap::SetLayerPropsResponse::try_from(c)
                    .map_or_else(|_| c.to_string(), |e| format!("{e:?}")),
            )),
            _ => unexpected(),
        }
    }

    pub fn check_unsaved(&mut self) -> Result<bool> {
        match self.rpc_keymap(keymap::request::RequestType::CheckUnsavedChanges(true))? {
            keymap::response::ResponseType::CheckUnsavedChanges(b) => Ok(b),
            _ => unexpected(),
        }
    }

    pub fn save(&mut self) -> Result<()> {
        match self.rpc_keymap(keymap::request::RequestType::SaveChanges(true))? {
            keymap::response::ResponseType::SaveChanges(keymap::SaveChangesResponse {
                result: Some(keymap::save_changes_response::Result::Ok(true)),
            }) => Ok(()),
            keymap::response::ResponseType::SaveChanges(r) => {
                Err(StudioError::Rejected(format!("{:?}", r.result)))
            }
            _ => unexpected(),
        }
    }

    pub fn discard(&mut self) -> Result<()> {
        match self.rpc_keymap(keymap::request::RequestType::DiscardChanges(true))? {
            keymap::response::ResponseType::DiscardChanges(_) => Ok(()),
            _ => unexpected(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::fake::*;
    use crate::studio::pb::zmk::{core, studio};
    use std::sync::atomic::AtomicBool;

    fn client(f: FakePad) -> Client<FakePad> {
        let mut c = Client::new(f);
        c.timeout = Duration::from_millis(300);
        c
    }

    fn lock_notification(unlocked: bool) -> studio::Response {
        studio::Response {
            r#type: Some(studio::response::Type::Notification(studio::Notification {
                subsystem: Some(studio::notification::Subsystem::Core(core::Notification {
                    notification_type: Some(
                        core::notification::NotificationType::LockStateChanged(unlocked as i32),
                    ),
                })),
            })),
        }
    }

    #[test]
    fn device_info_works_even_when_locked() {
        let mut f = FakePad::new();
        f.locked = true;
        let info = client(f).device_info().unwrap();
        assert_eq!(info.name, "NULL/PAD");
        assert!(client({
            let mut f = FakePad::new();
            f.locked = true;
            f
        })
        .lock_state()
        .unwrap());
    }

    #[test]
    fn keymap_locked_is_locked_error() {
        let mut f = FakePad::new();
        f.locked = true;
        assert_eq!(client(f).keymap().unwrap_err(), StudioError::Locked);
    }

    #[test]
    fn keymap_and_behaviors_when_unlocked() {
        let mut c = client(FakePad::new());
        let km = c.keymap().unwrap();
        assert_eq!(km.layers.len(), 8);
        assert!(km.layers.iter().all(|l| l.bindings.len() == 29));
        let b = c.behaviors().unwrap();
        assert_eq!(b[&(B_KEY_PRESS as u32)], "Key Press");
        assert_eq!(b[&(b_np_p(8) as u32)], "np_p8");
        assert_eq!(b.len(), 12);
    }

    #[test]
    fn wait_unlocked_after_notification() {
        let mut f = FakePad::new();
        f.locked = true;
        f.unlock_after = Some((1, true));
        let mut c = client(f);
        c.wait_unlocked(Duration::from_secs(2), &AtomicBool::new(false))
            .unwrap();
    }

    #[test]
    fn wait_unlocked_by_polling() {
        let mut f = FakePad::new();
        f.locked = true;
        f.unlock_after = Some((1, false));
        let mut c = client(f);
        c.wait_unlocked(Duration::from_secs(2), &AtomicBool::new(false))
            .unwrap();
    }

    #[test]
    fn wait_unlocked_times_out_and_cancels() {
        let mut f = FakePad::new();
        f.locked = true;
        let mut c = client(f);
        let no = AtomicBool::new(false);
        assert_eq!(
            c.wait_unlocked(Duration::from_millis(50), &no).unwrap_err(),
            StudioError::Locked
        );
        assert_eq!(
            c.wait_unlocked(Duration::from_secs(2), &AtomicBool::new(true))
                .unwrap_err(),
            StudioError::Cancelled
        );
    }

    #[test]
    fn foreign_id_response_is_ignored() {
        let mut f = FakePad::new();
        f.inject(studio::Response {
            r#type: Some(studio::response::Type::RequestResponse(
                studio::RequestResponse {
                    request_id: 999,
                    subsystem: None,
                },
            )),
        });
        assert_eq!(client(f).device_info().unwrap().name, "NULL/PAD");
    }

    #[test]
    fn timeout_when_silent() {
        let mut f = FakePad::new();
        f.silent = true;
        let mut c = client(f);
        c.timeout = Duration::from_millis(50);
        assert_eq!(c.device_info().unwrap_err(), StudioError::Timeout);
    }

    #[test]
    fn cut_mid_response_is_disconnected() {
        let mut f = FakePad::new();
        f.cut_after_bytes = Some(5);
        assert_eq!(
            client(f).device_info().unwrap_err(),
            StudioError::Disconnected
        );
    }

    #[test]
    fn set_binding_then_save() {
        let mut c = client(FakePad::new());
        let b = BehaviorBinding {
            behavior_id: B_TRANSPARENT,
            param1: 0,
            param2: 0,
        };
        c.set_binding(1, 5, b).unwrap();
        assert!(c.check_unsaved().unwrap());
        assert_eq!(c.keymap().unwrap().layers[1].bindings[5], b);
        c.save().unwrap();
        assert!(!c.check_unsaved().unwrap());
        assert_eq!(c.port_mut().keymap.layers[1].bindings[5], b);
    }

    #[test]
    fn set_binding_bad_position_is_rejected_and_discard_restores() {
        let mut c = client(FakePad::new());
        let b = BehaviorBinding {
            behavior_id: B_TRANSPARENT,
            param1: 0,
            param2: 0,
        };
        assert!(matches!(
            c.set_binding(1, 99, b),
            Err(StudioError::Rejected(_))
        ));
        let before = c.keymap().unwrap();
        c.set_binding(0, 0, b).unwrap();
        c.set_layer_name(0, "X").unwrap();
        c.discard().unwrap();
        assert_eq!(c.keymap().unwrap(), before);
    }

    #[test]
    fn notification_during_call_is_queued() {
        let mut f = FakePad::new();
        f.inject(lock_notification(true));
        let mut c = client(f);
        c.device_info().unwrap();
        let n = c.notifications();
        assert_eq!(n.len(), 1);
        assert!(c.notifications().is_empty());
    }

    #[test]
    #[ignore = "nécessite un vrai pad branché"]
    fn reads_real_pad() {
        let name = crate::studio::port::find_pad_port().expect("pad introuvable");
        let mut c = Client::new(crate::studio::port::open_pad(&name).unwrap());
        println!("{:?}", c.device_info().unwrap());
        if !c.lock_state().unwrap() {
            println!("couches : {}", c.keymap().unwrap().layers.len());
        }
    }
}

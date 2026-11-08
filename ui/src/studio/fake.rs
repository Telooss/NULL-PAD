//! Faux pad ZMK Studio pour les tests : parle le protocole en mémoire, sans port série.
use super::framing::{encode, Decoder};
use super::pb::zmk::{behaviors, core, keymap, meta, studio};
use prost::Message;
use std::collections::VecDeque;
use std::io::{self, Read, Write};

pub(crate) const HID_KEYBOARD: u32 = 0x07;
pub(crate) const HID_CONSUMER: u32 = 0x0C;
/// `&kp X` : param1 = (page << 16) | id.
pub(crate) const fn usage(page: u32, id: u32) -> u32 {
    (page << 16) | id
}

// Ids de comportements volontairement non contigus (à partir de 7).
pub(crate) const B_KEY_PRESS: i32 = 7;
pub(crate) const B_STUDIO_UNLOCK: i32 = 9;
pub(crate) const B_TRANSPARENT: i32 = 12;
pub(crate) const B_NP_00: i32 = 20;
/// np_p1..np_p8 = 30, 33, ..., 51.
pub(crate) const fn b_np_p(n: u32) -> i32 {
    27 + 3 * n as i32
}

pub(crate) fn behavior_names() -> Vec<(i32, String)> {
    let mut v = vec![
        (B_KEY_PRESS, "Key Press".to_string()),
        (B_STUDIO_UNLOCK, "Studio Unlock".to_string()),
        (B_TRANSPARENT, "Transparent".to_string()),
        (B_NP_00, "np_00".to_string()),
    ];
    v.extend((1..=8).map(|n| (b_np_p(n), format!("np_p{n}"))));
    v
}

fn bind(id: i32, p1: u32) -> keymap::BehaviorBinding {
    keymap::BehaviorBinding {
        behavior_id: id,
        param1: p1,
        param2: 0,
    }
}

fn kp(page: u32, id: u32) -> keymap::BehaviorBinding {
    bind(B_KEY_PRESS, usage(page, id))
}

/// Les 8 couches de `firmware/null_pad.keymap` (29 positions chacune).
pub(crate) fn null_pad_keymap() -> keymap::Keymap {
    let act = |p: u32, i: u32| kp(HID_CONSUMER, 0x500 + (p - 1) * 32 + i);
    let k = |id| kp(HID_KEYBOARD, id);
    let layers = (0..8u32)
        .map(|l| {
            let p = l + 1;
            let mut b: Vec<_> = if l == 0 {
                vec![
                    k(0x53),
                    k(0x54),
                    k(0x55),
                    k(0x56),
                    k(0x5F),
                    k(0x60),
                    k(0x61),
                    k(0x57),
                    k(0x5C),
                    k(0x5D),
                    k(0x5E),
                    act(1, 11),
                    k(0x59),
                    k(0x5A),
                    k(0x5B),
                    k(0x58),
                    k(0x62),
                    bind(B_NP_00, 0),
                    k(0x63),
                    act(1, 19),
                ]
            } else {
                (0..20).map(|i| act(p, i)).collect()
            };
            b.extend((1..=3).map(|n| bind(b_np_p(n), 0)));
            b.extend([0xE9, 0xEA, 0xE2].map(|id| kp(HID_CONSUMER, id)));
            b.extend([26, 27, 28].map(|i| act(p, i)));
            keymap::Layer {
                id: l,
                name: if l == 0 {
                    "DEF".into()
                } else {
                    format!("P{p:02}")
                },
                bindings: b,
            }
        })
        .collect();
    keymap::Keymap {
        layers,
        available_layers: 0,
        max_layer_name_length: 20,
    }
}

pub(crate) struct FakePad {
    pub locked: bool,
    pub unsaved: bool,
    /// Après N octets lus par le client, toute lecture échoue (câble arraché).
    pub cut_after_bytes: Option<usize>,
    /// Ne répond à rien (test de délai).
    pub silent: bool,
    /// Au repos, les lectures rendent Ok(0) au lieu de TimedOut.
    pub zero_reads: bool,
    /// Après la N-ième requête : déverrouille (et notifie si `true`).
    pub unlock_after: Option<(usize, bool)>,
    pub keymap: keymap::Keymap,
    saved: keymap::Keymap,
    requests: usize,
    served: usize,
    inbound: Decoder,
    outbound: VecDeque<u8>,
}

impl FakePad {
    pub fn new() -> Self {
        let keymap = null_pad_keymap();
        FakePad {
            locked: false,
            unsaved: false,
            cut_after_bytes: None,
            silent: false,
            zero_reads: false,
            unlock_after: None,
            saved: keymap.clone(),
            keymap,
            requests: 0,
            served: 0,
            inbound: Decoder::new(),
            outbound: VecDeque::new(),
        }
    }

    /// Met une réponse/notification brute dans la file de sortie (mauvais id, notification...).
    pub fn inject(&mut self, r: studio::Response) {
        self.outbound.extend(encode(&r.encode_to_vec()));
    }

    /// Octets bruts encadrés en trame (charge utile illisible).
    pub fn inject_raw(&mut self, payload: &[u8]) {
        self.outbound.extend(encode(payload));
    }

    fn reply(&mut self, id: u32, s: studio::request_response::Subsystem) {
        self.inject(studio::Response {
            r#type: Some(studio::response::Type::RequestResponse(
                studio::RequestResponse {
                    request_id: id,
                    subsystem: Some(s),
                },
            )),
        });
    }

    fn meta_error(e: meta::ErrorConditions) -> studio::request_response::Subsystem {
        studio::request_response::Subsystem::Meta(meta::Response {
            response_type: Some(meta::response::ResponseType::SimpleError(e as i32)),
        })
    }

    fn handle(&mut self, req: studio::Request) {
        use studio::request::Subsystem as Q;
        use studio::request_response::Subsystem as R;
        self.requests += 1;
        if self.silent {
            return;
        }
        let id = req.request_id;
        let out = match req.subsystem {
            Some(Q::Core(c)) => R::Core(self.core(c)),
            Some(_) if self.locked => Self::meta_error(meta::ErrorConditions::UnlockRequired),
            Some(Q::Behaviors(b)) => R::Behaviors(self.behaviors(b)),
            Some(Q::Keymap(k)) => R::Keymap(self.keymap(k)),
            None => Self::meta_error(meta::ErrorConditions::RpcNotFound),
        };
        self.reply(id, out);
        if let Some((n, notify)) = self.unlock_after {
            if self.requests == n {
                self.locked = false;
                if notify {
                    self.inject(studio::Response {
                        r#type: Some(studio::response::Type::Notification(studio::Notification {
                            subsystem: Some(studio::notification::Subsystem::Core(
                                core::Notification {
                                    notification_type: Some(
                                        core::notification::NotificationType::LockStateChanged(1),
                                    ),
                                },
                            )),
                        })),
                    });
                }
            }
        }
    }

    fn core(&self, c: core::Request) -> core::Response {
        use core::request::RequestType as Q;
        use core::response::ResponseType as R;
        core::Response {
            response_type: match c.request_type {
                Some(Q::GetDeviceInfo(_)) => Some(R::GetDeviceInfo(core::GetDeviceInfoResponse {
                    name: "NULL/PAD".into(),
                    serial_number: vec![0xDE, 0xAD, 0xBE, 0xEF],
                })),
                Some(Q::GetLockState(_)) => Some(R::GetLockState(if self.locked { 0 } else { 1 })),
                _ => None,
            },
        }
    }

    fn behaviors(&self, b: behaviors::Request) -> behaviors::Response {
        use behaviors::request::RequestType as Q;
        use behaviors::response::ResponseType as R;
        let names = behavior_names();
        behaviors::Response {
            response_type: match b.request_type {
                Some(Q::ListAllBehaviors(_)) => {
                    Some(R::ListAllBehaviors(behaviors::ListAllBehaviorsResponse {
                        behaviors: names.iter().map(|(i, _)| *i as u32).collect(),
                    }))
                }
                Some(Q::GetBehaviorDetails(d)) => names
                    .into_iter()
                    .find(|(i, _)| *i as u32 == d.behavior_id)
                    .map(|(i, n)| {
                        R::GetBehaviorDetails(behaviors::GetBehaviorDetailsResponse {
                            id: i as u32,
                            display_name: n,
                            metadata: vec![],
                        })
                    }),
                None => None,
            },
        }
    }

    fn keymap(&mut self, k: keymap::Request) -> keymap::Response {
        use keymap::request::RequestType as Q;
        use keymap::response::ResponseType as R;
        keymap::Response {
            response_type: match k.request_type {
                Some(Q::GetKeymap(_)) => Some(R::GetKeymap(self.keymap.clone())),
                Some(Q::SetLayerBinding(s)) => {
                    let slot = usize::try_from(s.key_position).ok().and_then(|pos| {
                        let layer = self.keymap.layers.iter_mut().find(|l| l.id == s.layer_id)?;
                        layer.bindings.get_mut(pos)
                    });
                    let code = match (slot, s.binding) {
                        (Some(slot), Some(b)) => {
                            *slot = b;
                            self.unsaved = true;
                            keymap::SetLayerBindingResponse::SetLayerBindingRespOk
                        }
                        _ => keymap::SetLayerBindingResponse::SetLayerBindingRespInvalidLocation,
                    };
                    Some(R::SetLayerBinding(code as i32))
                }
                Some(Q::CheckUnsavedChanges(_)) => Some(R::CheckUnsavedChanges(self.unsaved)),
                Some(Q::SaveChanges(_)) => {
                    self.saved = self.keymap.clone();
                    self.unsaved = false;
                    Some(R::SaveChanges(keymap::SaveChangesResponse {
                        result: Some(keymap::save_changes_response::Result::Ok(true)),
                    }))
                }
                Some(Q::DiscardChanges(_)) => {
                    self.keymap = self.saved.clone();
                    self.unsaved = false;
                    Some(R::DiscardChanges(true))
                }
                Some(Q::SetLayerProps(p)) => {
                    let code = match self.keymap.layers.iter_mut().find(|l| l.id == p.layer_id) {
                        Some(l) => {
                            l.name = p.name;
                            self.unsaved = true;
                            keymap::SetLayerPropsResponse::SetLayerPropsRespOk
                        }
                        None => keymap::SetLayerPropsResponse::SetLayerPropsRespErrInvalidId,
                    };
                    Some(R::SetLayerProps(code as i32))
                }
                _ => None,
            },
        }
    }
}

impl Write for FakePad {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for frame in self.inbound.push(buf) {
            let req = studio::Request::decode(frame.as_slice())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            self.handle(req);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for FakePad {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut room = buf.len();
        if let Some(cut) = self.cut_after_bytes {
            if self.served >= cut {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            room = room.min(cut - self.served);
        }
        if self.outbound.is_empty() {
            if self.zero_reads {
                return Ok(0);
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
            return Err(io::ErrorKind::TimedOut.into());
        }
        let n = room.min(self.outbound.len());
        for slot in buf.iter_mut().take(n) {
            *slot = self.outbound.pop_front().unwrap();
        }
        self.served += n;
        Ok(n)
    }
}

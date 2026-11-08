//! Trames série ZMK Studio : SOF 0xAB, EOF 0xAD, ESC 0xAC devant tout octet de contrôle du message.
pub const SOF: u8 = 0xAB;
pub const ESC: u8 = 0xAC;
pub const EOF: u8 = 0xAD;

pub fn encode(msg: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(msg.len() + 2);
    out.push(SOF);
    for &b in msg {
        if matches!(b, SOF | ESC | EOF) {
            out.push(ESC);
        }
        out.push(b);
    }
    out.push(EOF);
    out
}

/// Découpe un flux d'octets en trames ; l'état survit entre deux `push`.
#[derive(Default)]
pub struct Decoder {
    in_frame: bool,
    escaped: bool,
    buf: Vec<u8>,
}

impl Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Charges utiles (sans SOF/EOF, déséchappées) des trames terminées par ces octets.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut frames = Vec::new();
        for &b in bytes {
            if !self.in_frame {
                if b == SOF {
                    self.in_frame = true;
                }
            } else if self.escaped {
                self.escaped = false;
                self.buf.push(b);
            } else {
                match b {
                    SOF => self.buf.clear(),
                    ESC => self.escaped = true,
                    EOF => {
                        self.in_frame = false;
                        if !self.buf.is_empty() {
                            frames.push(std::mem::take(&mut self.buf));
                        }
                    }
                    _ => self.buf.push(b),
                }
            }
        }
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::pb::zmk::{keymap, studio};
    use prost::Message;

    #[test]
    fn encode_plain() {
        assert_eq!(encode(&[1, 2, 3]), [SOF, 1, 2, 3, EOF]);
    }

    #[test]
    fn encode_escapes_control_bytes() {
        assert_eq!(
            encode(&[SOF, ESC, EOF, 7]),
            [SOF, ESC, SOF, ESC, ESC, ESC, EOF, 7, EOF]
        );
    }

    #[test]
    fn decode_byte_by_byte() {
        let msg = [1, SOF, ESC, EOF, 9];
        let mut d = Decoder::new();
        let mut out = vec![];
        for b in encode(&msg) {
            out.extend(d.push(&[b]));
        }
        assert_eq!(out, vec![msg.to_vec()]);
    }

    #[test]
    fn esc_at_end_of_buffer_is_remembered() {
        let mut d = Decoder::new();
        assert!(d.push(&[SOF, 1, ESC]).is_empty());
        assert_eq!(d.push(&[EOF, EOF]), vec![vec![1, EOF]]);
    }

    #[test]
    fn two_frames_at_once() {
        let mut buf = encode(&[1]);
        buf.extend(encode(&[2, 3]));
        assert_eq!(Decoder::new().push(&buf), vec![vec![1], vec![2, 3]]);
    }

    #[test]
    fn stray_bytes_and_restart_on_sof() {
        let buf = [9, 9, EOF, SOF, 1, 2, SOF, 5, EOF, 8];
        assert_eq!(Decoder::new().push(&buf), vec![vec![5]]);
    }

    #[test]
    fn empty_frame_is_dropped() {
        assert!(Decoder::new().push(&[SOF, EOF]).is_empty());
    }

    #[test]
    fn request_roundtrip() {
        let req = studio::Request {
            request_id: 0xAB,
            subsystem: Some(studio::request::Subsystem::Keymap(keymap::Request {
                request_type: Some(keymap::request::RequestType::GetKeymap(true)),
            })),
        };
        let frames = Decoder::new().push(&encode(&req.encode_to_vec()));
        assert_eq!(frames.len(), 1);
        assert_eq!(studio::Request::decode(frames[0].as_slice()).unwrap(), req);
    }
}

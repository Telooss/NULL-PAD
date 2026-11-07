//! Sortie MIDI vers un port existant (loopMIDI), via WinMM (midir).
use crate::config::MidiSettings;
use arc_swap::ArcSwap;
use midir::{MidiOutput, MidiOutputConnection};
use std::sync::{Arc, Mutex};

fn msg(status: u8, channel: u8, a: u8, b: u8) -> [u8; 3] {
    [
        status | (channel.wrapping_sub(1) & 0x0F),
        a & 0x7F,
        b & 0x7F,
    ]
}

pub fn note_on(channel: u8, note: u8, velocity: u8) -> [u8; 3] {
    msg(0x90, channel, note, velocity)
}

pub fn note_off(channel: u8, note: u8) -> [u8; 3] {
    msg(0x80, channel, note, 0)
}

pub fn cc(channel: u8, cc: u8, value: u8) -> [u8; 3] {
    msg(0xB0, channel, cc, value)
}

/// Première sortie dont le nom contient `wanted` (sans casse).
pub fn pick_port(names: &[String], wanted: &str) -> Option<usize> {
    let w = wanted.to_lowercase();
    names.iter().position(|n| n.to_lowercase().contains(&w))
}

pub struct Midi {
    conn: Mutex<Option<MidiOutputConnection>>,
    port: ArcSwap<String>,
}

impl Midi {
    pub fn new() -> Arc<Midi> {
        Arc::new(Midi {
            conn: Mutex::new(None),
            port: ArcSwap::from_pointee(MidiSettings::default().port),
        })
    }

    pub fn apply(&self, s: &MidiSettings) {
        if **self.port.load() != s.port {
            self.port.store(Arc::new(s.port.clone()));
            *self.conn.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }

    pub fn send(&self, msgs: &[[u8; 3]]) -> Result<(), String> {
        let mut guard = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            *guard = Some(self.open()?);
        }
        let conn = guard.as_mut().expect("ouvert ci-dessus");
        for m in msgs {
            if let Err(e) = conn.send(m) {
                *guard = None; // le prochain appel rouvre
                return Err(format!("envoi MIDI : {e}"));
            }
        }
        Ok(())
    }

    fn open(&self) -> Result<MidiOutputConnection, String> {
        let wanted = self.port.load();
        let out = MidiOutput::new("nullpad").map_err(|e| format!("MIDI : {e}"))?;
        let ports = out.ports();
        let names: Vec<String> = ports
            .iter()
            .map(|p| out.port_name(p).unwrap_or_default())
            .collect();
        let i = pick_port(&names, &wanted)
            .ok_or_else(|| format!("port MIDI \"{wanted}\" introuvable (loopMIDI lancé ?)"))?;
        out.connect(&ports[i], "nullpad")
            .map_err(|e| format!("ouverture du port MIDI \"{}\" : {e}", names[i]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_messages() {
        assert_eq!(note_on(1, 60, 100), [0x90, 60, 100]);
        assert_eq!(note_on(16, 60, 100)[0], 0x9F);
        assert_eq!(note_off(16, 60), [0x8F, 60, 0]);
        assert_eq!(cc(16, 20, 1), [0xBF, 20, 1]);
    }

    #[test]
    fn picks_port_by_substring() {
        let names = vec!["Autre".to_string(), "NULL-PAD 1".to_string()];
        assert_eq!(pick_port(&names, "null-pad"), Some(1));
        assert_eq!(pick_port(&names, "zzz"), None);
    }

    #[test]
    fn unknown_port_reports_clear_error() {
        let m = Midi::new();
        m.apply(&MidiSettings {
            port: "zz-no-such-port-zz".into(),
        });
        assert_eq!(
            m.send(&[cc(1, 20, 1)]),
            Err("port MIDI \"zz-no-such-port-zz\" introuvable (loopMIDI lancé ?)".to_string())
        );
    }

    #[test]
    #[ignore = "ouvre un vrai port MIDI"]
    fn sends_to_real_port() {
        let m = Midi::new();
        m.apply(&MidiSettings {
            port: String::new(),
        }); // "" = premier port
        m.send(&[cc(1, 20, 1)]).unwrap();
    }
}

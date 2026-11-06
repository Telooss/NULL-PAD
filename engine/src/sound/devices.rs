//! Peripheriques audio : un thread possede les sorties et le micro, les rouvre au besoin.

use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::cpal::{self, Device, Host, SampleFormat, Stream};
use rodio::mixer::Mixer;
use rodio::{DeviceSinkBuilder, MixerDeviceSink};
use tracing::{debug, error, info, warn};

use crate::config::{MicMode, SoundSettings};
use crate::sound::board::{Outputs, Soundboard};
use crate::sound::mic::{mic_channel, push_frames};

pub const CABLE_HINT: &str = "CABLE Input";
const RETRY: Duration = Duration::from_secs(5);

/// Index du premier nom contenant `wanted` (sinon `auto`), sans casse.
/// Les deux absents : `None` (= peripherique par defaut).
pub fn pick_device(names: &[String], wanted: Option<&str>, auto: Option<&str>) -> Option<usize> {
    let needle = wanted.or(auto)?.to_lowercase();
    names.iter().position(|n| n.to_lowercase().contains(&needle))
}

/// Sans son ni micro via l'appli, aucun peripherique n'est ouvert.
fn wants_audio(s: &SoundSettings) -> bool {
    !s.sounds.is_empty() || s.mic_mode == MicMode::App
}

fn devices_changed(old: &SoundSettings, new: &SoundSettings) -> bool {
    old.monitor_device != new.monitor_device
        || old.cable_device != new.cable_device
        || old.mic_mode != new.mic_mode
        || old.mic_device != new.mic_device
        || old.sounds.is_empty() != new.sounds.is_empty()
}

/// Toutes les sorties demandees sont ouvertes. VB-Cable en detection auto
/// n'est exige qu'en mode micro `App`.
fn all_open(s: &SoundSettings, monitor: bool, cable: bool, mic: bool) -> bool {
    let app = s.mic_mode == MicMode::App;
    monitor && (cable || !(app || s.cable_device.is_some())) && (mic || !app)
}

enum AudioCmd {
    Apply(SoundSettings),
    StreamError,
}

pub struct AudioHandle {
    tx: Sender<AudioCmd>,
}

impl AudioHandle {
    pub fn apply(&self, s: &SoundSettings) {
        if self.tx.send(AudioCmd::Apply(s.clone())).is_err() {
            error!("thread audio arrete : reglages son ignores");
        }
    }
}

/// Lance le thread audio et rend la main tout de suite.
pub fn start_audio(board: Arc<Soundboard>) -> AudioHandle {
    let (tx, rx) = channel();
    let cb = tx.clone(); // garde le canal ouvert : le thread vit autant que le processus
    if let Err(e) = thread::Builder::new().name("audio".into()).spawn(move || run(board, rx, cb)) {
        error!("thread audio : lancement impossible : {e}");
    }
    AudioHandle { tx }
}

/// VB-Cable installe (une sortie contient `CABLE_HINT`).
pub fn cable_present() -> bool {
    find(&cpal::default_host(), false, None, Some(CABLE_HINT), "VB-Cable", false).is_some()
}

fn run(board: Arc<Soundboard>, rx: Receiver<AudioCmd>, tx: Sender<AudioCmd>) {
    let mut cur: Option<SoundSettings> = None;
    let mut dev = Opened::default();
    let mut complete = true;
    loop {
        let mut reset = false;
        match rx.recv_timeout(RETRY) {
            Ok(first) => {
                for cmd in std::iter::once(first).chain(rx.try_iter()) {
                    match cmd {
                        AudioCmd::Apply(s) => {
                            reset |= cur.as_ref().is_none_or(|c| devices_changed(c, &s));
                            cur = Some(s);
                        }
                        AudioCmd::StreamError => reset = true,
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let Some(s) = &cur else { continue };
        if reset {
            board.set_outputs(Outputs::default());
            dev = Opened::default(); // ferme tout avant de rouvrir
            complete = !wants_audio(s) || dev.fill(s, &board, &tx, true);
        } else if !complete {
            complete = dev.fill(s, &board, &tx, false);
        }
    }
}

#[derive(Default)]
struct Opened {
    monitor: Option<MixerDeviceSink>,
    cable: Option<MixerDeviceSink>,
    mic: Option<Stream>,
}

impl Opened {
    /// Ouvre ce qui manque, publie les sorties ; vrai si tout ce qui est demande est ouvert.
    /// `loud` faux (nouvelle tentative) : journal en debug pour ne pas inonder le fichier.
    fn fill(&mut self, s: &SoundSettings, board: &Arc<Soundboard>, tx: &Sender<AudioCmd>, loud: bool) -> bool {
        let host = cpal::default_host();
        if self.monitor.is_none() {
            let d = find(&host, false, s.monitor_device.as_deref(), None, "casque", loud);
            self.monitor = d.and_then(|d| open_sink(d, "casque", tx, loud));
        }
        if self.cable.is_none() {
            self.mic = None; // la MicSource vit dans le mixeur VB-Cable
            let d = find(&host, false, s.cable_device.as_deref(), Some(CABLE_HINT), "VB-Cable", loud);
            self.cable = d.and_then(|d| open_sink(d, "VB-Cable", tx, loud));
        }
        if s.mic_mode == MicMode::App && self.mic.is_none() {
            match &self.cable {
                Some(c) => self.mic = open_mic(&host, s, board, c.mixer(), tx, loud),
                None => note(loud, false, "micro via l'appli : pas de VB-Cable, pas de micro"),
            }
        }
        board.set_outputs(Outputs {
            monitor: self.monitor.as_ref().map(|m| m.mixer().clone()),
            cable: self.cable.as_ref().map(|m| m.mixer().clone()),
        });
        all_open(s, self.monitor.is_some(), self.cable.is_some(), self.mic.is_some())
    }
}

/// warn/error au premier essai, debug pendant les nouvelles tentatives.
fn note(loud: bool, failure: bool, msg: &str) {
    match (loud, failure) {
        (false, _) => debug!("{msg}"),
        (true, false) => warn!("{msg}"),
        (true, true) => error!("{msg}"),
    }
}

fn dev_name(d: &Device) -> String {
    d.description().map(|x| x.name().to_string()).unwrap_or_default()
}

/// `wanted` et `auto` absents : peripherique par defaut (cpal suit celui de Windows).
fn find(host: &Host, input: bool, wanted: Option<&str>, auto: Option<&str>, label: &str, loud: bool) -> Option<Device> {
    let found = if wanted.is_none() && auto.is_none() {
        if input { host.default_input_device() } else { host.default_output_device() }
    } else {
        let list = if input { host.input_devices().map(|d| d.collect()) } else { host.output_devices().map(|d| d.collect()) };
        let list: Vec<Device> = list.unwrap_or_else(|e| {
            note(loud, true, &format!("{label} : liste des peripheriques impossible : {e}"));
            Vec::new()
        });
        let names: Vec<String> = list.iter().map(dev_name).collect();
        pick_device(&names, wanted, auto).map(|i| list[i].clone())
    };
    if found.is_none() {
        note(loud, false, &format!("{label} introuvable ({})", wanted.or(auto).unwrap_or("par defaut")));
    }
    found
}

/// Rappel d'erreur des flux : peripherique perdu ou flux invalide -> reouverture.
fn on_error(tx: Sender<AudioCmd>) -> impl FnMut(cpal::Error) + Clone + Send + 'static {
    move |e: cpal::Error| match e.kind() {
        cpal::ErrorKind::DeviceNotAvailable | cpal::ErrorKind::StreamInvalidated => {
            error!("flux audio perdu : {e}");
            let _ = tx.send(AudioCmd::StreamError);
        }
        cpal::ErrorKind::Xrun => debug!("flux audio : {e}"), // un par craquement : trop bavard en warn
        _ => warn!("flux audio : {e}"),
    }
}

fn open_sink(dev: Device, label: &str, tx: &Sender<AudioCmd>, loud: bool) -> Option<MixerDeviceSink> {
    let name = dev_name(&dev);
    let res = DeviceSinkBuilder::from_device(dev)
        .and_then(|b| b.with_error_callback(on_error(tx.clone())).open_sink_or_fallback());
    match res {
        Ok(mut sink) => {
            sink.log_on_drop(false);
            let c = sink.config();
            info!(
                "{label} : {name}, {} Hz, {} canaux, tampon {:?}",
                c.sample_rate(),
                c.channel_count(),
                c.buffer_size()
            );
            Some(sink)
        }
        Err(e) => {
            note(loud, true, &format!("{label} ({name}) : ouverture impossible : {e}"));
            None
        }
    }
}

/// Micro via l'appli : flux d'entree -> anneau -> MicSource dans le mixeur VB-Cable.
fn open_mic(
    host: &Host,
    s: &SoundSettings,
    board: &Arc<Soundboard>,
    cable: &Mixer,
    tx: &Sender<AudioCmd>,
    loud: bool,
) -> Option<Stream> {
    let dev = find(host, true, s.mic_device.as_deref(), None, "micro", loud)?;
    let name = dev_name(&dev);
    let fail = |e: &dyn std::fmt::Display| note(loud, true, &format!("micro ({name}) : {e}"));
    let cfg = dev.default_input_config().map_err(|e| fail(&e)).ok()?;
    // shortcut: f32 seulement (WASAPI partage), elargir si un cas reel apparait
    if cfg.sample_format() != SampleFormat::F32 {
        fail(&format!("format {} non gere", cfg.sample_format()));
        return None;
    }
    let (ch, rate) = (cfg.channels(), cfg.sample_rate());
    let (mut prod, src) = mic_channel(ch, rate, board.clone());
    let stream = dev
        .build_input_stream(cfg.config(), move |data: &[f32], _: &_| push_frames(&mut prod, data, ch), on_error(tx.clone()), None)
        .map_err(|e| fail(&e))
        .ok()?;
    stream.play().map_err(|e| fail(&e)).ok()?;
    cable.add(src);
    info!("micro : {name}, {rate} Hz, {ch} canaux, tampon {:?}", cfg.buffer_size());
    Some(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MicMode, SoundDef};

    fn names() -> Vec<String> {
        ["Speakers (Realtek Audio)", "Speakers (Focusrite USB Audio)", "CABLE Input (VB-Audio Virtual Cable)"]
            .map(String::from)
            .to_vec()
    }

    fn with_sound() -> SoundSettings {
        let def = SoundDef { id: "a".into(), path: "a.wav".into(), volume: 100, on_repeat: Default::default() };
        SoundSettings { sounds: vec![def], ..Default::default() }
    }

    #[test]
    fn pick_by_substring_ignoring_case() {
        assert_eq!(pick_device(&names(), Some("focusrite"), None), Some(1));
    }

    #[test]
    fn pick_wanted_beats_auto() {
        assert_eq!(pick_device(&names(), Some("realtek"), Some(CABLE_HINT)), Some(0));
    }

    #[test]
    fn pick_auto_finds_cable() {
        assert_eq!(pick_device(&names(), None, Some(CABLE_HINT)), Some(2));
    }

    #[test]
    fn pick_nothing_found_or_asked() {
        assert_eq!(pick_device(&names(), Some("hdmi"), None), None);
        assert_eq!(pick_device(&names(), Some("hdmi"), Some(CABLE_HINT)), None); // pas de repli sur auto
        assert_eq!(pick_device(&names(), None, None), None);
    }

    #[test]
    fn pick_first_of_two_matches() {
        assert_eq!(pick_device(&names(), Some("speakers"), None), Some(0));
    }

    #[test]
    fn audio_wanted_with_sounds_or_app_mic() {
        assert!(!wants_audio(&SoundSettings::default()));
        assert!(wants_audio(&with_sound()));
        assert!(wants_audio(&SoundSettings { mic_mode: MicMode::App, ..Default::default() }));
    }

    #[test]
    fn device_settings_changes_detected() {
        let base = with_sound();
        assert!(!devices_changed(&base, &base.clone()));
        let other = SoundSettings { master_volume: 10, monitor_volume: 0, cable_volume: 5, mic_gain: 1, ..base.clone() };
        assert!(!devices_changed(&base, &other), "volumes seuls");
        let mut more = base.clone();
        more.sounds.push(SoundDef { id: "b".into(), ..base.sounds[0].clone() });
        assert!(!devices_changed(&base, &more), "liste de sons non vide dans les deux cas");
        for changed in [
            SoundSettings { monitor_device: Some("x".into()), ..base.clone() },
            SoundSettings { cable_device: Some("x".into()), ..base.clone() },
            SoundSettings { mic_mode: MicMode::App, ..base.clone() },
            SoundSettings { mic_device: Some("x".into()), ..base.clone() },
            SoundSettings::default(), // sons vides
        ] {
            assert!(devices_changed(&base, &changed), "{changed:?}");
        }
    }

    #[test]
    fn all_open_depends_on_what_is_requested() {
        let s = with_sound();
        assert!(all_open(&s, true, false, false), "VB-Cable auto absent : pas exige");
        assert!(!all_open(&s, false, true, false), "casque toujours exige");
        let named = SoundSettings { cable_device: Some("cable".into()), ..s.clone() };
        assert!(!all_open(&named, true, false, false));
        let app = SoundSettings { mic_mode: MicMode::App, ..s };
        assert!(!all_open(&app, true, false, false));
        assert!(!all_open(&app, true, true, false));
        assert!(all_open(&app, true, true, true));
    }

    /// Ouvre les vrais peripheriques (aucun son joue). Lancer a la main :
    /// `cargo test -p engine opens_real_devices -- --ignored`
    #[test]
    #[ignore]
    fn opens_real_devices() {
        let board = crate::sound::board::Soundboard::new();
        let audio = start_audio(board.clone());
        audio.apply(&with_sound()); // reglages par defaut + un son, sinon rien n'est ouvert
        std::thread::sleep(Duration::from_secs(2));
        assert!(board.has_monitor());
    }
}

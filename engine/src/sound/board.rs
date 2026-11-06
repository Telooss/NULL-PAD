//! Soundboard : joue les sons sur les sorties presentes, gere rappui, arret et volumes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use rodio::mixer::Mixer;

use crate::actions::audio::clamp_level;
use crate::config::{MicMode, OnRepeat, SoundSettings};
use crate::sound::library::Library;
use crate::sound::voice::{Master, Voice, VoiceState};

#[derive(Default)]
pub struct Outputs {
    pub monitor: Option<Mixer>,
    pub cable: Option<Mixer>,
}

pub struct Soundboard {
    master: Arc<Master>,
    outputs: ArcSwap<Outputs>,
    library: ArcSwap<Library>,
    settings: ArcSwap<SoundSettings>,
    voices: Mutex<HashMap<String, Arc<VoiceState>>>,
    mic_gain: AtomicU32, // bits f32
    mic_muted: AtomicBool,
}

impl Soundboard {
    pub fn new() -> Arc<Soundboard> {
        Arc::new(Soundboard {
            master: Master::new(1.0),
            outputs: ArcSwap::from_pointee(Outputs::default()),
            library: ArcSwap::from_pointee(Library::empty()),
            settings: ArcSwap::from_pointee(SoundSettings::default()),
            voices: Mutex::new(HashMap::new()),
            mic_gain: AtomicU32::new(1.0f32.to_bits()),
            mic_muted: AtomicBool::new(false),
        })
    }

    pub fn set_outputs(&self, o: Outputs) {
        self.outputs.store(Arc::new(o));
    }

    /// Nouveaux reglages : bibliotheque rechargee ; gain micro et volume maitre
    /// ne reviennent a la valeur du fichier que si celle-ci a change.
    pub fn apply(&self, s: &SoundSettings) {
        self.library.store(Arc::new(Library::build(&s.sounds, &self.library.load())));
        let old = self.settings.swap(Arc::new(s.clone()));
        if old.mic_gain != s.mic_gain {
            self.mic_gain.store((f32::from(s.mic_gain) / 100.0).to_bits(), Relaxed);
        }
        if old.master_volume != s.master_volume {
            self.master.set(f32::from(s.master_volume) / 100.0);
        }
    }

    pub fn master(&self) -> Arc<Master> {
        self.master.clone()
    }

    pub fn change_master(&self, delta: i8) -> f32 {
        let v = clamp_level(self.master.level(), delta);
        self.master.set(v);
        v
    }

    pub fn toggle_master_mute(&self) -> bool {
        self.master.toggle_mute()
    }

    pub fn play(&self, id: &str) -> Result<(), String> {
        let loaded = self.library.load().get(id).ok_or_else(|| format!("son '{id}' inconnu ou non charge"))?;
        let mut voices = self.voices.lock().unwrap();
        if let Some(old) = voices.get(id) {
            let playing = old.is_playing();
            if playing {
                old.stop();
            }
            if playing && loaded.on_repeat == OnRepeat::Stop {
                voices.remove(id);
                return Ok(());
            }
        }
        let state = VoiceState::new();
        let (s, o) = (self.settings.load(), self.outputs.load());
        for (mixer, vol) in [(&o.monitor, s.monitor_volume), (&o.cable, s.cable_volume)] {
            if let (Some(m), true) = (mixer, vol > 0) {
                let gain = loaded.volume * f32::from(vol) / 100.0;
                m.add(Voice::new(&loaded.clip, gain, self.master.clone(), state.clone()));
            }
        }
        voices.insert(id.to_string(), state);
        Ok(())
    }

    pub fn stop_all(&self) {
        for (_, v) in self.voices.lock().unwrap().drain() {
            v.stop();
        }
    }

    pub fn mic_via_app(&self) -> bool {
        self.settings.load().mic_mode == MicMode::App
    }

    pub fn change_mic_gain(&self, delta: i8) -> f32 {
        let v = clamp_level(f32::from_bits(self.mic_gain.load(Relaxed)), delta);
        self.mic_gain.store(v.to_bits(), Relaxed);
        v
    }

    pub fn toggle_mic_mute(&self) -> bool {
        !self.mic_muted.fetch_xor(true, Relaxed)
    }

    pub fn mic_gain(&self) -> f32 {
        if self.mic_muted.load(Relaxed) { 0.0 } else { f32::from_bits(self.mic_gain.load(Relaxed)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{OnRepeat, SoundDef};
    use crate::sound::library::write_wav;
    use rodio::mixer::{mixer, MixerSource};
    use std::num::NonZero;
    use std::path::PathBuf;

    fn mix() -> (rodio::mixer::Mixer, MixerSource) {
        mixer(NonZero::new(2).unwrap(), NonZero::new(48000).unwrap())
    }

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("nullpad-board-{}-{}.wav", std::process::id(), name))
    }

    /// Cree un son wav (1 s, 48 kHz stereo) et renvoie sa definition.
    fn def(id: &str, volume: u8, on_repeat: OnRepeat) -> SoundDef {
        let p = tmp(id);
        write_wav(&p, 48000, 2, 48000);
        SoundDef { id: id.into(), path: p.to_string_lossy().into(), volume, on_repeat }
    }

    fn settings(sounds: Vec<SoundDef>) -> SoundSettings {
        SoundSettings { sounds, ..SoundSettings::default() }
    }

    /// Soundboard avec deux sorties factices + leurs sources.
    fn setup(s: &SoundSettings) -> (Arc<Soundboard>, MixerSource, MixerSource) {
        let b = Soundboard::new();
        b.apply(s);
        let ((m, ms), (c, cs)) = (mix(), mix());
        b.set_outputs(Outputs { monitor: Some(m), cable: Some(c) });
        (b, ms, cs)
    }

    fn pull(src: &mut MixerSource, n: usize) -> Vec<f32> {
        // le mixeur s'arrete quand il n'a plus de source : on complete par du silence
        let mut v: Vec<f32> = src.take(n).collect();
        v.resize(n, 0.0);
        v
    }

    fn first_nonzero(src: &mut MixerSource) -> f32 {
        src.take(100).find(|s| *s != 0.0).expect("aucun echantillon")
    }

    fn v0(b: &Soundboard, id: &str) -> f32 {
        b.library.load().get(id).unwrap().clip.samples[0]
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn play_reaches_both_outputs_with_volumes() {
        let s = SoundSettings { monitor_volume: 50, cable_volume: 100, ..settings(vec![def("vol", 80, OnRepeat::Restart)]) };
        let (b, mut ms, mut cs) = setup(&s);
        b.play("vol").unwrap();
        let v = v0(&b, "vol");
        assert!(near(first_nonzero(&mut ms), 0.4 * v));
        assert!(near(first_nonzero(&mut cs), 0.8 * v));
    }

    #[test]
    fn zero_monitor_volume_skips_monitor() {
        let s = SoundSettings { monitor_volume: 0, ..settings(vec![def("zero", 100, OnRepeat::Restart)]) };
        let (b, mut ms, mut cs) = setup(&s);
        b.play("zero").unwrap();
        assert!(pull(&mut ms, 200).iter().all(|x| *x == 0.0));
        assert!(pull(&mut cs, 200).iter().any(|x| *x != 0.0));
    }

    #[test]
    fn restart_keeps_one_voice() {
        let (b, _ms, mut cs) = setup(&settings(vec![def("rst", 100, OnRepeat::Restart)]));
        for _ in 0..3 {
            b.play("rst").unwrap();
        }
        let out = pull(&mut cs, 4000); // > fondu (5 ms)
        let max = 1100.0 / 32768.0; // plus grand echantillon d'une seule voix
        let last = *out.last().unwrap();
        assert!(last != 0.0 && last <= max, "last {last}");
        assert_eq!(b.voices.lock().unwrap().len(), 1);
    }

    #[test]
    fn stop_mode_toggles() {
        let (b, _ms, _cs) = setup(&settings(vec![def("tog", 100, OnRepeat::Stop)]));
        let playing = |b: &Soundboard| b.voices.lock().unwrap().get("tog").is_some_and(|v| v.is_playing());
        b.play("tog").unwrap();
        assert!(playing(&b));
        b.play("tog").unwrap();
        assert!(!playing(&b));
        b.play("tog").unwrap();
        assert!(playing(&b));
    }

    #[test]
    fn stop_all_silences() {
        let (b, mut ms, mut cs) = setup(&settings(vec![def("a", 100, OnRepeat::Restart), def("b", 100, OnRepeat::Restart)]));
        b.play("a").unwrap();
        b.play("b").unwrap();
        b.stop_all();
        assert!(b.voices.lock().unwrap().is_empty());
        assert_eq!(*pull(&mut ms, 1000).last().unwrap(), 0.0);
        assert_eq!(*pull(&mut cs, 1000).last().unwrap(), 0.0);
    }

    #[test]
    fn unknown_sound_is_err() {
        let b = Soundboard::new();
        assert!(b.play("nope").is_err());
    }

    #[test]
    fn no_outputs_is_ok() {
        let b = Soundboard::new();
        b.apply(&settings(vec![def("none", 100, OnRepeat::Restart)]));
        assert!(b.play("none").is_ok());
    }

    #[test]
    fn mic_gain_kept_across_apply() {
        let b = Soundboard::new();
        let s = SoundSettings { mic_gain: 100, ..SoundSettings::default() };
        b.apply(&s);
        assert!(near(b.change_mic_gain(-20), 0.8));
        b.apply(&s);
        assert!(near(b.mic_gain(), 0.8));
        b.apply(&SoundSettings { mic_gain: 50, ..s });
        assert!(near(b.mic_gain(), 0.5));
        assert!(b.toggle_mic_mute());
        assert_eq!(b.mic_gain(), 0.0);
        assert!(!b.toggle_mic_mute());
    }

    #[test]
    fn master_kept_across_apply() {
        let b = Soundboard::new();
        let s = SoundSettings::default();
        b.apply(&s);
        assert!(near(b.change_master(-20), 0.8));
        b.apply(&s);
        assert!(near(b.master().level(), 0.8));
        b.apply(&SoundSettings { master_volume: 50, ..s });
        assert!(near(b.master().level(), 0.5));
    }

    #[test]
    fn master_scales_playing_sound() {
        let (b, mut ms, mut cs) = setup(&settings(vec![def("mas", 100, OnRepeat::Restart)]));
        b.play("mas").unwrap();
        let v = v0(&b, "mas");
        assert!(near(first_nonzero(&mut ms), v));
        assert!(near(first_nonzero(&mut cs), v));
        b.change_master(-50);
        // trames suivantes : on lit l'echantillon de position connue via le clip
        let (m, c) = (pull(&mut ms, 4), pull(&mut cs, 4));
        let clip = b.library.load().get("mas").unwrap().clip.clone();
        for out in [m, c] {
            let last = out[3];
            let idx = clip.samples.iter().position(|s| near(*s * 0.5, last)).expect("moitie");
            assert!(idx < 10, "idx {idx}");
        }
        assert!(b.toggle_master_mute());
        // la sourdine joue a la trame suivante : on regarde la fin du bloc
        assert_eq!(pull(&mut ms, 4)[3], 0.0);
        assert_eq!(pull(&mut cs, 4)[3], 0.0);
        assert!(!b.toggle_master_mute());
        assert!(pull(&mut cs, 4)[3] != 0.0);
    }
}

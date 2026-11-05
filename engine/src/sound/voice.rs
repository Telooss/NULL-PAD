//! Voix : lit un tampon partage, applique gain et volume maitre, fondu a l'arret.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::sync::Arc;
use std::time::Duration;

use rodio::{ChannelCount, SampleRate, Source};

#[derive(Clone)]
pub struct Clip {
    pub samples: Arc<[f32]>,
    pub channels: u16,
    pub rate: u32,
}

pub struct VoiceState {
    stop: AtomicBool,
    live: AtomicU32,
}

impl VoiceState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { stop: AtomicBool::new(false), live: AtomicU32::new(0) })
    }
    pub fn stop(&self) {
        self.stop.store(true, Relaxed);
    }
    pub fn is_playing(&self) -> bool {
        self.live.load(Relaxed) > 0 && !self.stop.load(Relaxed)
    }
}

pub struct Master {
    level: AtomicU32, // bits f32
    muted: AtomicBool,
}

impl Master {
    pub fn new(level: f32) -> Arc<Self> {
        Arc::new(Self { level: AtomicU32::new(level.to_bits()), muted: AtomicBool::new(false) })
    }
    pub fn get(&self) -> f32 {
        if self.muted.load(Relaxed) { 0.0 } else { self.level() }
    }
    pub fn set(&self, v: f32) {
        self.level.store(v.to_bits(), Relaxed);
    }
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Relaxed))
    }
    /// Bascule la sourdine, renvoie le nouvel etat (true = coupe).
    pub fn toggle_mute(&self) -> bool {
        !self.muted.fetch_xor(true, Relaxed)
    }
}

pub struct Voice {
    samples: Arc<[f32]>,
    channels: ChannelCount,
    rate: SampleRate,
    pos: usize,
    gain: f32,
    master: Arc<Master>,
    state: Arc<VoiceState>,
    frame_mul: f32,
    fade_total: u32,
    fade_left: Option<u32>,
    done: bool, // `live` deja decremente
}

impl Voice {
    pub fn new(clip: &Clip, gain: f32, master: Arc<Master>, state: Arc<VoiceState>) -> Voice {
        debug_assert!(clip.channels >= 1 && clip.rate >= 1);
        state.live.fetch_add(1, Relaxed);
        Voice {
            samples: clip.samples.clone(),
            channels: ChannelCount::new(clip.channels).unwrap(),
            rate: SampleRate::new(clip.rate).unwrap(),
            pos: 0,
            gain,
            master,
            state,
            frame_mul: 0.0,
            fade_total: clip.rate * 5 / 1000, // FADE_FRAMES
            fade_left: None,
            done: false,
        }
    }

    fn finish(&mut self) -> Option<f32> {
        if !self.done {
            self.done = true;
            self.state.live.fetch_sub(1, Relaxed);
        }
        None
    }
}

impl Iterator for Voice {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.done {
            return None;
        }
        if self.pos.is_multiple_of(self.channels.get() as usize) {
            // debut de trame : seul endroit ou le fondu et le volume changent
            if self.pos >= self.samples.len() {
                return self.finish();
            }
            if self.fade_left.is_none() && self.state.stop.load(Relaxed) {
                self.fade_left = Some(self.fade_total);
            }
            let fade = match &mut self.fade_left {
                Some(0) => return self.finish(),
                Some(n) => {
                    let g = *n as f32 / (self.fade_total + 1) as f32;
                    *n -= 1;
                    g
                }
                None => 1.0,
            };
            self.frame_mul = self.gain * self.master.get() * fade;
        }
        let s = self.samples[self.pos] * self.frame_mul;
        self.pos += 1;
        Some(s)
    }
}

impl Drop for Voice {
    fn drop(&mut self) {
        self.finish();
    }
}

impl Source for Voice {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        self.channels
    }
    fn sample_rate(&self) -> SampleRate {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(v: f32, n: usize, channels: u16, rate: u32) -> Clip {
        Clip { samples: vec![v; n].into(), channels, rate }
    }

    #[test]
    fn plays_all_samples_with_gain() {
        let mut v = Voice::new(&clip(0.5, 8, 2, 48000), 0.5, Master::new(1.0), VoiceState::new());
        for _ in 0..8 {
            assert_eq!(v.next(), Some(0.25));
        }
        assert_eq!(v.next(), None);
    }

    #[test]
    fn master_changes_live() {
        let m = Master::new(1.0);
        let mut v = Voice::new(&clip(1.0, 8, 2, 48000), 1.0, m.clone(), VoiceState::new());
        assert_eq!((v.next(), v.next()), (Some(1.0), Some(1.0)));
        m.set(0.5);
        assert_eq!((v.next(), v.next()), (Some(0.5), Some(0.5)));
        assert!(m.toggle_mute());
        assert_eq!(m.level(), 0.5);
        assert_eq!((v.next(), v.next()), (Some(0.0), Some(0.0)));
        assert!(!m.toggle_mute());
    }

    #[test]
    fn live_counts_voices() {
        let st = VoiceState::new();
        let c = clip(1.0, 4, 2, 48000);
        let mut a = Voice::new(&c, 1.0, Master::new(1.0), st.clone());
        let b = Voice::new(&c, 1.0, Master::new(1.0), st.clone());
        assert_eq!(st.live.load(Relaxed), 2);
        assert!(st.is_playing());
        while a.next().is_some() {}
        assert_eq!(st.live.load(Relaxed), 1);
        drop(b);
        drop(a); // deja terminee : pas de double decrement
        assert_eq!(st.live.load(Relaxed), 0);
        assert!(!st.is_playing());
    }

    #[test]
    fn stop_fades_then_ends() {
        let st = VoiceState::new();
        let mut v = Voice::new(&clip(1.0, 2 * 48000, 2, 48000), 1.0, Master::new(1.0), st.clone());
        for _ in 0..10 {
            assert_eq!(v.next(), Some(1.0));
        }
        st.stop();
        let rest: Vec<f32> = v.by_ref().collect();
        assert!((rest.len() as i64 - 480).abs() <= 2, "len {}", rest.len());
        assert!(rest.windows(2).all(|w| w[1] <= w[0]));
        assert!(rest.last().unwrap() < &0.01);
        assert!(rest.chunks(2).all(|f| f.len() < 2 || f[0] == f[1]));
        assert_eq!(v.next(), None);
        assert_eq!(st.live.load(Relaxed), 0);
    }
}

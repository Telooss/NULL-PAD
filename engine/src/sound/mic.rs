//! Micro via l'appli : pont non bloquant entre le rappel d'entree (producteur)
//! et le mixeur de sortie (MicSource, tire dans le rappel de sortie).

use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use rodio::{ChannelCount, SampleRate, Source};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::sound::board::Soundboard;
use crate::sound::voice::Master;

const TARGET_MS: usize = 15; // tampon vise apres amorcage ou rattrapage
const MAX_MS: usize = 50; // au-dela, la derive est rattrapee
const CAPACITY_MS: usize = 200;

pub struct MicSource {
    cons: Consumer<f32>,
    board: Arc<Soundboard>,
    master: Arc<Master>,
    channels: ChannelCount,
    rate: SampleRate,
    target: usize, // en echantillons, multiple de `channels`
    max: usize,
    primed: bool,
    pos: usize, // echantillon courant dans la trame
    frame_mul: f32,
}

/// Echantillons correspondant a `ms` (trames entieres).
fn samples_for(ms: usize, rate: u32, channels: u16) -> usize {
    ms * rate as usize / 1000 * channels as usize
}

pub fn mic_channel(channels: u16, rate: u32, board: Arc<Soundboard>) -> (Producer<f32>, MicSource) {
    let (prod, cons) = RingBuffer::new(samples_for(CAPACITY_MS, rate, channels));
    let src = MicSource {
        cons,
        master: board.master(),
        board,
        channels: NonZero::new(channels).expect("channels >= 1"),
        rate: NonZero::new(rate).expect("rate >= 1"),
        target: samples_for(TARGET_MS, rate, channels),
        max: samples_for(MAX_MS, rate, channels),
        primed: false,
        pos: 0,
        frame_mul: 0.0,
    };
    (prod, src)
}

impl MicSource {
    #[cfg(test)]
    fn queued(&self) -> usize {
        self.cons.slots()
    }
}

impl Iterator for MicSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.pos == 0 {
            // debut de trame : seul endroit ou l'on amorce, rattrape et lit les volumes
            let n = self.cons.slots();
            if !self.primed {
                if n < self.target {
                    return Some(0.0);
                }
                self.primed = true;
            }
            if n == 0 {
                self.primed = false;
                return Some(0.0);
            }
            if n > self.max {
                let ch = self.channels.get() as usize;
                if let Ok(chunk) = self.cons.read_chunk((n - self.target) / ch * ch) {
                    chunk.commit_all();
                }
            }
            self.frame_mul = self.board.mic_gain() * self.master.get();
        }
        // vide en milieu de trame (producteur coupe net) : silence
        let s = self.cons.pop().unwrap_or(0.0);
        self.pos = (self.pos + 1) % self.channels.get() as usize;
        Some(s * self.frame_mul)
    }
}

impl Source for MicSource {
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

    const RATE: u32 = 48000;

    fn setup() -> (Producer<f32>, MicSource, Arc<Soundboard>) {
        let b = Soundboard::new();
        let (p, s) = mic_channel(2, RATE, b.clone());
        (p, s, b)
    }

    /// Pousse `ms` de signal : gauche +1, droite -1.
    fn push_ms(p: &mut Producer<f32>, ms: usize) {
        for _ in 0..ms * RATE as usize / 1000 {
            p.push(1.0).unwrap();
            p.push(-1.0).unwrap();
        }
    }

    fn read(s: &mut MicSource, n: usize) -> Vec<f32> {
        (0..n).map(|_| s.next().unwrap()).collect()
    }

    #[test]
    fn silence_until_primed() {
        let (mut p, mut s, _b) = setup();
        push_ms(&mut p, 10);
        assert!(read(&mut s, 20).iter().all(|x| *x == 0.0));
        push_ms(&mut p, 10);
        assert_eq!(read(&mut s, 4), [1.0, -1.0, 1.0, -1.0]);
    }

    #[test]
    fn underrun_gives_silence_and_reprimes() {
        let (mut p, mut s, _b) = setup();
        push_ms(&mut p, 20);
        let n = 20 * RATE as usize / 1000 * 2;
        assert!(read(&mut s, n).iter().all(|x| x.abs() == 1.0));
        assert!(read(&mut s, 10).iter().all(|x| *x == 0.0)); // vide : desamorce
        push_ms(&mut p, 10);
        assert!(read(&mut s, 10).iter().all(|x| *x == 0.0)); // moins de 15 ms
        push_ms(&mut p, 10);
        assert_eq!(read(&mut s, 2), [1.0, -1.0]);
    }

    #[test]
    fn drift_is_trimmed_frame_aligned() {
        let (mut p, mut s, _b) = setup();
        push_ms(&mut p, 100);
        let first = read(&mut s, 2);
        let target = 15 * RATE as usize / 1000 * 2;
        assert!(s.queued().abs_diff(target) <= 2, "reste {}", s.queued());
        let left = s.queued();
        let rest = read(&mut s, left);
        let all: Vec<f32> = first.into_iter().chain(rest).collect();
        assert!(all.chunks(2).all(|f| f == [1.0, -1.0]));
    }

    #[test]
    fn gain_and_mute_apply() {
        let (mut p, mut s, b) = setup();
        push_ms(&mut p, 100);
        assert_eq!(read(&mut s, 2), [1.0, -1.0]);
        b.change_mic_gain(-50);
        assert_eq!(read(&mut s, 2), [0.5, -0.5]);
        assert!(b.toggle_mic_mute());
        assert_eq!(read(&mut s, 2), [0.0, 0.0]);
        assert!(!b.toggle_mic_mute());
        b.change_master(-50);
        assert_eq!(read(&mut s, 2), [0.25, -0.25]);
    }

    #[test]
    fn never_returns_none() {
        let (p, mut s, _b) = setup();
        drop(p);
        assert!(read(&mut s, 1000).iter().all(|x| *x == 0.0));
    }
}

//! Machine a etats pure : appui / appui long / double appui. Le temps est injecte.
use crate::codes::InputId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trigger {
    Press,
    Hold,
    DoubleTap,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Timing {
    pub hold_ms: u64,
    pub double_ms: u64,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            hold_ms: 300,
            double_ms: 250,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fire {
    pub profile: u8,
    pub input: InputId,
    pub trigger: Trigger,
}

/// Quelles liaisons existent pour cette touche (fourni par la config).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Wants {
    pub hold: bool,
    pub double: bool,
}

#[derive(Clone, Copy)]
enum State {
    /// Touche enfoncee, rien tire encore.
    Down { since: Instant, wants: Wants },
    /// Relachee, fenetre de double appui ouverte depuis `at`.
    Released { at: Instant },
    /// Deja tire (Hold ou DoubleTap) : le relachement ne fait rien.
    Consumed,
}

pub struct Triggers {
    timing: Timing,
    states: HashMap<(u8, InputId), State>,
}

fn fire(profile: u8, input: InputId, trigger: Trigger) -> Fire {
    Fire {
        profile,
        input,
        trigger,
    }
}

impl Triggers {
    pub fn new(timing: Timing) -> Self {
        Self {
            timing,
            states: HashMap::new(),
        }
    }

    fn hold(&self) -> Duration {
        Duration::from_millis(self.timing.hold_ms)
    }

    fn double(&self) -> Duration {
        Duration::from_millis(self.timing.double_ms)
    }

    pub fn down(&mut self, profile: u8, input: InputId, wants: Wants, now: Instant) -> Vec<Fire> {
        if input.is_rotation() || (!wants.hold && !wants.double) {
            return vec![fire(profile, input, Trigger::Press)];
        }
        let mut out = Vec::new();
        match self.states.get(&(profile, input)) {
            Some(State::Released { at }) if now < *at + self.double() => {
                self.states.insert((profile, input), State::Consumed);
                return vec![fire(profile, input, Trigger::DoubleTap)];
            }
            // Fenetre expiree sans tick : le premier appui se resout en Press.
            Some(State::Released { .. }) => out.push(fire(profile, input, Trigger::Press)),
            _ => {}
        }
        self.states
            .insert((profile, input), State::Down { since: now, wants });
        out
    }

    pub fn up(&mut self, profile: u8, input: InputId, now: Instant) -> Vec<Fire> {
        let key = (profile, input);
        match self.states.get(&key).copied() {
            Some(State::Down { since, wants }) => {
                if wants.hold && now >= since + self.hold() {
                    self.states.insert(key, State::Consumed);
                    vec![fire(profile, input, Trigger::Hold)]
                } else if wants.double {
                    self.states.insert(key, State::Released { at: now });
                    vec![]
                } else {
                    self.states.remove(&key);
                    vec![fire(profile, input, Trigger::Press)]
                }
            }
            Some(State::Consumed) => {
                self.states.remove(&key);
                vec![]
            }
            _ => vec![],
        }
    }

    pub fn tick(&mut self, now: Instant) -> Vec<Fire> {
        let (hold, double) = (self.hold(), self.double());
        let mut out = Vec::new();
        self.states.retain(|&(profile, input), st| match *st {
            State::Down { since, wants } if wants.hold && now >= since + hold => {
                out.push(fire(profile, input, Trigger::Hold));
                *st = State::Consumed;
                true
            }
            State::Released { at } if now >= at + double => {
                out.push(fire(profile, input, Trigger::Press));
                false
            }
            _ => true,
        });
        out
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.states
            .values()
            .filter_map(|st| match *st {
                State::Down { since, wants } if wants.hold => Some(since + self.hold()),
                State::Released { at } => Some(at + self.double()),
                _ => None,
            })
            .min()
    }

    pub fn reset(&mut self) {
        self.states.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: InputId = InputId(0);
    const H: Wants = Wants {
        hold: true,
        double: false,
    };
    const D: Wants = Wants {
        hold: false,
        double: true,
    };
    const HD: Wants = Wants {
        hold: true,
        double: true,
    };

    fn f(trigger: Trigger) -> Vec<Fire> {
        vec![fire(1, K, trigger)]
    }
    fn ms(t0: Instant, n: u64) -> Instant {
        t0 + Duration::from_millis(n)
    }
    fn tr() -> (Triggers, Instant) {
        (Triggers::new(Timing::default()), Instant::now())
    }

    #[test]
    fn press_fires_on_down_when_nothing_else_bound() {
        let (mut t, t0) = tr();
        assert_eq!(t.down(1, K, Wants::default(), t0), f(Trigger::Press));
        assert!(t.up(1, K, ms(t0, 10)).is_empty());
    }

    #[test]
    fn rotation_ignores_hold_and_double() {
        let (mut t, t0) = tr();
        let r = t.down(1, InputId(23), HD, t0);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].trigger, Trigger::Press);
        assert!(t.next_deadline().is_none());
    }

    #[test]
    fn hold_after_300ms() {
        let (mut t, t0) = tr();
        assert!(t.down(1, K, H, t0).is_empty());
        assert_eq!(t.next_deadline(), Some(ms(t0, 300)));
        assert!(t.tick(ms(t0, 299)).is_empty());
        assert_eq!(t.tick(ms(t0, 300)), f(Trigger::Hold));
        assert!(t.up(1, K, ms(t0, 400)).is_empty());
        assert!(t.next_deadline().is_none());
    }

    #[test]
    fn short_press_with_hold_bound_fires_press_on_up() {
        let (mut t, t0) = tr();
        t.down(1, K, H, t0);
        assert_eq!(t.up(1, K, ms(t0, 100)), f(Trigger::Press));
    }

    #[test]
    fn double_tap_within_250() {
        let (mut t, t0) = tr();
        assert!(t.down(1, K, D, t0).is_empty());
        assert!(t.up(1, K, ms(t0, 50)).is_empty());
        assert_eq!(t.down(1, K, D, ms(t0, 200)), f(Trigger::DoubleTap));
        assert!(t.up(1, K, ms(t0, 250)).is_empty());
        assert!(t.tick(ms(t0, 1000)).is_empty());
    }

    #[test]
    fn single_tap_with_double_bound_fires_after_window() {
        let (mut t, t0) = tr();
        t.down(1, K, D, t0);
        t.up(1, K, ms(t0, 50));
        assert_eq!(t.next_deadline(), Some(ms(t0, 300)));
        assert!(t.tick(ms(t0, 299)).is_empty());
        assert_eq!(t.tick(ms(t0, 300)), f(Trigger::Press));
        assert!(t.next_deadline().is_none());
    }

    #[test]
    fn hold_and_double_both_bound() {
        let (mut t, t0) = tr();
        t.down(1, K, HD, t0);
        assert_eq!(t.tick(ms(t0, 300)), f(Trigger::Hold));
        assert!(t.up(1, K, ms(t0, 350)).is_empty());
        t.down(1, K, HD, ms(t0, 1000));
        t.up(1, K, ms(t0, 1050));
        assert_eq!(t.down(1, K, HD, ms(t0, 1100)), f(Trigger::DoubleTap));
        assert!(t.tick(ms(t0, 1500)).is_empty());
        assert!(t.up(1, K, ms(t0, 1600)).is_empty());
    }

    #[test]
    fn reset_cancels_pending_hold() {
        let (mut t, t0) = tr();
        t.down(1, K, H, t0);
        t.reset();
        assert!(t.tick(ms(t0, 1000)).is_empty());
        assert!(t.next_deadline().is_none());
    }
}

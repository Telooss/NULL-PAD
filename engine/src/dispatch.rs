//! Repartiteur : usage HID -> declencheur -> action, sans jamais bloquer l'appelant.
use crate::actions::audio::{change_volume, toggle_mute, Endpoint};
use crate::actions::keys::{parse_shortcut, send_media, send_shortcut, send_text};
use crate::actions::launch::{command, launch, open};
use crate::codes::{decode, Code, InputId};
use crate::config::{Action, Config, Step};
use crate::ipc::{Event, Events};
use crate::midi::{cc, note_off, note_on, Midi};
use crate::obs::ObsHandle;
use crate::report::Edge;
use crate::sound::board::Soundboard;
use crate::triggers::{Fire, Trigger, Triggers};
use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Execute une action unitaire (jamais Macro ni Toggle : le Dispatcher les resout).
pub trait Executor: Send + Sync {
    fn run(&self, action: &Action) -> Result<(), String>;
}

/// Nom de la variante, envoye a l'interface comme `kind`.
pub fn kind(a: &Action) -> &'static str {
    match a {
        Action::Shortcut { .. } => "Shortcut",
        Action::Text { .. } => "Text",
        Action::Macro { .. } => "Macro",
        Action::Launch { .. } => "Launch",
        Action::Open { .. } => "Open",
        Action::Command { .. } => "Command",
        Action::Media { .. } => "Media",
        Action::Volume { .. } => "Volume",
        Action::VolumeMute => "VolumeMute",
        Action::MicVolume { .. } => "MicVolume",
        Action::MicMute => "MicMute",
        Action::Toggle { .. } => "Toggle",
        Action::Sound { .. } => "Sound",
        Action::StopSounds => "StopSounds",
        Action::SoundVolume { .. } => "SoundVolume",
        Action::SoundMute => "SoundMute",
        Action::ObsScene { .. } => "ObsScene",
        Action::ObsMute { .. } => "ObsMute",
        Action::ObsStream { .. } => "ObsStream",
        Action::ObsRecord { .. } => "ObsRecord",
        Action::ObsReplaySave => "ObsReplaySave",
        Action::MidiNote { .. } => "MidiNote",
        Action::MidiCc { .. } => "MidiCc",
    }
}

pub struct SystemExecutor {
    pub sound: Arc<Soundboard>,
    pub obs: ObsHandle,
    pub midi: Arc<Midi>,
}

impl Executor for SystemExecutor {
    fn run(&self, action: &Action) -> Result<(), String> {
        match action {
            Action::Shortcut { keys } => parse_shortcut(keys).map(|k| send_shortcut(&k)),
            Action::Text { text } => {
                send_text(text);
                Ok(())
            }
            Action::Media { key } => {
                send_media(*key);
                Ok(())
            }
            Action::Volume { delta } => change_volume(Endpoint::Output, *delta).map(drop),
            Action::VolumeMute => toggle_mute(Endpoint::Output).map(drop),
            Action::MicVolume { delta } if self.sound.mic_via_app() => {
                tracing::debug!("gain micro: {}", self.sound.change_mic_gain(*delta));
                Ok(())
            }
            Action::MicMute if self.sound.mic_via_app() => {
                tracing::debug!("micro coupe: {}", self.sound.toggle_mic_mute());
                Ok(())
            }
            Action::MicVolume { delta } => change_volume(Endpoint::Mic, *delta).map(drop),
            Action::MicMute => toggle_mute(Endpoint::Mic).map(drop),
            Action::Launch { program, args } => launch(program, args),
            Action::Open { target } => open(target),
            Action::Command { line } => command(line),
            Action::Sound { id } => self.sound.play(id),
            Action::StopSounds => {
                self.sound.stop_all();
                Ok(())
            }
            Action::SoundVolume { delta } => {
                tracing::debug!("volume soundboard: {}", self.sound.change_master(*delta));
                Ok(())
            }
            Action::SoundMute => {
                tracing::debug!("soundboard coupee: {}", self.sound.toggle_master_mute());
                Ok(())
            }
            Action::ObsScene { .. }
            | Action::ObsMute { .. }
            | Action::ObsStream { .. }
            | Action::ObsRecord { .. }
            | Action::ObsReplaySave => {
                self.obs.send(action.clone());
                Ok(())
            }
            Action::MidiNote {
                channel,
                note,
                velocity,
            } => self.midi.send(&[
                note_on(*channel, *note, *velocity),
                note_off(*channel, *note),
            ]),
            Action::MidiCc {
                channel,
                cc: n,
                value,
            } => self.midi.send(&[cc(*channel, *n, *value)]),
            Action::Macro { .. } | Action::Toggle { .. } => Err("action non unitaire".into()),
        }
    }
}

/// Execute toutes les etapes ; renvoie la premiere erreur (chaque erreur est journalisee).
fn run_steps(exec: &dyn Executor, steps: Vec<Step>) -> Result<(), String> {
    let mut first = Ok(());
    for s in steps {
        let res = match s {
            Step::Wait { ms } => {
                thread::sleep(Duration::from_millis(ms));
                continue;
            }
            Step::Do {
                action: Action::Macro { steps },
            } => run_steps(exec, steps),
            Step::Do { action } => exec.run(&action).inspect_err(|e| {
                tracing::error!("action en erreur: {e}");
            }),
        };
        if first.is_ok() {
            first = res;
        }
    }
    first
}

/// Ce qu'il faut pour rapporter le resultat d'une action a l'interface.
struct Origin {
    events: Events,
    input: &'static str,
    trigger: Trigger,
}

impl Origin {
    fn report(&self, kind: &str, res: Result<(), String>) {
        self.events.emit(&Event::Action {
            input: self.input.into(),
            trigger: self.trigger,
            kind: kind.into(),
            ok: res.is_ok(),
            error: res.err(),
        });
    }

    /// Action unitaire : journalise l'erreur et la rapporte.
    fn run(&self, exec: &dyn Executor, a: &Action) {
        let res = exec.run(a);
        if let Err(e) = &res {
            tracing::error!("action en erreur: {e}");
        }
        self.report(kind(a), res);
    }
}

pub struct Dispatcher {
    config: Arc<ArcSwap<Config>>,
    exec: Arc<dyn Executor>,
    triggers: Triggers,
    profile: Option<u8>,
    toggles: HashMap<(u8, InputId), bool>,
    worker: Sender<(Action, Origin)>,
    events: Events,
}

impl Dispatcher {
    pub fn new(config: Arc<ArcSwap<Config>>, exec: Arc<dyn Executor>, events: Events) -> Self {
        let (worker, rx) = channel::<(Action, Origin)>();
        let e = exec.clone();
        thread::spawn(move || {
            for (a, o) in rx {
                // Une panique ne doit pas tuer le worker : sinon plus aucune action clavier.
                if std::panic::catch_unwind(AssertUnwindSafe(|| o.run(e.as_ref(), &a))).is_err() {
                    tracing::error!("panique dans une action: {a:?}");
                    o.report(kind(&a), Err("panique".into()));
                }
            }
        });
        let triggers = Triggers::new(config.load().timing);
        Self {
            config,
            exec,
            triggers,
            profile: None,
            toggles: HashMap::new(),
            worker,
            events,
        }
    }

    pub fn events(&self) -> &Events {
        &self.events
    }

    pub fn current_profile(&self) -> Option<u8> {
        self.profile
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.triggers.next_deadline()
    }

    pub fn device_lost(&mut self) {
        self.triggers.reset();
    }

    pub fn on_edge(&mut self, edge: Edge, now: Instant) {
        let fires = match edge {
            Edge::Down(u) => match decode(u) {
                Some(Code::Action { profile, input }) => {
                    self.key(input, true, profile);
                    let cfg = self.config.load();
                    let wants = cfg.wants(profile, input);
                    self.triggers.set_timing(cfg.timing);
                    self.triggers.down(profile, input, wants, now)
                }
                Some(Code::ProfileAnnounce(p)) => {
                    self.profile = Some(p);
                    self.events.emit(&Event::Profile { profile: p });
                    return;
                }
                None => return,
            },
            Edge::Up(u) => match decode(u) {
                Some(Code::Action { profile, input }) => {
                    self.key(input, false, profile);
                    self.triggers.up(profile, input, now)
                }
                _ => return,
            },
        };
        fires.into_iter().for_each(|f| self.fire(f));
    }

    fn key(&self, input: InputId, down: bool, profile: u8) {
        self.events.emit(&Event::Key {
            input: input.name().into(),
            down,
            profile,
        });
    }

    pub fn on_tick(&mut self, now: Instant) {
        self.triggers
            .tick(now)
            .into_iter()
            .for_each(|f| self.fire(f));
    }

    fn fire(&mut self, f: Fire) {
        let Some(action) = self
            .config
            .load()
            .lookup(f.profile, f.input, f.trigger)
            .cloned()
        else {
            return;
        };
        let action = match action {
            Action::Toggle { on, off } => {
                let state = self.toggles.entry((f.profile, f.input)).or_insert(false);
                *state = !*state;
                if *state {
                    *on
                } else {
                    *off
                }
            }
            a => a,
        };
        let origin = Origin {
            events: self.events.clone(),
            input: f.input.name(),
            trigger: f.trigger,
        };
        self.route(action, origin);
    }

    fn route(&self, action: Action, o: Origin) {
        match action {
            Action::Macro { steps } => {
                let exec = self.exec.clone();
                thread::spawn(move || o.report("Macro", run_steps(exec.as_ref(), steps)));
            }
            Action::Launch { .. } | Action::Open { .. } | Action::Command { .. } => {
                let exec = self.exec.clone();
                thread::spawn(move || o.run(exec.as_ref(), &action));
            }
            // Si le worker est mort, on perd l'action : rien d'autre a faire.
            a => drop(self.worker.send((a, o))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::encode_action;
    use crate::config::{Binding, ObsOp, Profile};
    use crate::triggers::{Timing, Trigger};
    use std::sync::mpsc::Receiver;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Rec(Mutex<Vec<(Instant, String)>>);
    impl Executor for Rec {
        fn run(&self, a: &Action) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .push((Instant::now(), format!("{a:?}")));
            match a {
                Action::Text { text } if text.starts_with("fail") => Err(text.clone()),
                _ => Ok(()),
            }
        }
    }
    impl Rec {
        fn has(&self, s: &str) -> bool {
            self.0.lock().unwrap().iter().any(|(_, x)| x.contains(s))
        }
        fn log(&self) -> Vec<String> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .map(|(_, x)| x.clone())
                .collect()
        }
        fn wait_len(&self, n: usize) -> bool {
            let end = Instant::now() + Duration::from_millis(1000);
            while Instant::now() < end {
                if self.0.lock().unwrap().len() >= n {
                    return true;
                }
                thread::sleep(Duration::from_millis(1));
            }
            false
        }
    }

    fn text(t: &str) -> Action {
        Action::Text { text: t.into() }
    }

    fn setup(bindings: Vec<(u8, Action)>) -> (Dispatcher, Arc<Rec>) {
        let (d, rec, _) = setup_ev(bindings);
        (d, rec)
    }

    fn setup_ev(bindings: Vec<(u8, Action)>) -> (Dispatcher, Arc<Rec>, Receiver<String>) {
        let bindings = bindings
            .into_iter()
            .map(|(i, action)| Binding {
                input: InputId(i),
                trigger: Trigger::Press,
                action,
                label: None,
            })
            .collect();
        let cfg = Config {
            version: 1,
            timing: Timing::default(),
            sound: Default::default(),
            obs: Default::default(),
            midi: Default::default(),
            profiles: vec![Profile {
                id: 1,
                name: "p".into(),
                tag: "AAA".into(),
                bindings,
            }],
        };
        let rec = Arc::new(Rec::default());
        let (ev, rx) = Events::recording();
        let d = Dispatcher::new(Arc::new(ArcSwap::from_pointee(cfg)), rec.clone(), ev);
        (d, rec, rx)
    }

    fn next(rx: &Receiver<String>) -> String {
        rx.recv_timeout(Duration::from_secs(5))
            .expect("aucun evenement")
    }

    #[test]
    fn unknown_and_unbound_are_ignored() {
        let (mut d, rec) = setup(vec![(0, text("x"))]);
        let now = Instant::now();
        d.on_edge(Edge::Down(0x00E9), now);
        d.on_edge(Edge::Down(encode_action(1, InputId(5))), now);
        d.on_edge(Edge::Down(encode_action(2, InputId(0))), now);
        thread::sleep(Duration::from_millis(50));
        assert!(rec.log().is_empty());
    }

    #[test]
    fn press_dispatches_under_1ms() {
        let (mut d, rec) = setup(vec![(0, text("x"))]);
        let code = encode_action(1, InputId(0));
        let mut delays = Vec::new();
        for i in 0..50 {
            let t0 = Instant::now();
            d.on_edge(Edge::Down(code), t0);
            assert!(rec.wait_len(i + 1));
            delays.push(rec.0.lock().unwrap()[i].0 - t0);
        }
        delays.sort();
        assert!(
            delays[25] < Duration::from_millis(1),
            "median {:?}",
            delays[25]
        );
    }

    #[test]
    fn long_macro_does_not_block() {
        let mac = Action::Macro {
            steps: vec![Step::Wait { ms: 2000 }, Step::Do { action: text("a") }],
        };
        let (mut d, rec) = setup(vec![(0, mac), (1, text("b"))]);
        let t0 = Instant::now();
        d.on_edge(Edge::Down(encode_action(1, InputId(0))), t0);
        d.on_edge(Edge::Down(encode_action(1, InputId(1))), t0);
        assert!(rec.wait_len(1));
        assert!(t0.elapsed() < Duration::from_millis(50));
        assert!(rec.has("\"b\"") && !rec.has("\"a\""));
    }

    #[test]
    fn toggle_alternates() {
        let t = Action::Toggle {
            on: Box::new(text("on")),
            off: Box::new(text("off")),
        };
        let (mut d, rec) = setup(vec![(0, t)]);
        let code = encode_action(1, InputId(0));
        for n in 1..=3 {
            d.on_edge(Edge::Down(code), Instant::now());
            d.on_edge(Edge::Up(code), Instant::now());
            assert!(rec.wait_len(n));
        }
        let log = rec.log();
        assert!(
            log[0].contains("\"on\"") && log[1].contains("\"off\"") && log[2].contains("\"on\"")
        );
    }

    #[test]
    fn nested_macro_runs_inline() {
        let inner = Action::Macro {
            steps: vec![Step::Do { action: text("x") }],
        };
        let outer = Action::Macro {
            steps: vec![Step::Do { action: inner }],
        };
        let (mut d, rec) = setup(vec![(0, outer)]);
        d.on_edge(Edge::Down(encode_action(1, InputId(0))), Instant::now());
        assert!(rec.wait_len(1));
        assert!(rec.has("\"x\""));
    }

    struct PanicFirst(Rec, std::sync::atomic::AtomicBool);
    impl Executor for PanicFirst {
        fn run(&self, a: &Action) -> Result<(), String> {
            if !self.1.swap(true, std::sync::atomic::Ordering::SeqCst) {
                panic!("boom");
            }
            self.0.run(a)
        }
    }

    #[test]
    fn worker_survives_panic() {
        let cfg = setup(vec![(0, text("a")), (1, text("b"))]).0.config.clone();
        let ex = Arc::new(PanicFirst(Rec::default(), Default::default()));
        let mut d = Dispatcher::new(cfg, ex.clone(), Events::none());
        d.on_edge(Edge::Down(encode_action(1, InputId(0))), Instant::now());
        d.on_edge(Edge::Down(encode_action(1, InputId(1))), Instant::now());
        assert!(ex.0.wait_len(1));
        assert!(ex.0.has("\"b\""));
    }

    #[test]
    fn timing_is_hot_reloaded() {
        let mut cfg = (*setup(vec![]).0.config.load_full()).clone();
        cfg.profiles[0].bindings = vec![Binding {
            input: InputId(0),
            trigger: Trigger::Hold,
            action: text("h"),
            label: None,
        }];
        let swap = Arc::new(ArcSwap::from_pointee(cfg.clone()));
        let mut d = Dispatcher::new(swap.clone(), Arc::new(Rec::default()), Events::none());
        cfg.timing.hold_ms = 1000;
        swap.store(Arc::new(cfg));
        let t0 = Instant::now();
        d.on_edge(Edge::Down(encode_action(1, InputId(0))), t0);
        assert_eq!(d.next_deadline(), Some(t0 + Duration::from_millis(1000)));
    }

    #[test]
    fn sound_actions_go_through_the_worker() {
        let acts = [
            Action::Sound { id: "a".into() },
            Action::StopSounds,
            Action::SoundVolume { delta: 5 },
            Action::SoundMute,
        ];
        let (mut d, rec) = setup(
            acts.iter()
                .cloned()
                .enumerate()
                .map(|(i, a)| (i as u8, a))
                .collect(),
        );
        for i in 0..4 {
            d.on_edge(Edge::Down(encode_action(1, InputId(i))), Instant::now());
        }
        assert!(rec.wait_len(4));
        assert_eq!(
            rec.log(),
            acts.iter().map(|a| format!("{a:?}")).collect::<Vec<_>>()
        );
    }

    #[test]
    fn obs_and_midi_actions_reach_the_worker_queue() {
        // Verifie seulement le transfert au worker (executeur enregistreur), pas OBS/MIDI.
        let acts = [
            Action::ObsScene {
                scene: "Jeu".into(),
            },
            Action::ObsMute {
                input: "Micro".into(),
            },
            Action::ObsStream { op: ObsOp::Toggle },
            Action::ObsRecord { op: ObsOp::Toggle },
            Action::ObsReplaySave,
            Action::MidiNote {
                channel: 1,
                note: 60,
                velocity: 100,
            },
            Action::MidiCc {
                channel: 1,
                cc: 20,
                value: 1,
            },
        ];
        let (mut d, rec) = setup(
            acts.iter()
                .cloned()
                .enumerate()
                .map(|(i, a)| (i as u8, a))
                .collect(),
        );
        for i in 0..7 {
            d.on_edge(Edge::Down(encode_action(1, InputId(i))), Instant::now());
        }
        assert!(rec.wait_len(7));
        assert_eq!(
            rec.log(),
            acts.iter().map(|a| format!("{a:?}")).collect::<Vec<_>>()
        );
    }

    #[test]
    fn announce_sets_profile() {
        let (mut d, _) = setup(vec![]);
        d.on_edge(Edge::Down(0x0603), Instant::now());
        assert_eq!(d.current_profile(), Some(3));
    }

    #[test]
    fn emits_key_on_every_edge_and_profile_on_announce() {
        let (mut d, _, rx) = setup_ev(vec![]);
        let code = encode_action(2, InputId(5));
        d.on_edge(Edge::Down(code), Instant::now());
        d.on_edge(Edge::Up(code), Instant::now());
        d.on_edge(Edge::Down(0x0603), Instant::now());
        assert_eq!(
            next(&rx),
            "{\"type\":\"key\",\"input\":\"K05\",\"down\":true,\"profile\":2}\n"
        );
        assert_eq!(
            next(&rx),
            "{\"type\":\"key\",\"input\":\"K05\",\"down\":false,\"profile\":2}\n"
        );
        assert_eq!(next(&rx), "{\"type\":\"profile\",\"profile\":3}\n");
    }

    fn actions(rx: &Receiver<String>, n: usize) -> Vec<String> {
        let end = Instant::now() + Duration::from_secs(5);
        let mut v = Vec::new();
        while v.len() < n && Instant::now() < end {
            if let Ok(l) = rx.recv_timeout(Duration::from_millis(50)) {
                if l.contains("\"type\":\"action\"") {
                    v.push(l);
                }
            }
        }
        v
    }

    #[test]
    fn emits_action_result_with_kind_and_error() {
        let (mut d, _, rx) = setup_ev(vec![(0, text("ok")), (1, text("fail: boom"))]);
        d.on_edge(Edge::Down(encode_action(1, InputId(0))), Instant::now());
        assert_eq!(
            actions(&rx, 1),
            ["{\"type\":\"action\",\"input\":\"K00\",\"trigger\":\"Press\",\"kind\":\"Text\",\"ok\":true,\"error\":null}\n"]
        );
        d.on_edge(Edge::Down(encode_action(1, InputId(1))), Instant::now());
        assert_eq!(
            actions(&rx, 1),
            ["{\"type\":\"action\",\"input\":\"K01\",\"trigger\":\"Press\",\"kind\":\"Text\",\"ok\":false,\"error\":\"fail: boom\"}\n"]
        );
    }

    #[test]
    fn macro_emits_once_with_first_error_and_toggle_emits_resolved_kind() {
        let mac = Action::Macro {
            steps: vec![
                Step::Do { action: text("a") },
                Step::Do {
                    action: text("fail1"),
                },
                Step::Do {
                    action: text("fail2"),
                },
            ],
        };
        let t = Action::Toggle {
            on: Box::new(Action::SoundMute),
            off: Box::new(text("off")),
        };
        let (mut d, rec, rx) = setup_ev(vec![(0, mac), (1, t)]);
        d.on_edge(Edge::Down(encode_action(1, InputId(0))), Instant::now());
        let got = actions(&rx, 1);
        assert!(rec.wait_len(3), "toutes les etapes s'executent");
        assert_eq!(
            got,
            ["{\"type\":\"action\",\"input\":\"K00\",\"trigger\":\"Press\",\"kind\":\"Macro\",\"ok\":false,\"error\":\"fail1\"}\n"]
        );
        d.on_edge(Edge::Down(encode_action(1, InputId(1))), Instant::now());
        assert!(actions(&rx, 1)[0].contains("\"kind\":\"SoundMute\",\"ok\":true"));
        thread::sleep(Duration::from_millis(200));
        assert!(
            rx.try_iter().all(|l| !l.contains("action")),
            "un seul evenement par action"
        );
    }
}

//! Repartiteur : usage HID -> declencheur -> action, sans jamais bloquer l'appelant.
use crate::actions::audio::{change_volume, toggle_mute, Endpoint};
use crate::actions::keys::{parse_shortcut, send_media, send_shortcut, send_text};
use crate::actions::launch::{command, launch, open};
use crate::codes::{decode, Code, InputId};
use crate::config::{Action, Config, Step};
use crate::report::Edge;
use crate::triggers::{Fire, Triggers};
use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Execute une action unitaire (jamais Macro ni Toggle : le Dispatcher les resout).
pub trait Executor: Send + Sync {
    fn run(&self, action: &Action);
}

pub struct SystemExecutor;

impl Executor for SystemExecutor {
    fn run(&self, action: &Action) {
        let res: Result<(), String> = match action {
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
            Action::MicVolume { delta } => change_volume(Endpoint::Mic, *delta).map(drop),
            Action::MicMute => toggle_mute(Endpoint::Mic).map(drop),
            Action::Launch { program, args } => launch(program, args),
            Action::Open { target } => open(target),
            Action::Command { line } => command(line),
            Action::Sound { .. }
            | Action::StopSounds
            | Action::SoundVolume { .. }
            | Action::SoundMute => Err("soundboard non branchee".into()),
            Action::Macro { .. } | Action::Toggle { .. } => Err("action non unitaire".into()),
        };
        if let Err(e) = res {
            tracing::error!("action en erreur: {e}");
        }
    }
}

fn run_steps(exec: &dyn Executor, steps: Vec<Step>) {
    for s in steps {
        match s {
            Step::Wait { ms } => thread::sleep(Duration::from_millis(ms)),
            Step::Do {
                action: Action::Macro { steps },
            } => run_steps(exec, steps),
            Step::Do { action } => exec.run(&action),
        }
    }
}

pub struct Dispatcher {
    config: Arc<ArcSwap<Config>>,
    exec: Arc<dyn Executor>,
    triggers: Triggers,
    profile: Option<u8>,
    toggles: HashMap<(u8, InputId), bool>,
    worker: Sender<Action>,
}

impl Dispatcher {
    pub fn new(config: Arc<ArcSwap<Config>>, exec: Arc<dyn Executor>) -> Self {
        let (worker, rx) = channel::<Action>();
        let e = exec.clone();
        thread::spawn(move || {
            for a in rx {
                // Une panique ne doit pas tuer le worker : sinon plus aucune action clavier.
                if std::panic::catch_unwind(AssertUnwindSafe(|| e.run(&a))).is_err() {
                    tracing::error!("panique dans une action: {a:?}");
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
        }
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
                    let cfg = self.config.load();
                    let wants = cfg.wants(profile, input);
                    self.triggers.set_timing(cfg.timing);
                    self.triggers.down(profile, input, wants, now)
                }
                Some(Code::ProfileAnnounce(p)) => {
                    self.profile = Some(p);
                    return;
                }
                None => return,
            },
            Edge::Up(u) => match decode(u) {
                Some(Code::Action { profile, input }) => self.triggers.up(profile, input, now),
                _ => return,
            },
        };
        fires.into_iter().for_each(|f| self.fire(f));
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
        self.route(action);
    }

    fn route(&self, action: Action) {
        match action {
            Action::Macro { steps } => {
                let exec = self.exec.clone();
                thread::spawn(move || run_steps(exec.as_ref(), steps));
            }
            Action::Launch { .. } | Action::Open { .. } | Action::Command { .. } => {
                let exec = self.exec.clone();
                thread::spawn(move || exec.run(&action));
            }
            // Si le worker est mort, on perd l'action : rien d'autre a faire.
            a => drop(self.worker.send(a)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes::encode_action;
    use crate::config::{Binding, Profile};
    use crate::triggers::{Timing, Trigger};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Rec(Mutex<Vec<(Instant, String)>>);
    impl Executor for Rec {
        fn run(&self, a: &Action) {
            self.0
                .lock()
                .unwrap()
                .push((Instant::now(), format!("{a:?}")));
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
        let bindings = bindings
            .into_iter()
            .map(|(i, action)| Binding {
                input: InputId(i),
                trigger: Trigger::Press,
                action,
            })
            .collect();
        let cfg = Config {
            version: 1,
            timing: Timing::default(),
            sound: Default::default(),
            profiles: vec![Profile {
                id: 1,
                name: "p".into(),
                tag: "AAA".into(),
                bindings,
            }],
        };
        let rec = Arc::new(Rec::default());
        let d = Dispatcher::new(Arc::new(ArcSwap::from_pointee(cfg)), rec.clone());
        (d, rec)
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
        fn run(&self, a: &Action) {
            if !self.1.swap(true, std::sync::atomic::Ordering::SeqCst) {
                panic!("boom");
            }
            self.0.run(a);
        }
    }

    #[test]
    fn worker_survives_panic() {
        let cfg = setup(vec![(0, text("a")), (1, text("b"))]).0.config.clone();
        let ex = Arc::new(PanicFirst(Rec::default(), Default::default()));
        let mut d = Dispatcher::new(cfg, ex.clone());
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
        }];
        let swap = Arc::new(ArcSwap::from_pointee(cfg.clone()));
        let mut d = Dispatcher::new(swap.clone(), Arc::new(Rec::default()));
        cfg.timing.hold_ms = 1000;
        swap.store(Arc::new(cfg));
        let t0 = Instant::now();
        d.on_edge(Edge::Down(encode_action(1, InputId(0))), t0);
        assert_eq!(d.next_deadline(), Some(t0 + Duration::from_millis(1000)));
    }

    #[test]
    fn announce_sets_profile() {
        let (mut d, _) = setup(vec![]);
        d.on_edge(Edge::Down(0x0603), Instant::now());
        assert_eq!(d.current_profile(), Some(3));
    }
}

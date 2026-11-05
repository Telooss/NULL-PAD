//! Modele de config JSON, chargement et validation.
use crate::codes::InputId;
use crate::triggers::{Timing, Trigger, Wants};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::{fmt, fs, io};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    pub version: u32,
    pub timing: Timing,
    pub profiles: Vec<Profile>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Profile {
    pub id: u8,
    pub name: String,
    pub tag: String,
    pub bindings: Vec<Binding>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Binding {
    pub input: InputId,
    pub trigger: Trigger,
    pub action: Action,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum Action {
    Shortcut { keys: String },
    Text { text: String },
    Macro { steps: Vec<Step> },
    Launch { program: String, args: Vec<String> },
    Open { target: String },
    Command { line: String },
    Media { key: MediaKey },
    Volume { delta: i8 },
    VolumeMute,
    MicVolume { delta: i8 },
    MicMute,
    Toggle { on: Box<Action>, off: Box<Action> },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum Step {
    Do { action: Action },
    Wait { ms: u64 },
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKey {
    PlayPause,
    Next,
    Prev,
    Stop,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Json(serde_json::Error),
    Invalid(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Json(e) => write!(f, "json: {e}"),
            Self::Invalid(s) => write!(f, "config invalide: {s}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

fn check_action(a: &Action, nested: bool, in_macro: bool) -> Result<(), String> {
    match a {
        Action::Volume { delta } | Action::MicVolume { delta } if !(-100..=100).contains(delta) => {
            Err(format!("delta {delta} hors -100..=100"))
        }
        Action::Toggle { .. } if nested => Err("Toggle imbrique".into()),
        Action::Toggle { .. } if in_macro => Err("Toggle dans une Macro".into()),
        Action::Toggle { on, off } => {
            check_action(on, true, in_macro)?;
            check_action(off, true, in_macro)
        }
        Action::Macro { steps } => steps.iter().try_for_each(|s| match s {
            Step::Do { action } => check_action(action, nested, true),
            Step::Wait { .. } => Ok(()),
        }),
        _ => Ok(()),
    }
}

fn validate(c: &Config) -> Result<(), String> {
    if c.version != 1 {
        return Err(format!("version {} non supportee", c.version));
    }
    // Borne haute : une duree enorme deborde `Instant + Duration`.
    for (name, ms) in [
        ("hold_ms", c.timing.hold_ms),
        ("double_ms", c.timing.double_ms),
    ] {
        if !(50..=5000).contains(&ms) {
            return Err(format!("{name} {ms} hors 50..=5000"));
        }
    }
    let mut ids = HashSet::new();
    for p in &c.profiles {
        if !(1..=8).contains(&p.id) {
            return Err(format!("profile id {} hors 1..=8", p.id));
        }
        if !ids.insert(p.id) {
            return Err(format!("duplicate profile id {}", p.id));
        }
        if p.tag.chars().count() != 3 || !p.tag.chars().all(|c| matches!(c, 'A'..='Z' | '0'..='9'))
        {
            return Err(format!("tag {:?} invalide (3 caracteres A-Z0-9)", p.tag));
        }
        let mut seen = HashSet::new();
        for b in &p.bindings {
            if !seen.insert((b.input, b.trigger)) {
                return Err(format!(
                    "duplicate binding {} {:?} dans le profil {}",
                    b.input.name(),
                    b.trigger,
                    p.id
                ));
            }
            check_action(&b.action, false, false)?;
        }
    }
    Ok(())
}

pub fn parse(text: &str) -> Result<Config, ConfigError> {
    let c: Config = serde_json::from_str(text)?;
    validate(&c).map_err(ConfigError::Invalid)?;
    Ok(c)
}

pub fn load(path: &Path) -> Result<Config, ConfigError> {
    parse(&fs::read_to_string(path)?)
}

pub fn default_config() -> Config {
    let bind = |input: &str, action| Binding {
        input: InputId::from_name(input).expect("nom d'entree valide"),
        trigger: Trigger::Press,
        action,
    };
    Config {
        version: 1,
        timing: Timing::default(),
        profiles: vec![Profile {
            id: 1,
            name: "Default".into(),
            tag: "DEF".into(),
            bindings: vec![
                bind("ENC_R_CW", Action::MicVolume { delta: 2 }),
                bind("ENC_R_CCW", Action::MicVolume { delta: -2 }),
                bind("ENC_R_PRESS", Action::MicMute),
            ],
        }],
    }
}

pub fn save_default_if_missing(path: &Path) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(&default_config()).map_err(io::Error::from)?;
    fs::write(path, json)
}

impl Config {
    fn bindings(&self, profile: u8, input: InputId) -> impl Iterator<Item = &Binding> {
        self.profiles
            .iter()
            .filter(move |p| p.id == profile)
            .flat_map(|p| &p.bindings)
            .filter(move |b| b.input == input)
    }

    pub fn lookup(&self, profile: u8, input: InputId, trigger: Trigger) -> Option<&Action> {
        self.bindings(profile, input)
            .find(|b| b.trigger == trigger)
            .map(|b| &b.action)
    }

    pub fn wants(&self, profile: u8, input: InputId) -> Wants {
        self.bindings(profile, input)
            .fold(Wants::default(), |w, b| Wants {
                hold: w.hold || b.trigger == Trigger::Hold,
                double: w.double || b.trigger == Trigger::DoubleTap,
            })
    }
}

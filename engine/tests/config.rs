use engine::codes::InputId;
use engine::config::*;
use engine::triggers::{Trigger, Wants};

const GOOD: &str = include_str!("fixtures/two_bindings.json");

fn invalid(text: &str) {
    assert!(
        matches!(parse(text), Err(ConfigError::Invalid(_))),
        "{text}"
    );
}

#[test]
fn default_roundtrips() {
    let c = default_config();
    assert_eq!(parse(&serde_json::to_string(&c).unwrap()).unwrap(), c);
}

#[test]
fn rejects_timing_out_of_bounds() {
    for f in ["hold_ms", "double_ms"] {
        for bad in [49u64, 5001, u64::MAX] {
            let mut v: serde_json::Value = serde_json::from_str(GOOD).unwrap();
            v["timing"][f] = bad.into();
            invalid(&v.to_string());
        }
        for ok in [50u64, 5000] {
            let mut v: serde_json::Value = serde_json::from_str(GOOD).unwrap();
            v["timing"][f] = ok.into();
            assert!(parse(&v.to_string()).is_ok());
        }
    }
}

#[test]
fn rejects_bad_tag() {
    invalid(&GOOD.replace("AB1", "ab"));
    invalid(&GOOD.replace("AB1", "abcd"));
}

#[test]
fn rejects_duplicate_profile_id() {
    let mut c = parse(GOOD).unwrap();
    c.profiles.push(c.profiles[0].clone());
    invalid(&serde_json::to_string(&c).unwrap());
}

#[test]
fn rejects_profile_9() {
    invalid(&GOOD.replace("\"id\":1", "\"id\":9"));
}

#[test]
fn rejects_duplicate_binding() {
    let mut c = parse(GOOD).unwrap();
    c.profiles[0].bindings[1].trigger = Trigger::Press;
    invalid(&serde_json::to_string(&c).unwrap());
}

#[test]
fn rejects_bad_actions() {
    let toggle = |a: &str| format!(r#"{{"type":"Toggle","on":{a},"off":{{"type":"MicMute"}}}}"#);
    let with = |a: &str| GOOD.replace(r#"{"type":"Shortcut","keys":"Ctrl+S"}"#, a);
    invalid(&with(r#"{"type":"Volume","delta":101}"#));
    invalid(&with(r#"{"type":"MicVolume","delta":-101}"#));
    invalid(&with(&toggle(&toggle(r#"{"type":"MicMute"}"#))));
    invalid(&with(&format!(
        r#"{{"type":"Macro","steps":[{{"type":"Do","action":{}}}]}}"#,
        toggle(r#"{"type":"MicMute"}"#)
    )));
    assert!(parse(&with(&toggle(r#"{"type":"VolumeMute"}"#))).is_ok());
}

#[test]
fn rejects_bad_version() {
    invalid(&GOOD.replace("\"version\":1", "\"version\":2"));
}

#[test]
fn rejects_truncated_json() {
    assert!(matches!(
        parse(&GOOD[..GOOD.len() / 2]),
        Err(ConfigError::Json(_))
    ));
}

#[test]
fn lookup_and_wants() {
    let c = parse(GOOD).unwrap();
    let k0 = InputId::from_name("K00").unwrap();
    let k1 = InputId::from_name("K01").unwrap();
    assert_eq!(
        c.wants(1, k0),
        Wants {
            hold: true,
            double: false
        }
    );
    assert_eq!(c.wants(5, k0), Wants::default());
    assert_eq!(
        c.lookup(1, k0, Trigger::Press),
        Some(&Action::Shortcut {
            keys: "Ctrl+S".into()
        })
    );
    assert_eq!(c.lookup(1, k1, Trigger::Press), None);
    assert_eq!(c.lookup(5, k0, Trigger::Press), None);
}

#[test]
fn save_default_if_missing_writes_once() {
    let p = std::env::temp_dir().join(format!("macropad-test-{}/c.json", std::process::id()));
    save_default_if_missing(&p).unwrap();
    assert_eq!(load(&p).unwrap(), default_config());
    std::fs::write(&p, "x").unwrap();
    save_default_if_missing(&p).unwrap();
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "x");
    std::fs::remove_dir_all(p.parent().unwrap()).unwrap();
}

const SOUND: &str = include_str!("fixtures/sound.json");

#[test]
fn old_config_without_sound_still_loads() {
    assert_eq!(parse(GOOD).unwrap().sound, SoundSettings::default());
}

#[test]
fn sound_fixture_parses() {
    let c = parse(SOUND).unwrap();
    assert_eq!(c.sound.mic_mode, MicMode::External);
    assert_eq!(c.sound.sounds[0].volume, 80);
    assert_eq!(c.sound.sounds[0].on_repeat, OnRepeat::Stop);
    assert_eq!(c.sound.sounds[1].volume, 100);
    assert_eq!(c.sound.sounds[1].on_repeat, OnRepeat::Restart);
}

#[test]
fn rejects_bad_sound_settings() {
    let set = |from: &str, to: &str| {
        let t = SOUND.replace(from, to);
        assert_ne!(t, SOUND);
        invalid(&t);
    };
    set(r#""id":"clap""#, r#""id":"airhorn""#); // id en double
    set(r#""id":"clap""#, r#""id":"""#);
    set(r#""volume":80"#, r#""volume":101"#);
    for f in [
        "master_volume",
        "monitor_volume",
        "cable_volume",
        "mic_gain",
    ] {
        set(r#""sound":{"#, &format!(r#""sound":{{"{f}":101,"#));
    }
    let act = |a: &str| SOUND.replace(r#"{"type":"StopSounds"}"#, a);
    invalid(&act(r#"{"type":"SoundVolume","delta":101}"#));
    assert!(parse(&act(r#"{"type":"SoundVolume","delta":-100}"#)).is_ok());
    let ghost = r#"{"type":"Sound","id":"nope"}"#;
    invalid(&act(ghost));
    invalid(&act(&format!(
        r#"{{"type":"Toggle","on":{ghost},"off":{{"type":"StopSounds"}}}}"#
    )));
    invalid(&act(&format!(
        r#"{{"type":"Macro","steps":[{{"type":"Do","action":{ghost}}}]}}"#
    )));
}

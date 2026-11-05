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

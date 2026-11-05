use arc_swap::ArcSwap;
use engine::config::{default_config, load, Config};
use engine::watch::watch_config;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn json(tag: &str) -> String {
    let mut c = default_config();
    c.profiles[0].tag = tag.into();
    serde_json::to_string(&c).unwrap()
}

fn setup(name: &str) -> (PathBuf, Arc<ArcSwap<Config>>, impl Drop) {
    let dir = std::env::temp_dir().join(format!("macropad-watch-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    std::fs::write(&path, json("AAA")).unwrap();
    let target = Arc::new(ArcSwap::from_pointee(load(&path).unwrap()));
    let w = watch_config(path.clone(), target.clone()).unwrap();
    (path, target, w)
}

fn wait_tag(t: &ArcSwap<Config>, tag: &str) -> bool {
    let end = Instant::now() + Duration::from_secs(1);
    while Instant::now() < end {
        if t.load().profiles[0].tag == tag {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn valid_change_is_applied() {
    let (path, t, _w) = setup("valid");
    std::fs::write(&path, json("BBB")).unwrap();
    assert!(wait_tag(&t, "BBB"));
}

#[test]
fn truncated_write_keeps_previous() {
    let (path, t, _w) = setup("trunc");
    let full = json("BBB");
    std::fs::write(&path, &full[..full.len() / 2]).unwrap();
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(t.load().profiles[0].tag, "AAA");
}

#[test]
fn invalid_then_fixed() {
    let (path, t, _w) = setup("fixed");
    std::fs::write(&path, json("ab")).unwrap();
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(t.load().profiles[0].tag, "AAA");
    std::fs::write(&path, json("CCC")).unwrap();
    assert!(wait_tag(&t, "CCC"));
}

//! Bout en bout : le binaire en `--simulate`, avec des actions Command qui creent des fichiers marqueurs.
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn press_hold_doubletap_toggle_macro() {
    let dir = std::env::temp_dir();
    let id = std::process::id();
    let marker = |n: &str| -> PathBuf { dir.join(format!("macropad-e2e-{id}-{n}.txt")) };
    let cmd = |n: &str| json!({"type":"Command","line":format!("New-Item -ItemType File -Force -Path '{}'", marker(n).display())});
    let bind = |input: &str, trigger: &str, action: Value| json!({"input":input,"trigger":trigger,"action":action});
    let bindings = vec![
        bind("K00", "Press", cmd("press")),
        bind("K01", "Press", cmd("hold-press")),
        bind("K01", "Hold", cmd("hold")),
        bind("K02", "Press", cmd("dbl-press")),
        bind("K02", "DoubleTap", cmd("dbl")),
        bind(
            "K03",
            "Press",
            json!({"type":"Toggle","on":cmd("on"),"off":cmd("off")}),
        ),
        bind(
            "K04",
            "Press",
            json!({"type":"Macro","steps":[
            {"type":"Wait","ms":300},{"type":"Do","action":cmd("macro")}]}),
        ),
    ];
    let expected = ["press", "hold", "dbl", "on", "off", "macro"];
    let unexpected = ["hold-press", "dbl-press"];
    let cfg = dir.join(format!("macropad-e2e-{id}.json"));
    let json = json!({"version":1,"timing":{"hold_ms":300,"double_ms":250},
        "profiles":[{"id":1,"name":"P","tag":"E2E","bindings":bindings}]});
    std::fs::write(&cfg, json.to_string()).unwrap();
    for n in expected.iter().chain(&unexpected) {
        let _ = std::fs::remove_file(marker(n));
    }

    let mut child = Command::new(env!("CARGO_BIN_EXE_nullctl-engine"))
        .args(["--simulate", "--config"])
        .arg(&cfg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut send = |s: &str, pause: u64| {
        stdin.write_all(s.as_bytes()).unwrap();
        stdin.flush().unwrap();
        sleep(ms(pause));
    };
    send("down 1 K00\nup 1 K00\n", 50);
    send("down 1 K01\n", 450); // maintenu au-dela de hold_ms
    send("up 1 K01\n", 50);
    send("down 1 K02\nup 1 K02\n", 80);
    send("down 1 K02\nup 1 K02\n", 400);
    send("down 1 K03\nup 1 K03\n", 100);
    send("down 1 K03\nup 1 K03\n", 100);
    send("down 1 K04\nup 1 K04\n", 1500); // laisse la macro finir avant l'EOF
    drop(stdin);

    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(15) {
            child.kill().unwrap();
            panic!("engine did not exit within 15 s");
        }
        sleep(ms(50));
    }
    // PowerShell peut demarrer apres la sortie du moteur.
    while expected.iter().any(|n| !marker(n).exists()) && start.elapsed() < Duration::from_secs(20)
    {
        sleep(ms(100));
    }
    sleep(ms(500));
    let missing: Vec<_> = expected.iter().filter(|n| !marker(n).exists()).collect();
    let extra: Vec<_> = unexpected.iter().filter(|n| marker(n).exists()).collect();
    for n in expected.iter().chain(&unexpected) {
        let _ = std::fs::remove_file(marker(n));
    }
    let _ = std::fs::remove_file(&cfg);
    assert!(missing.is_empty(), "markers manquants: {missing:?}");
    assert!(extra.is_empty(), "markers inattendus: {extra:?}");
}

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn simulate_runs_command_action() {
    let dir = std::env::temp_dir();
    let id = std::process::id();
    let marker = dir.join(format!("macropad-sim-{id}.txt"));
    let cfg = dir.join(format!("macropad-sim-{id}.json"));
    let _ = std::fs::remove_file(&marker);
    let line = format!(
        "New-Item -ItemType File -Force -Path '{}'",
        marker.display()
    );
    let json = serde_json::json!({"version":1,"timing":{"hold_ms":300,"double_ms":250},
        "profiles":[{"id":1,"name":"P","tag":"SIM","bindings":[
            {"input":"K00","trigger":"Press","action":{"type":"Command","line":line}}]}]});
    std::fs::write(&cfg, json.to_string()).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_engine"))
        .args(["--simulate", "--config"])
        .arg(&cfg)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"down 1 K00\nup 1 K00\ndown 9 K00\n")
        .unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if start.elapsed() > Duration::from_secs(10) {
            child.kill().unwrap();
            panic!("engine did not exit within 10 s");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(status.success());
    // PowerShell start-up may outlast the engine's 500 ms grace period.
    while !marker.exists() && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(100));
    }
    let ok = marker.exists();
    let _ = std::fs::remove_file(&marker);
    let _ = std::fs::remove_file(&cfg);
    assert!(ok, "marker file not created");
}

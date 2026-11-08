//! Pipe d'evenements : un nom de pipe unique par test.
use engine::ipc::{serve_events, Event, Events};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(5);

fn pipe_name() -> String {
    static N: AtomicUsize = AtomicUsize::new(0);
    format!(
        r"\\.\pipe\nullctl-test-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    )
}

/// Ouvre le pipe (reessaie si l'instance suivante n'est pas encore creee) puis attend `n` clients.
fn connect(name: &str, ev: &Events, n: usize) -> BufReader<File> {
    let end = Instant::now() + WAIT;
    let f = loop {
        match File::open(name) {
            Ok(f) => break f,
            Err(e) if Instant::now() > end => panic!("open {name}: {e}"),
            Err(_) => thread::sleep(Duration::from_millis(10)),
        }
    };
    while ev.clients() < n {
        assert!(Instant::now() < end, "client jamais enregistre");
        thread::sleep(Duration::from_millis(5));
    }
    BufReader::new(f)
}

/// Lit une ligne dans un thread, pour ne jamais bloquer le test indefiniment.
fn read_line(r: BufReader<File>) -> String {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut r = r;
        let mut line = String::new();
        r.read_line(&mut line).unwrap();
        tx.send(line).unwrap();
    });
    rx.recv_timeout(WAIT).expect("aucune ligne recue")
}

#[test]
fn client_receives_events_as_json_lines() {
    let name = pipe_name();
    let ev = serve_events(&name).unwrap();
    let r = connect(&name, &ev, 1);
    ev.emit(&Event::Profile { profile: 2 });
    assert_eq!(read_line(r), "{\"type\":\"profile\",\"profile\":2}\n");
}

#[test]
fn two_clients_both_receive() {
    let name = pipe_name();
    let ev = serve_events(&name).unwrap();
    let a = connect(&name, &ev, 1);
    let b = connect(&name, &ev, 2);
    ev.emit(&Event::Profile { profile: 5 });
    let want = "{\"type\":\"profile\",\"profile\":5}\n";
    assert_eq!(read_line(a), want);
    assert_eq!(read_line(b), want);
}

#[test]
fn slow_client_never_blocks() {
    let name = pipe_name();
    let ev = serve_events(&name).unwrap();
    let _never_read = connect(&name, &ev, 1);
    // Dans un thread : une regression (emit bloque) echoue au lieu de bloquer le test.
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let t0 = Instant::now();
        for i in 0..10_000u32 {
            ev.emit(&Event::Key {
                input: "K00".into(),
                down: i % 2 == 0,
                profile: 1,
            });
        }
        tx.send(t0.elapsed()).unwrap();
    });
    let took = rx
        .recv_timeout(WAIT)
        .expect("emit bloque par un client lent");
    assert!(took < Duration::from_millis(200), "{took:?}");
}

#[test]
fn emit_without_client_is_noop() {
    let name = pipe_name();
    let ev = serve_events(&name).unwrap();
    ev.emit(&Event::Profile { profile: 1 });
    Events::none().emit(&Event::Profile { profile: 1 });
    assert_eq!(ev.clients(), 0);
    // Un client arrive ensuite : il ne recoit que ce qui est emis apres sa connexion.
    let r = connect(&name, &ev, 1);
    ev.emit(&Event::Profile { profile: 3 });
    assert_eq!(read_line(r), "{\"type\":\"profile\",\"profile\":3}\n");
}

#[test]
fn closed_client_is_removed() {
    let name = pipe_name();
    let ev = serve_events(&name).unwrap();
    drop(connect(&name, &ev, 1));
    let end = Instant::now() + WAIT;
    while ev.clients() > 0 {
        assert!(Instant::now() < end, "client ferme jamais retire");
        ev.emit(&Event::Profile { profile: 1 });
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn second_server_on_same_name_fails() {
    let name = pipe_name();
    let _ev = serve_events(&name).unwrap();
    assert!(serve_events(&name).is_err());
}

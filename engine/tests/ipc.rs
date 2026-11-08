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

mod control {
    use super::*;
    use engine::ipc::{serve_control, Request, Response};
    use serde_json::json;
    use std::io::Write;

    fn dummy(r: Request) -> Response {
        match r {
            Request::Status => Response {
                ok: true,
                error: None,
                data: Some(json!({"profile": 2, "link": "usb", "version": "x"})),
            },
            _ => Response {
                ok: false,
                error: Some("non".into()),
                data: None,
            },
        }
    }

    fn client(name: &str) -> File {
        let end = Instant::now() + WAIT;
        loop {
            match File::options().read(true).write(true).open(name) {
                Ok(f) => return f,
                Err(e) if Instant::now() > end => panic!("open {name}: {e}"),
                Err(_) => thread::sleep(Duration::from_millis(10)),
            }
        }
    }

    /// Une requete, une ligne de reponse (lue dans un thread pour ne jamais bloquer le test).
    fn ask(f: &mut File, line: &str) -> String {
        f.write_all(line.as_bytes()).unwrap();
        let r = BufReader::new(f.try_clone().unwrap());
        read_line(r)
    }

    #[test]
    fn status_roundtrip() {
        let name = pipe_name();
        serve_control(&name, dummy).unwrap();
        let mut c = client(&name);
        let got = ask(&mut c, "{\"cmd\":\"status\"}\n");
        assert_eq!(
            got,
            "{\"ok\":true,\"data\":{\"link\":\"usb\",\"profile\":2,\"version\":\"x\"}}\n"
        );
        // La connexion reste ouverte : une deuxieme requete passe.
        assert!(ask(&mut c, "{\"cmd\":\"test_obs\"}\n").contains("\"ok\":false"));
    }

    #[test]
    fn bad_request_gets_error() {
        let name = pipe_name();
        serve_control(&name, dummy).unwrap();
        let mut c = client(&name);
        let want = "{\"ok\":false,\"error\":\"requete invalide\"}\n";
        assert_eq!(ask(&mut c, "pas du json\n"), want);
        assert_eq!(ask(&mut c, "{\"cmd\":\"inconnu\"}\n"), want);
        // Toujours en vie apres une requete illisible.
        assert!(ask(&mut c, "{\"cmd\":\"status\"}\n").starts_with("{\"ok\":true"));
    }

    #[test]
    fn oversized_line_errors_and_disconnects() {
        let name = pipe_name();
        serve_control(&name, dummy).unwrap();
        let mut c = client(&name);
        let long = format!("{}\n", "a".repeat(5000));
        assert_eq!(
            ask(&mut c, &long),
            "{\"ok\":false,\"error\":\"requete invalide\"}\n"
        );
        let mut rest = String::new();
        let r = BufReader::new(c);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut r = r;
            tx.send(r.read_line(&mut rest).unwrap_or(0)).unwrap();
        });
        assert_eq!(rx.recv_timeout(WAIT).expect("pas deconnecte"), 0);
    }

    #[test]
    fn two_clients_in_parallel() {
        let name = pipe_name();
        serve_control(&name, dummy).unwrap();
        let mut a = client(&name);
        let mut b = client(&name); // `a` est connecte et muet : `b` n'attend pas
        assert!(ask(&mut b, "{\"cmd\":\"status\"}\n").starts_with("{\"ok\":true"));
        assert!(ask(&mut a, "{\"cmd\":\"status\"}\n").starts_with("{\"ok\":true"));
    }

    #[test]
    fn second_server_on_same_name_fails() {
        let name = pipe_name();
        serve_control(&name, dummy).unwrap();
        assert!(serve_control(&name, dummy).is_err());
    }
}

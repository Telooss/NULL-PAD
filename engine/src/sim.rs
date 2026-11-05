//! Mode simulation : lit des lignes sur stdin au lieu du NULL/PAD.
use crate::codes::{encode_action, encode_profile, InputId};
use crate::dispatch::Dispatcher;
use crate::report::Edge;
use std::io::BufRead;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

/// `down <profil> <INPUT>` | `up <profil> <INPUT>` | `announce <profil>`.
pub fn parse_line(line: &str) -> Result<Edge, String> {
    let w: Vec<&str> = line.split_whitespace().collect();
    let profile = |s: &str| {
        s.parse::<u8>()
            .ok()
            .filter(|p| (1..=8).contains(p))
            .ok_or_else(|| format!("profil invalide (1 a 8): {s}"))
    };
    let input = |s: &str| InputId::from_name(s).ok_or_else(|| format!("entree inconnue: {s}"));
    match w[..] {
        ["announce", p] => Ok(Edge::Down(encode_profile(profile(p)?))),
        ["down", p, i] => Ok(Edge::Down(encode_action(profile(p)?, input(i)?))),
        ["up", p, i] => Ok(Edge::Up(encode_action(profile(p)?, input(i)?))),
        _ => Err(format!("ligne incomprise: {line:?}")),
    }
}

pub fn run_simulation(mut dispatcher: Dispatcher) {
    let (tx, rx) = channel::<String>();
    thread::spawn(move || {
        for l in std::io::stdin().lock().lines().map_while(Result::ok) {
            if tx.send(l).is_err() {
                break;
            }
        }
    });
    let mut eof = false;
    loop {
        let now = Instant::now();
        let wait = dispatcher
            .next_deadline()
            .map(|d| d.saturating_duration_since(now));
        // A l'EOF : on attend les echeances en cours, puis 500 ms pour le travail lance.
        if eof {
            match wait {
                None => {
                    thread::sleep(Duration::from_millis(500));
                    return;
                }
                Some(w) => {
                    thread::sleep(w);
                    dispatcher.on_tick(Instant::now());
                    continue;
                }
            }
        }
        let res = match wait {
            Some(w) => rx.recv_timeout(w),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match res {
            Ok(line) if line.trim().is_empty() => {}
            Ok(line) => match parse_line(&line) {
                Ok(e) => dispatcher.on_edge(e, Instant::now()),
                Err(m) => tracing::error!("simulation: {m}"),
            },
            Err(RecvTimeoutError::Timeout) => dispatcher.on_tick(Instant::now()),
            Err(RecvTimeoutError::Disconnected) => eof = true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lines() {
        let k00 = InputId::from_name("K00").unwrap();
        assert_eq!(
            parse_line("down 1 K00"),
            Ok(Edge::Down(encode_action(1, k00)))
        );
        assert_eq!(parse_line("up 2 K00"), Ok(Edge::Up(encode_action(2, k00))));
        assert_eq!(parse_line("announce 3"), Ok(Edge::Down(encode_profile(3))));
        for bad in [
            "down 9 K00",
            "down 0 K00",
            "down 1 ZZ",
            "jump",
            "down 1",
            "",
        ] {
            assert!(parse_line(bad).is_err(), "{bad}");
        }
    }
}

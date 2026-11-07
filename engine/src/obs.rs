//! Client obs-websocket v5 minimal (synchrone, un thread dedie).

use crate::config::{Action, ObsOp, ObsSettings};
use arc_swap::ArcSwap;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::sync::Arc;
use std::time::Duration;
use tungstenite::{Message, WebSocket};

const TIMEOUT: Duration = Duration::from_secs(2);

type Ws = WebSocket<TcpStream>;

#[derive(Clone)]
pub struct ObsHandle {
    tx: SyncSender<Action>,
    settings: Arc<ArcSwap<ObsSettings>>,
}

/// `base64(sha256(base64(sha256(password + salt)) + challenge))` (protocol.md).
pub fn auth_string(password: &str, salt: &str, challenge: &str) -> String {
    let secret = STANDARD.encode(Sha256::digest(format!("{password}{salt}")));
    STANDARD.encode(Sha256::digest(format!("{secret}{challenge}")))
}

/// Message Request (op 6) pour une action OBS, `None` sinon.
pub fn request_for(action: &Action, id: u64) -> Option<Value> {
    let with_op = |op: &ObsOp, what: &str| {
        let verb = match op {
            ObsOp::Start => "Start",
            ObsOp::Stop => "Stop",
            ObsOp::Toggle => "Toggle",
        };
        format!("{verb}{what}")
    };
    let (ty, data) = match action {
        Action::ObsScene { scene } => {
            ("SetCurrentProgramScene".into(), json!({"sceneName": scene}))
        }
        Action::ObsMute { input } => ("ToggleInputMute".into(), json!({"inputName": input})),
        Action::ObsStream { op } => (with_op(op, "Stream"), json!({})),
        Action::ObsRecord { op } => (with_op(op, "Record"), json!({})),
        Action::ObsReplaySave => ("SaveReplayBuffer".into(), json!({})),
        _ => return None,
    };
    Some(
        json!({"op": 6, "d": {"requestType": ty, "requestId": id.to_string(), "requestData": data}}),
    )
}

impl ObsHandle {
    /// Ne bloque jamais : file pleine ou thread mort -> l'action est jetee.
    pub fn send(&self, action: Action) {
        match self.tx.try_send(action) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => tracing::warn!("OBS: file pleine, action ignoree"),
            Err(TrySendError::Disconnected(_)) => {
                tracing::warn!("OBS: thread arrete, action ignoree")
            }
        }
    }

    /// Ne bloque jamais : le thread OBS relit les reglages avant chaque action.
    pub fn apply(&self, s: &ObsSettings) {
        self.settings.store(Arc::new(s.clone()));
    }
}

pub fn start_obs() -> ObsHandle {
    let (tx, rx) = sync_channel::<Action>(32);
    let shared = Arc::new(ArcSwap::from_pointee(ObsSettings::default()));
    let seen = shared.clone();
    std::thread::Builder::new()
        .name("obs".into())
        .spawn(move || {
            let mut settings = seen.load_full();
            let mut conn: Option<Ws> = None;
            let mut next_id = 0u64;
            for action in rx {
                let latest = seen.load_full();
                if *latest != *settings {
                    conn = None;
                    settings = latest;
                }
                next_id += 1;
                let Some(req) = request_for(&action, next_id) else {
                    continue;
                };
                if conn.as_mut().is_some_and(|ws| !alive(ws)) {
                    conn = None;
                }
                if conn.is_none() {
                    match connect(&settings) {
                        Ok(ws) => conn = Some(ws),
                        Err(e) => {
                            tracing::warn!(
                                "OBS {}:{} injoignable: {e}",
                                settings.host,
                                settings.port
                            );
                            continue;
                        }
                    }
                }
                let ws = conn.as_mut().expect("connecte");
                if let Err(e) = request(ws, &req, &next_id.to_string()) {
                    tracing::warn!("OBS: {} echouee: {e}", req["d"]["requestType"]);
                    conn = None;
                }
            }
        })
        .expect("thread OBS");
    ObsHandle {
        tx,
        settings: shared,
    }
}

/// Vide sans attendre ce qui est arrive pendant l'inactivite ; faux si OBS a ferme.
/// Evite de perdre une action sur une connexion morte sans risquer de la rejouer.
fn alive(ws: &mut Ws) -> bool {
    if ws.get_ref().set_nonblocking(true).is_err() {
        return false;
    }
    let ok = loop {
        match ws.read() {
            Ok(Message::Close(_)) => break false,
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => break true,
            Err(_) => break false,
        }
    };
    ok && ws.get_ref().set_nonblocking(false).is_ok()
}

/// Prochain message texte JSON (les autres trames sont ignorees).
fn read_json(ws: &mut Ws) -> Result<Value, String> {
    loop {
        match ws.read().map_err(|e| e.to_string())? {
            Message::Text(t) => {
                if let Ok(v) = serde_json::from_str(&t) {
                    return Ok(v);
                }
            }
            Message::Close(f) => return Err(format!("connexion fermee par OBS ({f:?})")),
            _ => {}
        }
    }
}

fn connect(s: &ObsSettings) -> Result<Ws, String> {
    let mut last = String::from("aucune adresse");
    let mut stream = None;
    for addr in (s.host.as_str(), s.port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
    {
        match TcpStream::connect_timeout(&addr, TIMEOUT) {
            Ok(st) => {
                stream = Some(st);
                break;
            }
            Err(e) => last = e.to_string(),
        }
    }
    let stream = stream.ok_or(last)?;
    stream
        .set_read_timeout(Some(TIMEOUT))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(TIMEOUT))
        .map_err(|e| e.to_string())?;
    let host = if s.host.contains(':') {
        format!("[{}]", s.host)
    } else {
        s.host.clone()
    };
    let (mut ws, _) = tungstenite::client(format!("ws://{host}:{}", s.port), stream)
        .map_err(|e| format!("handshake WebSocket: {e}"))?;

    let hello = read_json(&mut ws)?;
    if hello["op"] != 0 {
        return Err(format!("Hello attendu, recu op {}", hello["op"]));
    }
    let mut identify = json!({"rpcVersion": 1, "eventSubscriptions": 0});
    if let Some(a) = hello["d"].get("authentication") {
        let Some(pw) = &s.password else {
            return Err("OBS demande un mot de passe, aucun n'est configure".into());
        };
        let salt = a["salt"].as_str().unwrap_or("");
        let challenge = a["challenge"].as_str().unwrap_or("");
        identify["authentication"] = json!(auth_string(pw, salt, challenge));
    }
    ws.send(Message::text(json!({"op": 1, "d": identify}).to_string()))
        .map_err(|e| e.to_string())?;
    let identified = read_json(&mut ws).map_err(|e| format!("{e} (mot de passe incorrect ?)"))?;
    if identified["op"] != 2 {
        return Err(format!("Identified attendu, recu op {}", identified["op"]));
    }
    Ok(ws)
}

/// Envoie la requete et attend sa RequestResponse (op 7) ; `result == false` -> warn.
fn request(ws: &mut Ws, req: &Value, id: &str) -> Result<(), String> {
    ws.send(Message::text(req.to_string()))
        .map_err(|e| e.to_string())?;
    loop {
        let v = read_json(ws)?;
        if v["op"] == 7 && v["d"]["requestId"] == id {
            let st = &v["d"]["requestStatus"];
            if st["result"] != true {
                tracing::warn!(
                    "OBS: {} refusee (code {}: {})",
                    req["d"]["requestType"],
                    st["code"],
                    st["comment"].as_str().unwrap_or("")
                );
            }
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    const SALT: &str = "lM1GncleQOaCu9lT1yeUZhFYnqhsLLP1G5lAGo3ixaI=";
    const CHALLENGE: &str = "+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeYcTNS3PDaeY=";
    const WAIT: Duration = Duration::from_secs(10);

    #[test]
    fn auth_matches_reference() {
        let pw = "supersecretpassword";
        let secret = STANDARD.encode(Sha256::digest(format!("{pw}{SALT}")));
        let expected = STANDARD.encode(Sha256::digest(format!("{secret}{CHALLENGE}")));
        assert_eq!(auth_string(pw, SALT, CHALLENGE), expected);
        // Valeur calculee independamment (Python hashlib/base64) sur l'exemple de protocol.md.
        assert_eq!(expected, "1Ct943GAT+6YQUUX47Ia/ncufilbe6+oD6lY+5kaCu4=");
    }

    #[test]
    fn request_for_maps_all_obs_actions() {
        let cases = [
            (
                Action::ObsScene {
                    scene: "Jeu".into(),
                },
                "SetCurrentProgramScene",
                json!({"sceneName": "Jeu"}),
            ),
            (
                Action::ObsMute {
                    input: "Micro".into(),
                },
                "ToggleInputMute",
                json!({"inputName": "Micro"}),
            ),
            (
                Action::ObsStream { op: ObsOp::Start },
                "StartStream",
                json!({}),
            ),
            (
                Action::ObsStream { op: ObsOp::Stop },
                "StopStream",
                json!({}),
            ),
            (
                Action::ObsStream { op: ObsOp::Toggle },
                "ToggleStream",
                json!({}),
            ),
            (
                Action::ObsRecord { op: ObsOp::Start },
                "StartRecord",
                json!({}),
            ),
            (
                Action::ObsRecord { op: ObsOp::Stop },
                "StopRecord",
                json!({}),
            ),
            (
                Action::ObsRecord { op: ObsOp::Toggle },
                "ToggleRecord",
                json!({}),
            ),
            (Action::ObsReplaySave, "SaveReplayBuffer", json!({})),
        ];
        for (action, ty, data) in cases {
            let got = request_for(&action, 7).expect("action OBS");
            assert_eq!(
                got,
                json!({"op": 6, "d": {"requestType": ty, "requestId": "7", "requestData": data}}),
                "{action:?}"
            );
        }
        assert_eq!(request_for(&Action::VolumeMute, 1), None);
    }

    fn read_json(ws: &mut Ws) -> Option<Value> {
        super::read_json(ws).ok()
    }

    /// Faux OBS : accepte `conns` connexions l'une apres l'autre. Chaque requete op 6
    /// recue est transmise sur `tx` ; une authentification refusee envoie `{"rejected":true}`.
    /// Apres `per_conn` requetes, la connexion est fermee.
    fn fake_obs(
        listener: TcpListener,
        password: Option<&'static str>,
        conns: usize,
        per_conn: usize,
        tx: mpsc::Sender<Value>,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            for _ in 0..conns {
                let (stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(WAIT)).unwrap();
                let mut ws = tungstenite::accept(stream).unwrap();
                let mut hello =
                    json!({"op": 0, "d": {"obsWebSocketVersion": "5.5.0", "rpcVersion": 1}});
                if password.is_some() {
                    hello["d"]["authentication"] = json!({"challenge": CHALLENGE, "salt": SALT});
                }
                ws.send(Message::text(hello.to_string())).unwrap();
                let ident = read_json(&mut ws).unwrap();
                assert_eq!(ident["op"], 1);
                assert_eq!(ident["d"]["rpcVersion"], 1);
                if let Some(pw) = password {
                    if ident["d"]["authentication"] != auth_string(pw, SALT, CHALLENGE) {
                        tx.send(json!({"rejected": true})).unwrap();
                        let _ = ws.close(None);
                        let _ = ws.flush();
                        continue;
                    }
                }
                ws.send(Message::text(
                    json!({"op": 2, "d": {"negotiatedRpcVersion": 1}}).to_string(),
                ))
                .unwrap();
                for _ in 0..per_conn {
                    let Some(req) = read_json(&mut ws) else { break };
                    assert_eq!(req["op"], 6);
                    let resp = json!({"op": 7, "d": {
                        "requestType": req["d"]["requestType"],
                        "requestId": req["d"]["requestId"],
                        "requestStatus": {"result": true, "code": 100}}});
                    ws.send(Message::text(resp.to_string())).unwrap();
                    tx.send(req).unwrap();
                }
                let _ = ws.close(None);
                let _ = ws.flush();
            }
        })
    }

    fn settings(port: u16, password: Option<&str>) -> ObsSettings {
        ObsSettings {
            host: "127.0.0.1".into(),
            port,
            password: password.map(String::from),
        }
    }

    fn bind() -> (TcpListener, u16) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        (l, port)
    }

    fn scene(name: &str) -> Action {
        Action::ObsScene { scene: name.into() }
    }

    #[test]
    fn sends_scene_request() {
        let (l, port) = bind();
        let (tx, rx) = mpsc::channel();
        let _srv = fake_obs(l, None, 1, 1, tx);
        let obs = start_obs();
        obs.apply(&settings(port, None));
        obs.send(scene("Jeu"));
        let req = rx.recv_timeout(WAIT).expect("requete recue");
        assert_eq!(req["d"]["requestType"], "SetCurrentProgramScene");
        assert_eq!(req["d"]["requestData"]["sceneName"], "Jeu");
    }

    #[test]
    fn authenticates_with_password() {
        let (l, port) = bind();
        let (tx, rx) = mpsc::channel();
        let _srv = fake_obs(l, Some("supersecretpassword"), 1, 1, tx);
        let obs = start_obs();
        obs.apply(&settings(port, Some("supersecretpassword")));
        obs.send(scene("Jeu"));
        let req = rx.recv_timeout(WAIT).expect("requete authentifiee");
        assert_eq!(req["d"]["requestType"], "SetCurrentProgramScene");
    }

    #[test]
    fn wrong_password_warns_and_survives() {
        let (l, port) = bind();
        let (tx, rx) = mpsc::channel();
        let _srv = fake_obs(l, Some("bon"), 1, 1, tx);
        let obs = start_obs();
        obs.apply(&settings(port, Some("mauvais")));
        obs.send(scene("Jeu"));
        assert_eq!(rx.recv_timeout(WAIT).unwrap(), json!({"rejected": true}));

        let (l2, port2) = bind();
        let (tx2, rx2) = mpsc::channel();
        let _srv2 = fake_obs(l2, Some("bon"), 1, 1, tx2);
        obs.apply(&settings(port2, Some("bon")));
        obs.send(scene("Apres"));
        let req = rx2.recv_timeout(WAIT).expect("le thread OBS a survecu");
        assert_eq!(req["d"]["requestData"]["sceneName"], "Apres");
        assert!(
            rx.try_recv().is_err(),
            "aucune requete sur le serveur refuse"
        );
    }

    #[test]
    fn reconnects_after_server_restart() {
        let (l, port) = bind();
        let (tx, rx) = mpsc::channel();
        let srv = fake_obs(l, None, 1, 1, tx.clone());
        let obs = start_obs();
        obs.apply(&settings(port, None));
        obs.send(scene("Un"));
        assert_eq!(
            rx.recv_timeout(WAIT).unwrap()["d"]["requestData"]["sceneName"],
            "Un"
        );
        srv.join().unwrap(); // premier serveur ferme, listener libere

        let l2 = TcpListener::bind(("127.0.0.1", port)).expect("meme port");
        let _srv2 = fake_obs(l2, None, 1, 1, tx);
        obs.send(scene("Deux"));
        let req = rx.recv_timeout(WAIT).expect("envoi apres redemarrage");
        assert_eq!(req["d"]["requestData"]["sceneName"], "Deux");
    }

    #[test]
    fn send_never_blocks() {
        let (l, port) = bind();
        drop(l);
        let obs = start_obs();
        obs.apply(&settings(port, None));
        let t = Instant::now();
        for _ in 0..100 {
            obs.send(scene("Jeu"));
        }
        assert!(t.elapsed() < Duration::from_millis(50), "{:?}", t.elapsed());
    }

    #[test]
    fn apply_never_blocks_when_queue_is_full() {
        let (l, port) = bind();
        // Accepte puis se tait : le thread OBS reste bloque dans le handshake (~2 s).
        let _mute = thread::spawn(move || {
            let held = l.accept();
            thread::sleep(Duration::from_secs(4));
            drop(held);
        });
        let obs = start_obs();
        obs.apply(&settings(port, None));
        obs.send(scene("Jeu"));
        thread::sleep(Duration::from_millis(200)); // le thread prend cette action et bloque
        while obs.tx.try_send(scene("Jeu")).is_ok() {}
        let t = Instant::now();
        obs.apply(&settings(port, Some("autre")));
        assert!(t.elapsed() < Duration::from_millis(50), "{:?}", t.elapsed());
    }
}

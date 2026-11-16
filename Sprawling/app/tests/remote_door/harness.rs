// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use kernel::{Address, AxCode, EventKind, IdemKey, RunId, Seq};
use remote_access::door::DeviceId;
use remote_access::handshake::{self, Invitation, PairReply, Reply, Session};
use remote_access::keys::{SigningKey, VerifyingKey};
use remote_access::seal::Payload;
use tokio::net::TcpStream;
// boundary-ok: the device's half of the remote door exists only in remote_access, so this check speaks it from Rust (`Sprawling/app/Spec.lean` §8-151)
use tokio_tungstenite::tungstenite::Message;
// boundary-ok: the same device, on the remote listener this city opened
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

pub(crate) type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(crate) const PATIENCE: Duration = Duration::from_secs(60);
pub(crate) const PROVED: &str = "the history is proved";
pub(crate) const STOPPED: &str = "the ledger stopped taking writes";
pub(crate) const PHONE: &str = "phone";

pub(crate) async fn page_at(home: &str) -> Socket {
    let mut page = connect(home, "/ws").await;
    let port = home.rsplit_once(':').unwrap().1.parse::<u16>().unwrap();
    let hello = wire::ClientFrame::Hello(wire::Hello {
        wire_v: wire::WIRE_V,
        schema: wire::schema_hash(),
        token: sprawling::serving::key_file::read_key(port).unwrap(),
    });
    page.send(Message::Text(serde_json::to_string(&hello).unwrap().into()))
        .await
        .unwrap();
    page
}

pub(crate) fn open(n: &[u8]) -> String {
    door_frame(wire::WireCommand::OpenRemoteDoor(wire::DoorOpening {
        lasting_ms: 60 * 60 * 1000,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, n),
    }))
}

pub(crate) fn confirm(code: &str, n: &[u8]) -> String {
    door_frame(wire::WireCommand::ConfirmRemoteDoor(wire::DoorAnswer {
        code: code.to_owned(),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, n),
    }))
}

pub(crate) fn door_frame(command: wire::WireCommand) -> String {
    serde_json::to_string(&wire::ClientFrame::Command(Box::new(command))).unwrap()
}

pub(crate) async fn send_text(page: &mut Socket, text: &str) {
    page.send(Message::Text(text.to_owned().into()))
        .await
        .unwrap();
}

pub(crate) async fn refused(page: &mut Socket, text: &str) -> Option<AxCode> {
    send_text(page, text).await;
    let deadline = tokio::time::Instant::now().checked_add(PATIENCE)?;
    loop {
        let message = tokio::time::timeout_at(deadline, page.next())
            .await
            .ok()??;
        let Ok(Message::Text(text)) = message else {
            continue;
        };
        if let Ok(wire::ServerFrame::Refusal(refusal)) = serde_json::from_str(&text) {
            return Some(*refusal.code());
        }
    }
}

pub(crate) fn fetched(at: &str, path: &str) -> (String, Vec<String>, Vec<u8>) {
    let mut stream = match std::net::TcpStream::connect(at) {
        Ok(stream) => stream,
        Err(refused) => return (format!("{at}: {refused}"), Vec::new(), Vec::new()),
    };
    stream.set_read_timeout(Some(PATIENCE)).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {at}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut answer = Vec::new();

    drop(stream.read_to_end(&mut answer));
    let Some(split) = answer.windows(4).position(|each| each == b"\r\n\r\n") else {
        return (
            String::from_utf8_lossy(&answer).into_owned(),
            Vec::new(),
            Vec::new(),
        );
    };
    let head = String::from_utf8_lossy(&answer[..split]).into_owned();
    let mut lines = head.split("\r\n");
    let status = lines.next().unwrap_or_default().to_owned();
    let mut headers: Vec<String> = lines
        .map(str::to_ascii_lowercase)
        .filter(|line| !line.starts_with("date:"))
        .collect();
    headers.sort();
    (status, headers, answer[split.saturating_add(4)..].to_vec())
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Visit {
    pub(crate) welcomed: bool,

    pub(crate) read_answered: bool,

    pub(crate) act_carried_out: bool,

    pub(crate) local_only_refused: Option<AxCode>,
}

pub(crate) async fn visit(at: &str, invitation: &Invitation) -> Visit {
    let key = SigningKey::from_seed(&[9; 32]).unwrap();
    let (id, city) = pair(at, invitation, &key).await;
    let (mut socket, mut session) = admitted(at, id, &city, &key).await;
    let greeting = wire::ClientFrame::Hello(wire::Hello {
        wire_v: wire::WIRE_V,
        schema: wire::schema_hash(),
        token: None,
    });
    let asking = wire::ClientFrame::Ask(wire::Ask {
        ask_id: wire::AskId(1),
        query: wire::Query::Metrics,
    });
    let halting = wire::ClientFrame::Command(Box::new(wire::WireCommand::Halt {
        scope: wire::HaltScope::City,
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"remote halt"),
    }));
    let revealing = wire::ClientFrame::Command(Box::new(wire::WireCommand::Reveal {
        at: Address::parse("lab").unwrap(),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"remote reveal"),
    }));
    let welcomed = exchange(&mut socket, &mut session, &greeting, |frame| {
        matches!(frame, wire::ServerFrame::Welcome(_))
    })
    .await;
    let read_answered = exchange(&mut socket, &mut session, &asking, |frame| {
        matches!(frame, wire::ServerFrame::Answered(_))
    })
    .await;
    let act_carried_out = exchange(&mut socket, &mut session, &halting, |frame| {
        matches!(frame, wire::ServerFrame::Event(record) if record.kind() == EventKind::CityHalted)
    })
    .await;
    let local_only_refused = exchange(&mut socket, &mut session, &revealing, |frame| {
        matches!(frame, wire::ServerFrame::Refusal(_))
    })
    .await;
    Visit {
        welcomed: welcomed.is_some(),
        read_answered: read_answered.is_some(),
        act_carried_out: act_carried_out.is_some(),
        local_only_refused: match local_only_refused {
            Some(wire::ServerFrame::Refusal(refused)) => Some(*refused.code()),
            Some(_) | None => None,
        },
    }
}

pub(crate) async fn exchange(
    socket: &mut Socket,
    session: &mut Session,
    frame: &wire::ClientFrame,
    wanted: fn(&wire::ServerFrame) -> bool,
) -> Option<wire::ServerFrame> {
    let text = serde_json::to_string(frame).unwrap();
    let sealed = session
        .sealer
        .seal(&Payload::Frame(text).to_bytes())
        .unwrap();

    socket.send(Message::Binary(sealed.into())).await.ok()?;
    let deadline = tokio::time::Instant::now().checked_add(PATIENCE)?;
    loop {
        let message = tokio::time::timeout_at(deadline, socket.next())
            .await
            .ok()??;
        let Ok(Message::Binary(sealed)) = message else {
            continue;
        };
        let opened = session.opener.open(&sealed).ok()?;
        let Payload::Frame(text) = Payload::from_bytes(&opened).ok()? else {
            continue;
        };
        let frame: wire::ServerFrame = serde_json::from_str(&text).ok()?;
        if wanted(&frame) {
            return Some(frame);
        }
    }
}

pub(crate) async fn pair(
    at: &str,
    invitation: &Invitation,
    key: &SigningKey,
) -> (DeviceId, VerifyingKey) {
    let mut socket = connect(at, "/remote/pair").await;
    let pairing = handshake::device_pair_hello([1; 32]).unwrap();
    send(&mut socket, pairing.hello().as_bytes().to_vec()).await;
    let reply = PairReply::from_bytes(&receive(&mut socket).await).unwrap();
    let claimed = pairing.claim(&reply, invitation, key).unwrap();
    send(&mut socket, claimed.sealed.clone()).await;
    let mut session = claimed.session;
    let id = session.opener.open(&receive(&mut socket).await).unwrap();
    (
        DeviceId::from_entropy(<[u8; 16]>::try_from(id).unwrap()),
        claimed.city,
    )
}

pub(crate) async fn admitted(
    at: &str,
    id: DeviceId,
    city: &VerifyingKey,
    key: &SigningKey,
) -> (Socket, Session) {
    let mut socket = connect(at, "/remote/session").await;
    let waiting = handshake::device_hello(id, [7; 32]).unwrap();
    send(&mut socket, waiting.hello().as_bytes().to_vec()).await;
    let reply = Reply::from_bytes(&receive(&mut socket).await).unwrap();
    let (finish, session) = waiting.finish(&reply, city, key).unwrap();
    send(&mut socket, finish.as_bytes().to_vec()).await;
    (socket, session)
}

pub(crate) async fn connect(at: &str, path: &str) -> Socket {
    connect_async(format!("ws://{at}{path}")).await.unwrap().0
}

pub(crate) async fn send(socket: &mut Socket, bytes: Vec<u8>) {
    socket.send(Message::Binary(bytes.into())).await.unwrap();
}

pub(crate) async fn receive(socket: &mut Socket) -> Vec<u8> {
    loop {
        match tokio::time::timeout(PATIENCE, socket.next()).await {
            Ok(Some(Ok(Message::Binary(bytes)))) => return bytes.to_vec(),
            Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
            other => panic!("the remote listener sent no sealed bytes: {other:?}"),
        }
    }
}

pub(crate) fn choose_this_binary_as_the_route(city_root: &Path) {
    let file = kernel::layout::CityLayout::new(city_root).city_config();
    let before = std::fs::read_to_string(&file).unwrap_or_default();
    let program = std::env::current_exe().unwrap();
    let table = format!(
        "\n[remote]\nroute = \"command\"\ncommand = '{}'\n\
         args = [\"route_stand_in\", \"--exact\", \"--ignored\", \"--nocapture\"]\n\
         permanence = \"fixed\"\n",
        program.display()
    );
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, before + &table).unwrap();
}

pub(crate) fn between(printed: &str, from: &str, end: fn(char) -> bool) -> String {
    let (_, after) = printed
        .split_once(from)
        .unwrap_or_else(|| panic!("no `{from}` in what the console printed:\n{printed}"));
    after.split(end).next().unwrap_or_default().to_owned()
}

pub(crate) fn lines_of(stream: impl Read + Send + 'static) -> Receiver<String> {
    let (sender, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines() {
            let Ok(line) = line else { return };
            if sender.send(line).is_err() {
                return;
            }
        }
    });
    lines
}

#[allow(
    clippy::disallowed_methods,
    reason = "test code: the patience is read off the wall clock"
)]
pub(crate) fn read_until<'a>(
    lines: &Receiver<String>,
    ends: &[&'a str],
) -> (String, Option<&'a str>) {
    let deadline = Instant::now().checked_add(PATIENCE).unwrap();
    let mut read = String::new();
    while let Ok(next) = lines.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        read.push_str(&next);
        read.push('\n');
        if let Some(end) = ends.iter().copied().find(|end| next.contains(end)) {
            return (read, Some(end));
        }
    }
    (read, None)
}

pub(crate) struct Served {
    child: Child,
    console: ChildStdin,
    pub(crate) printed: Receiver<String>,

    logged: Receiver<String>,
}

impl Served {
    #[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
    pub(crate) fn start(city_root: &Path) -> Served {
        // boundary-ok: the remote door is opened from the served city's console, so the check starts the city it reaches (`Sprawling/app/Spec.lean` §8-151)
        let mut child = Command::new(env!("CARGO_BIN_EXE_sprawling"))
            .arg("serve")
            .arg(city_root)
            .arg("127.0.0.1:0")
            .arg("--console")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let printed = lines_of(child.stdout.take().unwrap());
        let logged = lines_of(child.stderr.take().unwrap());
        let console = child.stdin.take().unwrap();

        let served = Served {
            child,
            console,
            printed,
            logged,
        };
        let (log, end) = read_until(&served.logged, &[PROVED, STOPPED]);
        assert_eq!(
            end,
            Some(PROVED),
            "the served city did not prove its history within {PATIENCE:?}; its log:\n{log}"
        );
        served
    }

    pub(crate) fn type_line(&mut self, line: &str, answer: &str) -> String {
        writeln!(self.console, "{line}").unwrap();
        self.console.flush().unwrap();
        let (printed, end) = read_until(&self.printed, &[answer]);
        assert!(
            end.is_some(),
            "`{line}` was not answered with `{answer}` within {PATIENCE:?}; the console printed:\n{printed}"
        );
        printed
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        drop(self.child.kill());
        drop(self.child.wait());
    }
}

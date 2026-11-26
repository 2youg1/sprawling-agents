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
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
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
        approval: None,
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

pub(crate) fn visit(at: &str, invitation: &Invitation) -> Visit {
    let key = SigningKey::from_seed(&[9; 32]).unwrap();
    let (id, city) = pair(at, invitation, &key);
    let (mut socket, mut session) = admitted(at, id, &city, &key);
    assert_eq!(
        socket.frame(None, &socket.origin, vec![0]),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        socket.frame(Some(&socket.bearer), "https://another.example", vec![0]),
        StatusCode::FORBIDDEN
    );
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
    });
    let read_answered = exchange(&mut socket, &mut session, &asking, |frame| {
        matches!(frame, wire::ServerFrame::Answered(_))
    });
    let act_carried_out = exchange(
        &mut socket,
        &mut session,
        &halting,
        |frame| matches!(frame, wire::ServerFrame::Event(record) if record.kind() == EventKind::CityHalted),
    );
    let local_only_refused = exchange(&mut socket, &mut session, &revealing, |frame| {
        matches!(frame, wire::ServerFrame::Refusal(_))
    });
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

fn exchange(
    socket: &mut Pipe,
    session: &mut Session,
    frame: &wire::ClientFrame,
    wanted: fn(&wire::ServerFrame) -> bool,
) -> Option<wire::ServerFrame> {
    let text = serde_json::to_string(frame).unwrap();
    let sealed = session
        .sealer
        .seal(&Payload::Frame(text).to_bytes())
        .unwrap();

    socket.send(sealed);
    loop {
        let sealed = socket.receive();
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

fn pair(at: &str, invitation: &Invitation, key: &SigningKey) -> (DeviceId, VerifyingKey) {
    let pairing = handshake::device_pair_hello([1; 32]).unwrap();
    let mut socket = Pipe::open(at, "/remote/pair", pairing.hello().as_bytes().to_vec());
    let reply = PairReply::from_bytes(&socket.receive()).unwrap();
    let claimed = pairing.claim(&reply, invitation, key).unwrap();
    socket.send(claimed.sealed);
    let mut session = claimed.session;
    let id = session.opener.open(&socket.receive()).unwrap();
    assert_eq!(socket.packet(), (1, Vec::new()));
    assert_eq!(socket.response.read(&mut [0_u8; 1]).unwrap(), 0);
    assert_eq!(
        socket.frame(Some(&socket.bearer), &socket.origin, vec![0]),
        StatusCode::FORBIDDEN
    );
    (
        DeviceId::from_entropy(<[u8; 16]>::try_from(id).unwrap()),
        claimed.city,
    )
}

fn admitted(at: &str, id: DeviceId, city: &VerifyingKey, key: &SigningKey) -> (Pipe, Session) {
    let waiting = handshake::device_hello(id, [7; 32]).unwrap();
    let mut socket = Pipe::open(at, "/remote/session", waiting.hello().as_bytes().to_vec());
    let reply = Reply::from_bytes(&socket.receive()).unwrap();
    let (finish, session) = waiting.finish(&reply, city, key).unwrap();
    socket.send(finish.as_bytes().to_vec());
    (socket, session)
}

pub(crate) async fn connect(at: &str, path: &str) -> Socket {
    connect_async(format!("ws://{at}{path}")).await.unwrap().0
}

struct Pipe {
    client: Client,
    response: Response,
    origin: String,
    frames: String,
    bearer: String,
}

impl Pipe {
    fn open(at: &str, path: &str, first: Vec<u8>) -> Self {
        let origin = format!("https://{at}");
        let url = format!("http://{at}{path}");
        let client = gateway::client_for(kernel::Proxying::ExceptLocal, &url)
            .timeout(PATIENCE)
            .build()
            .unwrap();
        let frames = format!("http://{at}/remote/frame");
        let response = client
            .post(url)
            .header("origin", &origin)
            .header("sec-fetch-site", "same-origin")
            .body(first)
            .send()
            .unwrap()
            .error_for_status()
            .unwrap();
        let bearer = response.headers()["x-sprawling-pipe"]
            .to_str()
            .unwrap()
            .to_owned();
        assert_eq!(bearer.len(), 64);
        assert_eq!(response.headers()["cache-control"], "no-store");
        Self {
            client,
            response,
            origin,
            frames,
            bearer,
        }
    }

    fn send(&self, bytes: Vec<u8>) {
        assert_eq!(
            self.frame(Some(&self.bearer), &self.origin, bytes),
            StatusCode::NO_CONTENT
        );
    }

    fn frame(&self, token: Option<&str>, origin: &str, body: Vec<u8>) -> StatusCode {
        let request = self
            .client
            .post(&self.frames)
            .header("origin", origin)
            .body(body);
        let request = match token {
            Some(token) => request.bearer_auth(token),
            None => request,
        };
        request.send().unwrap().status()
    }

    fn receive(&mut self) -> Vec<u8> {
        let (kind, body) = self.packet();
        assert_eq!(kind, 0, "the city stopped before sending the next frame");
        body
    }

    fn packet(&mut self) -> (u8, Vec<u8>) {
        let mut header = [0_u8; 5];
        self.response.read_exact(&mut header).unwrap();
        let kind = header[0];
        let length = u32::from_be_bytes(header[1..].try_into().unwrap());
        let mut body = vec![0_u8; length.try_into().unwrap()];
        self.response.read_exact(&mut body).unwrap();
        (kind, body)
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

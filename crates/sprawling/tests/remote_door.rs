// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The remote door end to end, over real sockets (sprawling-SPEC.md
//! 8-151).
//!
//! A served city, a person typing `/remote` on its console, a route the
//! city's `[remote]` table chose, and a device speaking the remote door
//! through the loopback listener: pairing, a session, one frame of each
//! verb class, and the door closed again. The Ledger is read last,
//! because what the door did is what its history says.
//!
//! **The route is this test binary.** The table names [`route_stand_in`]
//! as a command route; started by the city, it reads the loopback
//! address from `SPRAWLING_REMOTE_LOCAL` and answers that address itself
//! as the address outside uses, so "outside" is this machine and the
//! test needs no tunnel and no second program.
//!
//! **Why Rust and not the Lean checker** (sprawling-SPEC.md section 12):
//! the device's half of the pairing and session handshakes and of the
//! seal exists only in `remote_access`. The lines that start the binary
//! and open WebSockets carry the boundary waiver for that reason.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use kernel::{Address, AxCode, EventKind, EventRecord, IdemKey, RunId, Seq};
use remote_access::door::DeviceId;
use remote_access::handshake::{self, CityFingerprint, Invitation, PairReply, Reply, Session};
use remote_access::keys::{SigningKey, VerifyingKey};
use remote_access::route::command::LOCAL_ENV;
use remote_access::seal::Payload;
use tokio::net::TcpStream;
// boundary-ok: the device's half of the remote door exists only in remote_access, so this check speaks it from Rust (sprawling-SPEC.md 8-151)
use tokio_tungstenite::tungstenite::Message;
// boundary-ok: the same device, on the remote listener this city opened
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// How long any one step may take: a debug binary opening a city, the
/// route starting, a frame crossing the relay.
const PATIENCE: Duration = Duration::from_secs(60);

/// What the device is called when it pairs.
const PHONE: &str = "phone";

/// The route command the end-to-end test names in the city's `[remote]`
/// table. Run by a person or a test runner it finds no loopback address
/// and returns at once.
#[test]
#[ignore = "the route command the end-to-end test starts; it does nothing on its own"]
fn route_stand_in() {
    let Ok(local) = std::env::var(LOCAL_ENV) else {
        return;
    };
    // On a line of its own: the harness may have begun one already.
    println!("\n{{\"url\": \"https://{local}\"}}");
    // The city ends this process when the door closes.
    std::thread::sleep(PATIENCE.saturating_mul(2));
}

#[test]
fn a_device_reaches_the_city_through_the_door_over_real_sockets() {
    let dir = tempfile::tempdir().unwrap();
    let raised = sprawling::assembly::init_city(dir.path()).unwrap();
    choose_this_binary_as_the_route(dir.path());
    let mut city = Served::start(dir.path());

    let opened = city.type_line("/remote open --for 1h", "the remote door is open");
    let at = between(&opened, "at https://", char::is_whitespace);
    let invited = city.type_line(&format!("/remote pair {PHONE}"), "pairing code");
    let invitation = Invitation {
        city: CityFingerprint::read(&between(&invited, "&city=", char::is_whitespace)).unwrap(),
        code: between(&invited, "#pair=", |each| each == '&'),
    };

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let answered = runtime.block_on(visit(&at, &invitation));
    let closed = city.type_line("/remote close", "the remote door is closed");
    drop(city);

    let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
    let door: Vec<EventKind> = verified
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap().kind())
        .filter(|kind| {
            matches!(
                kind,
                EventKind::RemoteOpened
                    | EventKind::DevicePaired
                    | EventKind::RemoteSessionStarted
                    | EventKind::CityHalted
                    | EventKind::RemoteClosed
            )
        })
        .collect();
    assert_eq!(
        (answered, door),
        (
            Visit {
                welcomed: true,
                read_answered: true,
                act_carried_out: true,
                local_only_refused: Some(AxCode::GateDenied),
            },
            vec![
                EventKind::RemoteOpened,
                EventKind::DevicePaired,
                EventKind::RemoteSessionStarted,
                EventKind::CityHalted,
                EventKind::RemoteClosed,
            ]
        ),
        "the console printed, closing:\n{closed}"
    );
}

/// What the device saw in its one session.
#[derive(Debug, PartialEq, Eq)]
struct Visit {
    /// The city welcomed the greeting the relay carried in.
    welcomed: bool,
    /// The question (`Read`) came back answered.
    read_answered: bool,
    /// The halt (`Act`) came back as the city's own `city_halted`.
    act_carried_out: bool,
    /// The code the reveal (`LocalOnly`) was refused with.
    local_only_refused: Option<AxCode>,
}

/// Pairs, opens a session, and sends one frame of each class, each after
/// the city answered the one before.
async fn visit(at: &str, invitation: &Invitation) -> Visit {
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

/// Seals `frame` for the city, sends it, and answers the first frame
/// back that `wanted` accepts; `None` when none came in time.
async fn exchange(
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
    send(socket, sealed).await;
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

/// The pairing handshake on `/remote/pair`: the device's id and the city
/// key it now pins.
async fn pair(at: &str, invitation: &Invitation, key: &SigningKey) -> (DeviceId, VerifyingKey) {
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

/// The session handshake on `/remote/session`, and the device's half of
/// the session it opened.
async fn admitted(
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

async fn connect(at: &str, path: &str) -> Socket {
    connect_async(format!("ws://{at}{path}")).await.unwrap().0
}

async fn send(socket: &mut Socket, bytes: Vec<u8>) {
    socket.send(Message::Binary(bytes.into())).await.unwrap();
}

/// The next binary message, or a panic naming what came instead.
async fn receive(socket: &mut Socket) -> Vec<u8> {
    loop {
        match tokio::time::timeout(PATIENCE, socket.next()).await {
            Ok(Some(Ok(Message::Binary(bytes)))) => return bytes.to_vec(),
            Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
            other => panic!("the remote listener sent no sealed bytes: {other:?}"),
        }
    }
}

/// Appends a `[remote]` table to the city's own file that names this
/// test binary, run as [`route_stand_in`], as the route.
fn choose_this_binary_as_the_route(city_root: &Path) {
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

/// The text in `printed` after `from`, up to the first character `end`
/// accepts.
fn between(printed: &str, from: &str, end: fn(char) -> bool) -> String {
    let (_, after) = printed
        .split_once(from)
        .unwrap_or_else(|| panic!("no `{from}` in what the console printed:\n{printed}"));
    after.split(end).next().unwrap_or_default().to_owned()
}

/// A served city with its console on this test's pipes, ended when
/// dropped.
struct Served {
    child: Child,
    console: ChildStdin,
    printed: Receiver<String>,
}

impl Served {
    fn start(city_root: &Path) -> Served {
        // A port of its own rather than `:0`: the remote door's relay
        // reaches the city at the address `serve` was given
        // (sprawling-SPEC.md 8-151).
        let port = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        // boundary-ok: the remote door is opened from the served city's console, so the check starts the city it reaches (sprawling-SPEC.md 8-151)
        let mut child = Command::new(env!("CARGO_BIN_EXE_sprawling"))
            .arg("serve")
            .arg(city_root)
            .arg(format!("127.0.0.1:{port}"))
            .arg("--console")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, printed) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if sender.send(line).is_err() {
                    return;
                }
            }
        });
        let console = child.stdin.take().unwrap();
        Served {
            child,
            console,
            printed,
        }
    }

    /// Types `line` at the console and answers what it printed up to the
    /// first line holding `answer`, waiting up to [`PATIENCE`] for each
    /// line it prints.
    fn type_line(&mut self, line: &str, answer: &str) -> String {
        writeln!(self.console, "{line}").unwrap();
        self.console.flush().unwrap();
        let mut printed = String::new();
        while let Ok(next) = self.printed.recv_timeout(PATIENCE) {
            printed.push_str(&next);
            printed.push('\n');
            if next.contains(answer) {
                return printed;
            }
        }
        assert!(
            printed.contains(answer),
            "`{line}` was not answered with `{answer}`; the console printed:\n{printed}"
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

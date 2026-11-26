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
use kernel::{AxCode, IdemKey, RunId, Seq};
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

pub(crate) async fn connect(at: &str, path: &str) -> Socket {
    connect_async(format!("ws://{at}{path}")).await.unwrap().0
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

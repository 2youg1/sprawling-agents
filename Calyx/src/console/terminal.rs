// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::sync::Arc;

use super::cli::{Next, Say, Session};
use super::lifecycle::{Asker, Event, Face, Surface};
pub use wire::Answering;

const HEADLESS_DEPTH: usize = 256;

pub struct Terminal {
    pub url: String,
    pub token: Option<String>,
    pub city: String,
    pub client: String,
    pub bind: SocketAddr,
    pub surface: Surface,
}

pub type Clock = Arc<dyn Fn() -> Result<kernel::TimeMs, kernel::AxError> + Send + Sync>;

pub type CarryRemote = Arc<dyn Fn(&str) -> String + Send + Sync>;

pub type OpenPaired =
    Arc<dyn Fn(&str, &wire::LocalDoor) -> Result<(), kernel::AxError> + Send + Sync>;

pub struct Inside {
    pub desk: Arc<accounting::worker::CommandDesk>,
    pub answering: Answering,
    pub carry_remote: CarryRemote,
    pub open_paired: OpenPaired,
    pub door: wire::LocalDoor,
    pub lifecycle: tokio::sync::mpsc::Sender<Event>,
    pub clock: Clock,
}

impl Inside {
    pub fn tell(&self, event: Event) {
        match self.lifecycle.blocking_send(event) {
            Ok(()) | Err(_) => {}
        }
    }
}

pub fn serving(terminal: &Terminal, vitals: Option<&wire::MetricsAnswer>, pid: u32) -> String {
    let reach = if terminal.bind.ip().is_loopback() {
        "this machine only"
    } else if terminal.bind.ip().is_unspecified() {
        "every interface on this machine"
    } else {
        "one interface, reachable from the network"
    };
    let door = match (terminal.token.is_some(), terminal.bind.ip().is_loopback()) {
        (true, _) => "a pairing key is required",
        (false, true) => "a key - this machine's key file, or a paired browser's session",
        (false, false) => "the pairing token you configured, or a paired browser's session",
    };
    let counts = match vitals {
        Some(v) => format!(
            "    runs      {} active, {} frozen\n    \
             waiting   {} approval(s), {} signal(s)\n    \
             holds     {} building(s), {} event(s), {} discard(s) outstanding\n",
            v.runs_active,
            v.runs_frozen,
            v.approvals_waiting,
            v.signals_waiting,
            v.buildings,
            v.events,
            v.discards_outstanding
        ),
        None => "    runs      the city did not answer; its views may be rebuilding\n".to_owned(),
    };
    format!(
        "\n  sprawling is serving.\n\n    \
         city      {}\n    \
         WebUI     {}\n    \
         bound     {}, {reach}\n    \
         door      {door}\n    \
         client    {}\n    \
         process   pid {pid} - `cargo xtask mem {pid}` weighs it\n\n\
         {counts}",
        terminal.city, terminal.url, terminal.bind, terminal.client
    )
}

pub struct Started {
    pub writer: Option<std::thread::JoinHandle<()>>,
    pub notice: Say,
}

pub fn start(
    terminal: Terminal,
    inside: Inside,
    watching: tokio::sync::broadcast::Receiver<wire::Committed>,
    face: tokio::sync::watch::Receiver<Face>,
) -> Started {
    match terminal.surface {
        Surface::Cli | Surface::QuietHost => super::host::start(terminal, inside, watching, face),
        Surface::Headless => {
            let say = lines_to_stdout();
            let notice = Arc::clone(&say);
            std::thread::spawn(move || {
                drive(&terminal, &inside, &mut std::io::stdin().lock(), &say);
            });
            Started {
                writer: None,
                notice,
            }
        }
    }
}

fn lines_to_stdout() -> Say {
    let (to, from) = std::sync::mpsc::sync_channel::<String>(HEADLESS_DEPTH);
    std::thread::spawn(move || {
        let mut out = std::io::stdout();
        for line in from {
            if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
                return;
            }
        }
    });
    Arc::new(move |line: String| {
        if to.send(line).is_err() {
            // Reader gone; headless surface is finished.
        }
    })
}

pub(super) fn drive<R: BufRead>(terminal: &Terminal, inside: &Inside, input: &mut R, say: &Say) {
    let mut session = match Session::begin() {
        Ok(session) => session,
        Err(err) => {
            say(format!("  {err}"));
            say(format!("  {}", err.recovery()));
            return;
        }
    };
    let mut typed = String::new();
    loop {
        typed.clear();
        match input.read_line(&mut typed) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        match session.carry(terminal, inside, &typed, say) {
            Next::Stay => {}
            Next::Web => {
                say(format!("  {}", terminal.url));
                if let Err(unopened) = (inside.open_paired)(&terminal.url, &inside.door) {
                    say(format!("  {}: {}", unopened.action(), unopened.recovery()));
                }
            }
            Next::Quit | Next::AskBeforeQuit(_) => {
                say("  the city is closing; the runs under way finish first".to_owned());
                inside.tell(Event::Quit(Asker::Console, wire::CloseMode::Drain));
            }
        }
    }
}

pub struct LineKeys {
    origin: [u8; 16],
    lines: kernel::Seq,
}

impl LineKeys {
    pub fn drawn() -> Result<LineKeys, kernel::AxError> {
        let mut origin = [0u8; 16];
        getrandom::fill(&mut origin).map_err(|err| {
            kernel::AxError::failure(
                kernel::AxCode::ConfigInvalid,
                "draw an origin for this console's keys",
                err.to_string(),
            )
            .with_recovery(
                "this machine's entropy source refused; the city keeps serving, drive it from the WebUI or `sprawling call`",
            )
        })?;
        Ok(LineKeys {
            origin,
            lines: kernel::Seq::FIRST,
        })
    }

    pub fn next(&mut self) -> kernel::IdemKey {
        let key = kernel::IdemKey::derive(&kernel::RunId::CITY, self.lines, &self.origin);
        self.lines = self.lines.next().unwrap_or(self.lines);
        key
    }
}

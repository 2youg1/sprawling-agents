// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The listener's own facts, and the line console a harness drives
//! (`crates/sprawling/spec/Console.lean` §8-11, §8-21).
//!
//! [`Terminal`] is the one home of what this process knows about its
//! listener - URL, bound address, city directory, client source,
//! pairing code, the face it opens on. Those are facts about this
//! process rather than about a history, so no query can answer them and
//! `/serving` reprints them from here.
//!
//! [`start`] gives the console its face: the raw CLI or the quiet host
//! when the terminal is the city's (`bin::console::ui`), or, when it is
//! a pipe, the line console that reads whole lines and answers each on
//! standard output, which is how a harness drives `/remote`.
//!
//! The point a reader most often gets wrong: end of input ends the line
//! console and not the city - a city that stopped answering because
//! nobody was typing would have made interaction a condition of service.

use std::io::BufRead;
use std::net::SocketAddr;
use std::sync::Arc;

use super::cli::{Next, Say, Session};
use super::lifecycle::{Asker, Event, Face, Surface};
pub(crate) use wire::Answering;

/// What a console needs from the process that started it.
pub struct Terminal {
    pub url: String,
    /// The key a person may be shown - only the one minted for a serve
    /// beyond this machine. It is drawn on the quiet host's alternate
    /// screen and nowhere else: never in an address, the scrollback or
    /// a line console's output. The pairing code a second browser on
    /// this machine types is the door's (`Inside::door`), because each
    /// guess replaces it.
    pub token: Option<String>,
    /// The city directory being served, as a person would type it.
    pub city: String,
    /// Where the client bundle comes from: embedded, or a directory read
    /// per request.
    pub client: String,
    /// The address actually bound, which `url` cannot recover: a city on
    /// `0.0.0.0` is opened at `127.0.0.1` and the difference is exactly
    /// what decides whether strangers can reach it.
    pub bind: SocketAddr,
    /// The face the console opens on.
    pub surface: Surface,
}

/// What the console reaches the city through: the socket's own desk and
/// answering function, the remote door or why this serve has none, and
/// the lifecycle it tells about `/web`, `/quit` and a lost terminal.
pub(crate) struct Inside {
    pub(crate) desk: Arc<accounting::worker::CommandDesk>,
    pub(crate) answering: Answering,
    pub(crate) remote: Result<crate::outside::console::Remote, kernel::AxError>,
    /// This machine's door for a browser: `/web` asks it for an open
    /// code, and the quiet host shows its pairing code
    /// (`crates/sprawling/spec/Firstrun.lean` §8-8).
    pub(crate) door: wire::LocalDoor,
    pub(crate) lifecycle: tokio::sync::mpsc::Sender<Event>,
}

impl Inside {
    /// Tells the lifecycle what happened. A lifecycle that stopped
    /// listening belongs to a process that is ending, so nothing is lost.
    pub(crate) fn tell(&self, event: Event) {
        match self.lifecycle.blocking_send(event) {
            Ok(()) | Err(_) => {}
        }
    }
}

/// What this process is doing, in one screen.
///
/// Pure: the listener's own facts come from [`Terminal`], the city's
/// counts come from a `MetricsAnswer` the caller already obtained, and
/// the process identifier arrives as a parameter. Nothing here reads a
/// clock, a socket or a disk, so the whole readout is one comparison in
/// a test.
///
/// **Resident memory is deliberately absent.** What "resident" means
/// differs per platform, and `xtask::mem` is the one authority that says
/// so: `smaps_rollup` Pss on Linux, `ps rss` on macOS, `WorkingSet64` on
/// Windows. A second table here would be the third authority that
/// module's own documentation warns about, so this prints the process
/// identifier and the measurement stays one paste away.
pub(crate) fn serving(
    terminal: &Terminal,
    vitals: Option<&wire::MetricsAnswer>,
    pid: u32,
) -> String {
    let reach = if terminal.bind.ip().is_loopback() {
        "this machine only"
    } else if terminal.bind.ip().is_unspecified() {
        "every interface on this machine"
    } else {
        "one interface, reachable from the network"
    };
    // Every face demands a key (wire D54); what differs is who holds it.
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
        // The listener's half is still true and still worth having:
        // printing nothing because one of two sources was quiet would
        // lose the half that answers "which port".
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

/// A console that has started: the writer whose end means the terminal
/// is put back, when the terminal is the city's, and where a line meant
/// for the person wherever they are goes, which the remote door's
/// confirmation codes use.
pub(crate) struct Started {
    pub(crate) writer: Option<std::thread::JoinHandle<()>>,
    pub(crate) notice: Say,
}

/// Starts the console on its face.
///
/// Spawned rather than awaited because reading a keyboard blocks and the
/// reactor is serving a city.
pub(crate) fn start(
    terminal: Terminal,
    inside: Inside,
    watching: tokio::sync::broadcast::Receiver<wire::Committed>,
    face: tokio::sync::watch::Receiver<Face>,
) -> Started {
    match terminal.surface {
        Surface::Cli | Surface::QuietHost => {
            let (writer, notice) = super::ui::start(terminal, inside, watching, face);
            Started {
                writer: Some(writer),
                notice,
            }
        }
        Surface::Headless => {
            let say = super::ui::lines_to_stdout();
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

/// The line console, over any reader so a test can drive it: each whole
/// line carried by the same [`Session`] the CLI uses, each answer one
/// line through `say`.
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
            // End of input: a pipe, a service, a machine with nobody at
            // it. The console stops; the city does not.
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        match session.carry(terminal, inside, &typed, say) {
            Next::Stay => {}
            Next::Web => {
                say(format!("  {}", terminal.url));
                // Never fatal: the URL is on the screen either way. The
                // browser is handed an open code through a file only this
                // account reads, never a key on its command line.
                if let Err(unopened) = crate::firstrun::open_paired(&terminal.url, &inside.door) {
                    say(format!("  {}: {}", unopened.action(), unopened.recovery()));
                }
            }
            // Nobody is asked here: a line console has no keys to answer
            // with, so the runs under way are waited for.
            Next::Quit | Next::AskBeforeQuit(_) => {
                say("  the city is closing; the runs under way finish first".to_owned());
                inside.tell(Event::Quit(Asker::Console, wire::CloseMode::Drain));
            }
        }
    }
}

/// One idempotency key per typed line.
///
/// The city answers a key it has seen with its first answer, and keeps
/// every key across restarts, so a key derived from the words alone
/// swallowed a line typed twice and replayed a refusal after its cause
/// was fixed. The line count separates two lines of one console; the
/// origin, drawn from OS entropy, separates two consoles that reach the
/// same count.
pub(crate) struct LineKeys {
    origin: [u8; 16],
    lines: kernel::Seq,
}

impl LineKeys {
    pub(crate) fn drawn() -> Result<LineKeys, kernel::AxError> {
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

    /// The key for the line just read, advancing to the next line.
    pub(crate) fn next(&mut self) -> kernel::IdemKey {
        let key = kernel::IdemKey::derive(&kernel::RunId::CITY, self.lines, &self.origin);
        // A console that has read 2^64 lines reuses its last key; no
        // person reaches it, and wrapping would reuse the first one.
        self.lines = self.lines.next().unwrap_or(self.lines);
        key
    }
}

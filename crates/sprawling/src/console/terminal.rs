// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The console itself: the listener's own facts, and the loop that
//! carries judged lines into the city (`crates/sprawling/spec/Console.lean` §8-11).
//!
//! [`Terminal`] is the one home of what the startup banner printed —
//! URL, bound address, city directory, client source, pairing token.
//! Those are facts about this process rather than about a history, so no
//! query can answer them and `/serving` reprints them from here, beside
//! counts that come from the `Metrics` query a browser also asks.
//!
//! The loop runs on threads of its own ([`start`]) because reading a
//! keyboard blocks while the reactor serves a city, and [`drive`] takes
//! any reader and writer so a test drives exactly the loop a person
//! does. A command goes onto the `CommandDesk` a browser's frames land
//! on and a question goes into the `Answering` function the socket
//! calls, so this console decides nothing the server does not.
//!
//! The point a reader most often gets wrong: end of input ends the
//! console and not the city — a city that stopped answering because
//! nobody was typing would have made interaction a condition of service.

use super::language::{Line, help, parse};

/// What a console needs from the process that started it.
///
/// The pairing token arrives as a copy of the one `serve` already read,
/// so `/web` can carry it and nobody has to transcribe a secret. It is
/// not re-read from the environment here: one read, one authority.
///
/// The other three are what the startup banner printed and the event
/// stream then scrolled away. A person watching a city from somewhere
/// else needs them back on demand, and this process is the only thing
/// that knows them - they are facts about a listener, not about a
/// history, so no query can answer them.
pub struct Terminal {
    pub url: String,
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
    /// How much of each committed record the event stream prints.
    pub records: Records,
}

/// How much of each committed record the console prints.
///
/// A record carries what the User typed and what a model answered, and a
/// terminal is read over a shoulder, scrolled into a recording and kept in
/// a scrollback file, so the default is the line that says what happened
/// and where, and the payload is shown only when the User asks for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Records {
    /// One line per record: its seq, its kind and its address.
    Summary,
    /// The record whole, payload included, as `sprawling call` prints it.
    Whole,
}

use kernel::Address;
use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::sync::Arc;
pub(crate) use wire::Answering;

/// What the console reaches the city through: the socket's own desk and
/// answering function, and the remote door or why this serve has none.
pub(crate) struct Inside {
    pub(crate) desk: Arc<accounting::worker::CommandDesk>,
    pub(crate) answering: Answering,
    pub(crate) remote: Result<crate::outside::console::Remote, kernel::AxError>,
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
    let door = match (terminal.token.is_some(), terminal.bind.ip().is_loopback()) {
        (true, _) => "a pairing key is required",
        (false, true) => "none - a loopback listener asks for nothing",
        // Worth saying plainly rather than leaving to be discovered: an
        // unkeyed listener beyond loopback is a city that anyone able to
        // route to it may drive.
        (false, false) => "NONE - anyone who can reach this address can drive this city",
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

/// The URL `/web` opens, with the pairing token on it when there is one.
///
/// A token in a query string is a token in the browser's history, and
/// that is the trade this makes deliberately: the alternative is a
/// person copying a secret by hand between two windows, which they will
/// do wrongly and then paste somewhere worse.
pub(crate) fn web_url(terminal: &Terminal) -> String {
    match &terminal.token {
        None => terminal.url.clone(),
        Some(token) => format!("{}/?token={token}", terminal.url.trim_end_matches('/')),
    }
}

/// Runs the console on threads of its own until the input ends.
///
/// Spawned rather than awaited because reading a keyboard blocks and the
/// reactor is serving a city. Ending the input ends the console and
/// **not** the city: one that stopped answering because nobody was
/// typing would have made interaction a condition of service.
pub(crate) fn start(
    terminal: Terminal,
    inside: Inside,
    mut watching: tokio::sync::broadcast::Receiver<wire::Committed>,
) {
    let records = terminal.records;
    std::thread::spawn(move || {
        while let Ok(committed) = watching.blocking_recv() {
            match printed(committed.record(), records) {
                Ok(text) => println!("{text}"),
                // The record is on the ledger and reached every socket as
                // its frame; only this printout lacks it, so the gap is
                // named rather than left silent.
                Err(error) => eprintln!(
                    "  seq {} is in the history but could not be printed here: {error}",
                    committed.record().seq().value()
                ),
            }
        }
    });
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        drive(
            &terminal,
            &inside,
            &mut stdin.lock(),
            &mut std::io::stdout(),
        );
    });
}

/// One committed record, as the event stream prints it.
///
/// The kind is spelled by its serde name, the one the ledger and the wire
/// use, so the summary line names no kind a second way; the whole record
/// is the shape `sprawling call` prints.
pub(super) fn printed(
    record: &kernel::EventRecord,
    records: Records,
) -> Result<String, serde_json::Error> {
    match records {
        Records::Whole => serde_json::to_string(record),
        Records::Summary => {
            let kind = serde_json::to_value(record.kind())?;
            let kind = kind.as_str().unwrap_or("?");
            let at = record.addr().map_or("city", Address::as_str);
            Ok(format!("  seq {}  {kind}  {at}", record.seq().value()))
        }
    }
}

/// One line to the console.
///
/// A console whose writes fail is a console nobody is reading, and the
/// city is not the console's to stop, so the failure ends here — stated
/// once rather than at every line this file prints.
fn say<W: Write>(out: &mut W, line: &str) {
    drop(out.write_all(line.as_bytes()));
    drop(out.write_all(b"\n"));
}

/// The loop, over any reader and writer so a test can drive it.
pub(super) fn drive<R: BufRead, W: Write>(
    terminal: &Terminal,
    inside: &Inside,
    input: &mut R,
    out: &mut W,
) {
    let Inside {
        desk, answering, ..
    } = inside;
    let mut keys = match LineKeys::drawn() {
        Ok(keys) => keys,
        Err(err) => {
            say(out, &format!("  {err}"));
            say(out, &format!("  {}", err.recovery()));
            return;
        }
    };
    let mut selected: Option<Address> = None;
    let mut typed = String::new();
    loop {
        typed.clear();
        if write!(out, "> ").is_err() || out.flush().is_err() {
            return;
        }
        match input.read_line(&mut typed) {
            // End of input: a pipe, a service, a machine with nobody at
            // it. The console stops; the city does not.
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let idem = keys.next();
        match parse(&typed, selected.as_ref(), idem) {
            Line::Nothing => {}
            Line::Help => {
                say(out, &help(selected.as_ref()));
            }
            Line::OpenWeb => {
                let url = web_url(terminal);
                say(out, &format!("  {url}"));
                // Never fatal: the URL is on the screen either way.
                drop(crate::firstrun::open_in_browser(&url));
            }
            Line::Serving => {
                // The counts come from the one question that already
                // owns them, so this screen renders a number it never
                // computes. A city too busy to answer still has a port.
                let vitals = match answering(wire::Query::Metrics) {
                    (_, Ok(wire::Answer::Metrics(vitals))) => Some(*vitals),
                    (_, Ok(_) | Err(_)) => None,
                };
                say(out, &serving(terminal, vitals.as_ref(), std::process::id()));
            }
            Line::Select(addr) => {
                say(out, &format!("  work goes to {}", addr.as_str()));
                selected = Some(addr);
            }
            Line::Remote(line) => {
                say(out, &crate::outside::console::carry(&inside.remote, line));
            }
            Line::Quit => {
                say(out, "  the console is closing; the city keeps serving");
                return;
            }
            Line::Unknown { verb, nearest } => {
                say(out, &format!("  no verb `{verb}`"));
                if !nearest.is_empty() {
                    say(out, &format!("  did you mean: {}", nearest.join(", ")));
                }
            }
            Line::Malformed { verb, reason } => {
                say(out, &format!("  `/{verb}` cannot be read: {reason}"));
                say(
                    out,
                    "  the body is the wire's own JSON; `idem` may be left out",
                );
            }
            Line::Frame(frame) => post(desk, answering, *frame, out),
            Line::Work(task) => {
                let Some(addr) = selected.clone() else {
                    continue;
                };
                post(desk, answering, dispatch(&addr, &task, idem), out);
            }
        }
    }
}

/// One frame, onto the same desk a browser's frames land on, or into the
/// same answering function a browser's questions reach.
fn post<W: Write>(
    desk: &accounting::worker::CommandDesk,
    answering: &Answering,
    frame: wire::ClientFrame,
    out: &mut W,
) {
    match frame {
        wire::ClientFrame::Command(command) => {
            // A refusal comes back here rather than into a log file, over
            // the reply address the socket path already uses.
            desk.post(
                (*command).into(),
                wire::Reply::to(move |error: kernel::AxError| {
                    eprintln!("  {error}");
                    eprintln!("  {}", error.recovery());
                    wire::Delivered::ToThePeer
                }),
            );
        }
        // A question is not desk work: nothing is queued and nothing is
        // refused later. It goes to the same function the socket calls,
        // so a person inside a city stops being told to open a second
        // terminal and ask it from outside.
        wire::ClientFrame::Ask(ask) => answer(answering, ask.query, out),
        wire::ClientFrame::Hello(_) | wire::ClientFrame::Monitor(_) => {
            say(out, "  this console is already inside the city");
        }
    }
}

/// One question, answered where it was asked.
///
/// JSONL, one object per line - the shape `sprawling call` prints and
/// the shape the event stream above already uses. Tables and diagrams
/// belong to the browser; a console that drew them would be serving two
/// masters at once.
fn answer<W: Write>(answering: &Answering, query: wire::Query, out: &mut W) {
    // The console prints each answer where it was asked, so it pairs
    // nothing and has no use for the date the answer was read at.
    let (_as_of, answered) = answering(query);
    match answered {
        Ok(answer) => match serde_json::to_string(&answer) {
            Ok(text) => {
                say(out, &text);
            }
            // An answer this build can produce but not spell is a defect
            // in the wire type, and hiding it would make the console
            // silently lossy about the one thing it exists to show.
            Err(err) => {
                say(out, &format!("  the answer could not be rendered: {err}"));
            }
        },
        Err(error) => {
            say(out, &format!("  {error}"));
            say(out, &format!("  {}", error.recovery()));
        }
    }
}

/// A line of work, as the Command a browser would have sent for it.
fn dispatch(addr: &Address, task: &str, idem: kernel::IdemKey) -> wire::ClientFrame {
    wire::ClientFrame::Command(Box::new(wire::WireCommand::Dispatch {
        addr: addr.clone(),
        task: task.to_owned(),
        goal: String::new(),
        policy: wire::RunPolicy::of(wire::Mode::Work),
        idem,
        // `/at` already chose the room; a line typed after it
        // continues what is working there.
        session: None,
        effort: None,
        model: None,
    }))
}

/// One idempotency key per typed line.
///
/// The city answers a key it has seen with its first answer, and keeps
/// every key across restarts, so a key derived from the words alone
/// swallowed a line typed twice and replayed a refusal after its cause
/// was fixed. The line count separates two lines of one console; the
/// origin, drawn from OS entropy, separates two consoles that reach the
/// same count.
struct LineKeys {
    origin: [u8; 16],
    lines: kernel::Seq,
}

impl LineKeys {
    fn drawn() -> Result<LineKeys, kernel::AxError> {
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
    fn next(&mut self) -> kernel::IdemKey {
        let key = kernel::IdemKey::derive(&kernel::RunId::CITY, self.lines, &self.origin);
        // A console that has read 2^64 lines reuses its last key; no
        // person reaches it, and wrapping would reuse the first one.
        self.lines = self.lines.next().unwrap_or(self.lines);
        key
    }
}

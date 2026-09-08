// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a line typed into a serving city means (sprawling-SPEC.md
//! section 8-11).
//!
//! `sprawling up` used to print four lines and block until Ctrl-C. That
//! terminal is a surface the product threw away, and on a machine with
//! no browser it is the only surface there is.
//!
//! Everything here is a pure judgement over one line of text. What the
//! judgement produces is either a control action the terminal carries
//! out or a `ClientFrame` that goes to the same desk a browser's frames
//! go to, so the console decides nothing the server does not.

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
}

/// The one function that turns a question into an answer, shared with
/// the socket rather than reimplemented beside it.
///
/// `assembly::serve` builds it once and hands the same `Arc` to both
/// surfaces, so a number this console prints and a number a browser
/// draws cannot disagree: they are one call.
pub(crate) type Answering =
    Arc<dyn Fn(channels::Query) -> Result<channels::Answer, kernel::AxError> + Send + Sync>;
use kernel::Address;
use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::sync::Arc;

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
    vitals: Option<&channels::MetricsAnswer>,
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
    desk: Arc<crate::serving::CommandDesk>,
    answering: Answering,
    mut watching: tokio::sync::broadcast::Receiver<kernel::EventRecord>,
) {
    // What happened, printed as it happens, one JSON object per line -
    // the same shape `sprawling call` prints, because a second rendering
    // would be a second description of every event kind.
    std::thread::spawn(move || {
        while let Ok(record) = watching.blocking_recv() {
            if let Ok(text) = serde_json::to_string(&record) {
                println!("{text}");
            }
        }
    });
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        drive(
            &terminal,
            &desk,
            &answering,
            &mut stdin.lock(),
            &mut std::io::stdout(),
        );
    });
}

/// The loop, over any reader and writer so a test can drive it.
pub(super) fn drive<R: BufRead, W: Write>(
    terminal: &Terminal,
    desk: &crate::serving::CommandDesk,
    answering: &Answering,
    input: &mut R,
    out: &mut W,
) {
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
        match parse(&typed, selected.as_ref()) {
            Line::Nothing => {}
            Line::Help => {
                let _ = writeln!(out, "{}", help(selected.as_ref()));
            }
            Line::OpenWeb => {
                let url = web_url(terminal);
                let _ = writeln!(out, "  {url}");
                // Never fatal: the URL is on the screen either way.
                let _ = crate::firstrun::open_in_browser(&url);
            }
            Line::Serving => {
                // The counts come from the one question that already
                // owns them, so this screen renders a number it never
                // computes. A city too busy to answer still has a port.
                let vitals = match answering(channels::Query::Metrics) {
                    Ok(channels::Answer::Metrics(vitals)) => Some(*vitals),
                    Ok(_) | Err(_) => None,
                };
                let _ = writeln!(
                    out,
                    "{}",
                    serving(terminal, vitals.as_ref(), std::process::id())
                );
            }
            Line::Select(addr) => {
                let _ = writeln!(out, "  work goes to {}", addr.as_str());
                selected = Some(addr);
            }
            Line::Quit => {
                let _ = writeln!(out, "  the console is closing; the city keeps serving");
                return;
            }
            Line::Unknown { verb, nearest } => {
                let _ = writeln!(out, "  no verb `{verb}`");
                if !nearest.is_empty() {
                    let _ = writeln!(out, "  did you mean: {}", nearest.join(", "));
                }
            }
            Line::Frame(frame) => post(desk, answering, *frame, out),
            Line::Work(task) => {
                let Some(addr) = selected.clone() else {
                    continue;
                };
                match dispatch(&addr, &task) {
                    Ok(frame) => post(desk, answering, frame, out),
                    Err(err) => {
                        let _ = writeln!(out, "  {err}");
                    }
                }
            }
        }
    }
}

/// One frame, onto the same desk a browser's frames land on, or into the
/// same answering function a browser's questions reach.
fn post<W: Write>(
    desk: &crate::serving::CommandDesk,
    answering: &Answering,
    frame: channels::ClientFrame,
    out: &mut W,
) {
    match frame {
        channels::ClientFrame::Command(command) => {
            // A refusal comes back here rather than into a log file, over
            // the reply address the socket path already uses.
            desk.post(
                (*command).into(),
                channels::Reply::to(move |error: kernel::AxError| {
                    eprintln!("  {error}");
                    eprintln!("  {}", error.recovery());
                    channels::Delivered::ToThePeer
                }),
            );
        }
        // A question is not desk work: nothing is queued and nothing is
        // refused later. It goes to the same function the socket calls,
        // so a person inside a city stops being told to open a second
        // terminal and ask it from outside.
        channels::ClientFrame::Query(query) => answer(answering, query, out),
        channels::ClientFrame::Hello(_) => {
            let _ = writeln!(out, "  this console is already inside the city");
        }
    }
}

/// One question, answered where it was asked.
///
/// JSONL, one object per line - the shape `sprawling call` prints and
/// the shape the event stream above already uses. Tables and diagrams
/// belong to the browser; a console that drew them would be serving two
/// masters at once.
fn answer<W: Write>(answering: &Answering, query: channels::Query, out: &mut W) {
    match answering(query) {
        Ok(answer) => match serde_json::to_string(&answer) {
            Ok(text) => {
                let _ = writeln!(out, "{text}");
            }
            // An answer this build can produce but not spell is a defect
            // in the wire type, and hiding it would make the console
            // silently lossy about the one thing it exists to show.
            Err(err) => {
                let _ = writeln!(out, "  the answer could not be rendered: {err}");
            }
        },
        Err(error) => {
            let _ = writeln!(out, "  {error}");
            let _ = writeln!(out, "  {}", error.recovery());
        }
    }
}

/// A line of work, as the Command a browser would have sent for it.
///
/// # Errors
/// Refuses only when the mode tag this console names stops being a mode
/// tag, which would be a change in `channels::wire` this file has not
/// followed - so it is reported rather than assumed away.
fn dispatch(addr: &Address, task: &str) -> Result<channels::ClientFrame, kernel::AxError> {
    Ok(channels::ClientFrame::Command(Box::new(
        channels::WireCommand::Dispatch {
            addr: addr.clone(),
            task: task.to_owned(),
            goal: String::new(),
            mode: channels::ModeTag::parse("plan")?,
            idem: kernel::IdemKey::derive(
                &kernel::RunId::CITY,
                kernel::Seq::FIRST,
                format!("console:{}:{task}", addr.as_str()).as_bytes(),
            ),
            // `/at` already chose the room; a line typed after it
            // continues what is working there.
            session: None,
            effort: None,
        },
    )))
}

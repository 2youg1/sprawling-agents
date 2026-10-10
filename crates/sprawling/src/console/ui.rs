// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one thread that writes the terminal (`crates/sprawling/spec/Console.lean`
//! §8-11): the CLI on the main screen, the quiet host on the alternate
//! one, and the terminal put back on every way out.
//!
//! Every other thread hands this one a [`Show`] over a bounded channel,
//! and a line that finds the channel full is dropped, as a diagnostic
//! may be: a worker or the accounting thread never waits on a terminal.
//! Keys arrive over the same channel from a reader thread. The face is
//! the lifecycle's, read from its `watch`; this thread only draws it.
//!
//! The CLI is a transcript in the terminal's own scrollback and a live
//! region under it - what waits for the person, who is working, the
//! calls under way, and the composer - drawn by the console's renderer
//! (`console_ffi`) one frame at a time, and only when something changed.
//!
//! The point a reader most often gets wrong: a typed Ctrl+C is a key
//! here, not a signal. The city answers it once per session with a hint
//! and otherwise leaves it to the terminal, which copies a selection.

use std::io::Write;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender};
use std::time::Duration;

use console_ffi::scene::Entry;
use crossterm::event::KeyEvent;

use super::cli::{Say, Session};
use super::editor::Editor;
use super::lifecycle::{Event, Face, Handoff, INTERRUPT_GRACE, Sinks};
use super::local_time;
use super::program_status::{self, Held, Reporting};
use super::screen::{QuietLines, Screen, Written};
use super::terminal::{Inside, Terminal};
use menu::Menu;
use wire::CloseMode;

/// How many lines may wait for this thread before the next is dropped.
pub(crate) const UI_DEPTH: usize = 256;

/// How often the thread looks at the lifecycle's face when nothing
/// arrives on its channel.
const FACE_LOOK: Duration = Duration::from_millis(100);

/// How many transcript lines the quiet host keeps for the CLI to show
/// when the person comes back; older ones are in the WebUI.
const HELD_BACK: usize = 512;

/// How many calls under way the live region lists; a wave wider than
/// this shows its newest.
pub(super) const CALLS_SHOWN: usize = 3;

/// What the once-per-session Ctrl+C hint says.
pub(super) const CTRL_C_HINT: &str = "  复制请先选中文字；关闭城市用 /quit";

/// What another thread hands the terminal's one writer.
pub(crate) enum Show {
    /// A line of the session, printed above the prompt.
    Line(String),
    /// A line for the person wherever they are: above the prompt in the
    /// CLI, the transient third line in the quiet host.
    Notice(String),
    Key(KeyEvent),
    /// Text the terminal handed over as one paste.
    Paste(String),
    Record(wire::Committed),
    Resized,
    /// Reading the terminal failed: it is gone.
    Lost,
}

/// A `Say` onto the channel: a full channel drops the line.
fn sayer(to: &SyncSender<Show>, wrap: fn(String) -> Show) -> Say {
    let to = to.clone();
    Arc::new(move |line: String| drop(to.try_send(wrap(line))))
}

/// Starts the writer, the key reader and the record reader, and answers
/// the writer, whose end means the terminal has been put back.
pub(crate) fn start(
    terminal: Terminal,
    inside: Inside,
    watching: tokio::sync::broadcast::Receiver<wire::Committed>,
    face: tokio::sync::watch::Receiver<Face>,
) -> (std::thread::JoinHandle<()>, Say) {
    let (to, from) = std::sync::mpsc::sync_channel(UI_DEPTH);
    let notice = sayer(&to, Show::Notice);
    input::read_keys(to.clone());
    input::read_records(to.clone(), watching);
    let say = sayer(&to, Show::Line);
    let writer = std::thread::spawn(move || {
        let mut ui = Ui::new(terminal, inside, say);
        ui.run(&from, face);
    });
    (writer, notice)
}

/// The writer's state: the terminal, the line being typed, and the face
/// it is drawing.
struct Ui {
    terminal: Terminal,
    inside: Inside,
    say: Say,
    session: Option<Session>,
    editor: Editor,
    shown: Face,
    screen: Screen,
    /// `/quit` with runs going, waiting for Enter, n or Esc: how many.
    asking: Option<u64>,
    hinted: bool,
    /// The slash menu, while Tab has it open.
    menu: Option<Menu>,
    transient: Option<String>,
    /// The door's pairing code, which each guess replaces, so the quiet
    /// host draws the one in force.
    pairing: tokio::sync::watch::Receiver<String>,
    /// Whether the city reports its state to the terminal, and what the
    /// terminal holds (`program_status`).
    reporting: Reporting,
    held: Held,
    /// What plain lines ask, as the composer's settings row writes it.
    offer: String,
    /// Transcript lines that arrived while the quiet host showed.
    held_back: Vec<Entry>,
}

impl Ui {
    fn new(terminal: Terminal, inside: Inside, say: Say) -> Ui {
        let session = match Session::begin() {
            Ok(session) => Some(session),
            Err(err) => {
                say(format!("  {err}"));
                None
            }
        };
        let pairing = inside.door.pairing_code();
        let offer = session
            .as_ref()
            .map(|session| session.offer(&inside))
            .unwrap_or_default();
        Ui {
            terminal,
            inside,
            say,
            session,
            editor: Editor::default(),
            shown: Face::Opening,
            screen: Screen::default(),
            asking: None,
            hinted: false,
            menu: None,
            transient: None,
            pairing,
            reporting: Reporting::from_environment(),
            held: Held::OPENING,
            offer,
            held_back: Vec::new(),
        }
    }

    fn run(&mut self, from: &Receiver<Show>, mut face: tokio::sync::watch::Receiver<Face>) {
        let taken = self.screen.take();
        self.drawn(taken);
        self.print(vec![self.banner()]);
        loop {
            let now = *face.borrow_and_update();
            if now != self.shown {
                self.turn_to(now);
            }
            if matches!(self.pairing.has_changed(), Ok(true)) && self.shown == Face::QuietHost {
                self.quiet();
            }
            if let Face::Gone(_) = self.shown {
                return;
            }
            match from.recv_timeout(FACE_LOOK) {
                Ok(show) => self.show(show),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    /// The first line: the city, and where its page is served.
    fn banner(&self) -> Entry {
        let url = &self.terminal.url;
        let address = url
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/');
        Entry::Banner {
            city: self.terminal.city.clone(),
            address: address.to_owned(),
            url: url.clone(),
        }
    }

    /// Draws the face the lifecycle moved to.
    fn turn_to(&mut self, next: Face) {
        self.shown = next;
        match next {
            Face::QuietHost => {
                self.quiet();
                self.recount();
            }
            Face::Cli => {
                self.transient = None;
                let left = self.screen.leave_quiet();
                self.drawn(left);
                let held = std::mem::take(&mut self.held_back);
                self.redraw(&held);
                self.recount();
            }
            Face::Stopping {
                sinks: Sinks::Cut, ..
            } => self.screen.cut(),
            Face::Stopping {
                mode,
                sinks: Sinks::Held,
            } => {
                let progress = match mode {
                    CloseMode::Drain => {
                        "  sprawling is closing; the runs under way finish first (/quit again stops them now)"
                            .to_owned()
                    }
                    CloseMode::Interrupt => format!(
                        "  sprawling is closing; the runs under way stop at their next safe point, within {} s",
                        INTERRUPT_GRACE.as_secs()
                    ),
                };
                self.notice(progress);
            }
            Face::Gone(handoff) => {
                self.status(program_status::Event::Closed);
                let given = self.screen.give_back();
                self.drawn(given);
                if handoff == Handoff::Skipped {
                    self.line(
                        "  the city exits without a handoff; `sprawling resume` will close what was open",
                    );
                }
            }
            Face::Opening | Face::Headless => {}
        }
    }

    fn show(&mut self, show: Show) {
        match show {
            Show::Line(line) => self.line(&line),
            Show::Notice(line) => self.notice(line),
            Show::Key(key) => self.key(key),
            Show::Paste(pasted) => self.paste(&pasted),
            Show::Record(committed) => {
                self.heard(committed.record());
                let entries = match self.session.as_mut() {
                    Some(session) => {
                        let room = session.room.clone();
                        session
                            .seen
                            .read(committed.record(), &room, local_time::local)
                    }
                    None => Vec::new(),
                };
                self.print(entries);
            }
            Show::Resized => {
                if self.shown == Face::QuietHost {
                    self.quiet();
                } else {
                    self.redraw(&[]);
                }
            }
            Show::Lost => {
                self.screen.cut();
                self.inside.tell(Event::TerminalLost);
            }
        }
    }

    /// A line for the person: the quiet host's third line there, a line
    /// above the prompt everywhere else.
    fn notice(&mut self, line: String) {
        if self.screen.is_quiet() {
            self.transient = Some(line);
            self.quiet();
        } else {
            self.line(&line);
        }
    }

    /// One line from the console itself, in the transcript.
    fn line(&mut self, line: &str) {
        self.print(vec![live::note(line)]);
    }

    /// Transcript lines: drawn into the scrollback, or kept for the CLI
    /// while the quiet host shows.
    fn print(&mut self, entries: Vec<Entry>) {
        if self.screen.is_quiet() {
            self.held_back.extend(entries);
            let over = self.held_back.len().saturating_sub(HELD_BACK);
            self.held_back.drain(..over);
            return;
        }
        self.redraw(&entries);
    }

    /// One frame: `entries` into the scrollback, the live region drawn
    /// again under them.
    fn redraw(&mut self, entries: &[Entry]) {
        let typed = self.editor.text();
        let placeholder = live::placeholder(self.session.as_ref());
        let Ui {
            screen,
            session,
            editor,
            menu,
            asking,
            offer,
            shown,
            ..
        } = self;
        let live = match shown {
            Face::Cli | Face::Stopping { .. } => Some(live::live(
                session.as_ref(),
                live::Composing {
                    typed: &typed,
                    cursor: editor.cursor(),
                    placeholder: &placeholder,
                    offer,
                },
                menu.as_ref(),
                *asking,
            )),
            Face::QuietHost | Face::Opening | Face::Headless | Face::Gone(_) => None,
        };
        let written = screen.frame(entries, live, super::screen::size().0);
        self.drawn(written);
    }

    /// The quiet host: the address, the pairing code with the key minted
    /// for a serve beyond this machine beside it, at most one transient
    /// line, and nothing else. The alternate screen is the one place the
    /// minted key is drawn (§8-11).
    fn quiet(&mut self) {
        let code = self.pairing.borrow_and_update().clone();
        let transient = self.transient.as_deref().map(str::trim);
        let written = self.screen.quiet(&QuietLines {
            url: &self.terminal.url,
            code: &code,
            key: self.terminal.token.as_deref(),
            transient,
        });
        self.drawn(written);
    }

    /// A terminal nobody can write to is a terminal that is gone, which
    /// the lifecycle hears once.
    fn drawn(&self, written: Written) {
        match written {
            Written::Shown => {}
            Written::Lost => self.inside.tell(Event::TerminalLost),
        }
    }
}

/// A `Say` for a line console: each line through a writer thread of its
/// own, so no worker waits on a pipe nobody reads.
pub(crate) fn lines_to_stdout() -> Say {
    let (to, from) = std::sync::mpsc::sync_channel::<String>(UI_DEPTH);
    std::thread::spawn(move || {
        let mut out = std::io::stdout();
        for line in from {
            if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
                return;
            }
        }
    });
    Arc::new(move |line: String| drop(to.try_send(line)))
}

mod input;
mod keys;
mod live;
mod menu;
mod reporting;

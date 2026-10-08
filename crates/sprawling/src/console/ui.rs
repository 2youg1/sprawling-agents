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
//! The point a reader most often gets wrong: a typed Ctrl+C is a key
//! here, not a signal. The city answers it once per session with a hint
//! and otherwise leaves it to the terminal, which copies a selection.

use std::io::Write;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender};
use std::time::Duration;

use crossterm::event::{KeyEvent, KeyEventKind};

use super::cli::{Say, Session};
use super::editor::Editor;
use super::lifecycle::{Deadline, Event, Face, Handoff};
use super::screen::{Screen, Written};
use super::terminal::{Inside, Terminal};
use wire::CloseMode;

/// How many lines may wait for this thread before the next is dropped.
pub(crate) const UI_DEPTH: usize = 256;

/// How often the thread looks at the lifecycle's face when nothing
/// arrives on its channel.
const FACE_LOOK: Duration = Duration::from_millis(100);

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
    read_keys(to.clone());
    read_records(to.clone(), watching);
    let say = sayer(&to, Show::Line);
    let writer = std::thread::spawn(move || {
        let mut ui = Ui::new(terminal, inside, say);
        ui.run(&from, face);
    });
    (writer, notice)
}

fn read_keys(to: SyncSender<Show>) {
    std::thread::spawn(move || {
        loop {
            let show = match crossterm::event::read() {
                Ok(crossterm::event::Event::Key(key)) if key.kind != KeyEventKind::Release => {
                    Show::Key(key)
                }
                Ok(crossterm::event::Event::Resize(..)) => Show::Resized,
                Ok(_) => continue,
                Err(_) => Show::Lost,
            };
            let lost = matches!(show, Show::Lost);
            if to.send(show).is_err() || lost {
                return;
            }
        }
    });
}

fn read_records(
    to: SyncSender<Show>,
    mut watching: tokio::sync::broadcast::Receiver<wire::Committed>,
) {
    use tokio::sync::broadcast::error::RecvError;
    std::thread::spawn(move || {
        loop {
            let show = match watching.blocking_recv() {
                Ok(committed) => Show::Record(committed),
                Err(RecvError::Lagged(missed)) => Show::Line(format!(
                    "  {missed} records were not shown here; the WebUI has them"
                )),
                Err(RecvError::Closed) => return,
            };
            drop(to.try_send(show));
        }
    });
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
    /// `/quit` with runs going, waiting for Enter, n or Esc.
    asking: bool,
    hinted: bool,
    /// The verbs Tab offered, while their menu is showing.
    menu: bool,
    transient: Option<String>,
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
        Ui {
            terminal,
            inside,
            say,
            session,
            editor: Editor::default(),
            shown: Face::Opening,
            screen: Screen::default(),
            asking: false,
            hinted: false,
            menu: false,
            transient: None,
        }
    }

    fn run(&mut self, from: &Receiver<Show>, mut face: tokio::sync::watch::Receiver<Face>) {
        let taken = self.screen.take();
        self.drawn(taken);
        self.header();
        loop {
            let now = *face.borrow_and_update();
            if now != self.shown {
                self.turn_to(now);
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

    fn header(&mut self) {
        let room = self
            .session
            .as_ref()
            .map_or(kernel::consts_policy::HALL_MAYOR, |s| s.room.as_str())
            .to_owned();
        let line = format!(
            "sprawling  {}  {room}  {}   /web opens the page",
            self.terminal.city, self.terminal.bind
        );
        self.line(&line);
    }

    /// Draws the face the lifecycle moved to.
    fn turn_to(&mut self, next: Face) {
        self.shown = next;
        match next {
            Face::QuietHost => self.quiet(),
            Face::Cli => {
                self.transient = None;
                let left = self.screen.leave_quiet();
                self.drawn(left);
                self.draw_input();
            }
            Face::Stopping {
                deadline: Deadline::Armed,
                ..
            } => self.screen.cut(),
            Face::Stopping { mode, .. } => {
                let progress = match mode {
                    CloseMode::Drain => {
                        "  sprawling is closing; the runs under way finish first (/quit again stops them now)"
                    }
                    CloseMode::Interrupt => {
                        "  sprawling is closing; the runs under way stop at their next safe point"
                    }
                };
                self.notice(progress.to_owned());
            }
            Face::Gone(handoff) => {
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
            Show::Record(committed) => {
                let lines = match self.session.as_mut() {
                    Some(session) => {
                        let room = session.room.clone();
                        session.seen.read(committed.record(), &room)
                    }
                    None => Vec::new(),
                };
                for line in lines {
                    self.line(&line);
                }
            }
            Show::Resized => {
                if self.shown == Face::QuietHost {
                    self.quiet();
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

    /// One line above the prompt, the prompt drawn again below it.
    fn line(&mut self, line: &str) {
        if self.screen.is_quiet() {
            return;
        }
        let editor = match self.shown {
            Face::QuietHost => None,
            Face::Cli | Face::Stopping { .. } => Some(&self.editor),
            Face::Opening | Face::Headless | Face::Gone(_) => None,
        };
        let written = self.screen.above(line, editor);
        self.drawn(written);
    }

    /// The quiet host: the address, the pairing code, at most one
    /// transient line, and nothing else.
    fn quiet(&mut self) {
        let mut lines = vec![self.terminal.url.clone()];
        if let Some(code) = &self.terminal.pairing {
            lines.push(format!("pairing code  {code}"));
        }
        if let Some(transient) = &self.transient {
            lines.push(transient.trim().to_owned());
        }
        let written = self.screen.quiet(&lines);
        self.drawn(written);
    }

    fn erase_input(&mut self) {
        let written = self.screen.erase_input();
        self.drawn(written);
    }

    fn draw_input(&mut self) {
        let written = self.screen.draw_input(&self.editor);
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

mod keys;

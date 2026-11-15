// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender};
use std::time::Duration;

use crate::scene::Entry;
use crossterm::event::KeyEvent;

use super::cli::{Say, Session};
use super::editor::Editor;
use super::lifecycle::{Event, Face, Handoff, INTERRUPT_GRACE, Sinks};
use super::local_time;
use super::mei::MeiEvent as ExternalMeiEvent;
use super::program_status::{self, Held, Reporting};
use super::screen::{QuietLines, Screen, Written};
use super::terminal::{Inside, Terminal};
use menu::Menu;
use wire::CloseMode;

pub(crate) const UI_DEPTH: usize = 256;

const FACE_LOOK: Duration = Duration::from_millis(100);

const HELD_BACK: usize = 512;

pub(super) const CALLS_SHOWN: usize = 3;

pub(super) const CTRL_C_HINT: &str = "  复制请先选中文字；关闭城市用 /quit";

pub(crate) enum Show {
    Line(String),
    Notice(String),
    Key(KeyEvent),
    Paste(String),
    Record(wire::Committed),
    Mei(ExternalMeiEvent),
    Resized,
    Lost,
}

fn sayer(to: &SyncSender<Show>, wrap: fn(String) -> Show) -> Say {
    let to = to.clone();
    Arc::new(move |line: String| drop(to.try_send(wrap(line))))
}

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

struct Ui {
    terminal: Terminal,
    inside: Inside,
    say: Say,
    session: Option<Session>,
    editor: Editor,
    shown: Face,
    screen: Screen,
    asking: Option<u64>,
    hinted: bool,
    menu: Option<Menu>,
    transient: Option<String>,
    pairing: tokio::sync::watch::Receiver<String>,
    reporting: Reporting,
    held: Held,
    offer: String,
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
            Show::Mei(_) => {}
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

    fn notice(&mut self, line: String) {
        if self.screen.is_quiet() {
            self.transient = Some(line);
            self.quiet();
        } else {
            self.line(&line);
        }
    }

    fn line(&mut self, line: &str) {
        self.print(vec![live::note(line)]);
    }

    fn print(&mut self, entries: Vec<Entry>) {
        if self.screen.is_quiet() {
            self.held_back.extend(entries);
            let over = self.held_back.len().saturating_sub(HELD_BACK);
            self.held_back.drain(..over);
            return;
        }
        self.redraw(&entries);
    }

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
        let written = screen.frame(entries, live, super::screen::size());
        self.drawn(written);
    }

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

    fn drawn(&self, written: Written) {
        match written {
            Written::Shown => {}
            Written::Lost => self.inside.tell(Event::TerminalLost),
        }
    }
}

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
pub(crate) mod mei;
mod menu;
mod reporting;

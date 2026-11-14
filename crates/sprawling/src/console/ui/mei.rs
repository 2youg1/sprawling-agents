// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Mei's one terminal writer, reusing the editor, screen and OSC 7501
//! (`crates/sprawling/spec/Console.lean` §8-174, D107 and D108).

use std::io::BufReader;
use std::process::ExitCode;
use std::sync::mpsc::{Receiver, SyncSender};

use accounting::Clock as _;
use console_ffi::scene::{Calling, Composer, Entry, Live, Working};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kernel::TimeMs;

use super::super::editor::Editor;
use super::super::local_time;
use super::super::mei::{Mei, MeiEvent, MeiProcess};
use super::super::program_status::{self, Held, Reporting};
use super::super::screen::{Screen, Written};
use super::{Show, UI_DEPTH};
use super::{input, keys, live as console_live};
use crate::assembly::SystemClock;

/// How many replay lines one batch draws before pending keys, exits and a
/// lost terminal take effect again; a long trace stays interruptible.
const REPLAY_BATCH: usize = 64;

/// Opens the Mei writer. A replay file uses the same messages and screen as
/// a real process; no second display path exists.
pub(crate) fn run(replay: Option<&str>) -> ExitCode {
    let replay = match replay {
        Some(path) => match std::fs::File::open(path) {
            Ok(file) => Some(BufReader::new(file)),
            Err(err) => {
                eprintln!(
                    "{}",
                    super::super::mei::AdapterFailure {
                        code: "E_MEI_CONFIG",
                        action: "open the Mei replay file",
                        rule: "the replay path must name a readable Mei JSONL file",
                        recovery: "pass the file saved from a live run, for example `sprawling mei --replay run.jsonl`",
                        subject: err.to_string(),
                        path: path.to_owned(),
                        value: path.to_owned(),
                    }
                    .render()
                );
                return ExitCode::FAILURE;
            }
        },
        None => None,
    };
    let (to, from) = std::sync::mpsc::sync_channel(UI_DEPTH);
    input::read_keys(to.clone());
    let mut ui = Ui::new(to, replay);
    ui.run(&from)
}

struct Ui {
    to: SyncSender<Show>,
    editor: Editor,
    screen: Screen,
    seen: Mei,
    process: Option<MeiProcess>,
    replay: Option<BufReader<std::fs::File>>,
    replay_done: bool,
    reporting: Reporting,
    held: Held,
    lost: bool,
}

impl Ui {
    fn new(to: SyncSender<Show>, replay: Option<BufReader<std::fs::File>>) -> Ui {
        let model = std::env::var_os("HARNESS_MODEL")
            .map(|model| model.to_string_lossy().into_owned())
            .unwrap_or_else(|| "HARNESS_MODEL is not set".to_owned());
        Ui {
            to,
            editor: Editor::default(),
            screen: Screen::default(),
            seen: Mei::new(model),
            process: None,
            replay_done: replay.is_none(),
            replay,
            reporting: Reporting::from_environment(),
            held: Held::OPENING,
            lost: false,
        }
    }

    fn run(&mut self, from: &Receiver<Show>) -> ExitCode {
        if self.screen.take() == Written::Lost {
            return self.give_back(from, ExitCode::FAILURE);
        }
        self.print(vec![Entry::Banner {
            city: "Archetype:Mei".to_owned(),
            address: gateway().trim_start_matches("http://").to_owned(),
            url: gateway(),
        }]);
        loop {
            if self.lost {
                return self.give_back(from, ExitCode::FAILURE);
            }
            self.advance_replay();
            match from.try_recv() {
                Ok(show) => {
                    if let Some(code) = self.show(show) {
                        return self.give_back(from, code);
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    if self.replay_done {
                        match from.recv() {
                            Ok(show) => {
                                if let Some(code) = self.show(show) {
                                    return self.give_back(from, code);
                                }
                            }
                            Err(_) => return self.give_back(from, ExitCode::SUCCESS),
                        }
                    }
                    // A replay still draining loops around: the next batch
                    // prints without waiting for a key.
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return self.give_back(from, ExitCode::SUCCESS);
                }
            }
        }
    }

    /// Draws the next bounded batch of replay lines. Esc, a lost terminal
    /// and a running task take effect between batches, and the caller keeps
    /// calling without waiting for keys until the trace is drained.
    fn advance_replay(&mut self) {
        for _ in 0..REPLAY_BATCH {
            if self.replay_done || self.process.is_some() || self.lost {
                return;
            }
            let Some(replay) = self.replay.as_mut() else {
                return;
            };
            let mut line = String::new();
            match std::io::BufRead::read_line(replay, &mut line) {
                Ok(0) => self.replay_done = true,
                Ok(_) => {
                    let at = self.now();
                    let entries = self.seen.line(line.trim_end_matches(['\r', '\n']), at);
                    self.print(entries);
                }
                Err(err) => {
                    self.replay_done = true;
                    self.print(vec![console_live::note(&format!("read Mei replay: {err}"))]);
                }
            }
        }
    }

    fn show(&mut self, show: Show) -> Option<ExitCode> {
        match show {
            Show::Line(line) | Show::Notice(line) => self.print(vec![console_live::note(&line)]),
            Show::Key(key) => return self.key(key),
            Show::Paste(pasted) => {
                self.editor.paste(&pasted);
                self.redraw(&[]);
            }
            Show::Mei(event) => self.mei(event),
            Show::Resized => self.redraw(&[]),
            Show::Lost => {
                self.lost = true;
                self.stop();
            }
            Show::Record(_) => {}
        }
        None
    }

    fn key(&mut self, key: KeyEvent) -> Option<ExitCode> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return None;
        }
        if key.code == KeyCode::Enter && !key.modifiers.contains(KeyModifiers::SHIFT) {
            self.enter();
        } else if key.code == KeyCode::Esc {
            return self.escape();
        } else {
            let alt = key.modifiers.contains(KeyModifiers::ALT);
            if let Some(edit) = keys::editing(key, alt) {
                self.editor.apply(edit);
                self.redraw(&[]);
            }
        }
        None
    }

    fn escape(&mut self) -> Option<ExitCode> {
        if !self.editor.is_empty() {
            self.editor.apply(super::super::editor::Edit::Clear);
            self.redraw(&[]);
            return None;
        }
        if self.process.is_some() {
            self.stop();
            return None;
        }
        Some(ExitCode::SUCCESS)
    }

    fn enter(&mut self) {
        let Some(typed) = self.editor.enter() else {
            return self.redraw(&[]);
        };
        if typed.trim().is_empty() {
            return;
        }
        let at = self.now();
        self.print(vec![Entry::You {
            at: local_time::local(at),
            said: typed.clone(),
        }]);
        if self.process.is_some() {
            self.print(vec![console_live::note(
                "one Mei task is already running; Esc stops it",
            )]);
            return;
        }
        if self.replay.is_some() {
            self.print(vec![console_live::note("replay is read-only")]);
            return;
        }
        self.seen.begin(at);
        self.status(program_status::Event::Counted(self.seen.counts()));
        match MeiProcess::spawn(&typed, self.to.clone()) {
            Ok(process) => self.process = Some(process),
            Err(error) => {
                let at = self.now();
                let entries = self.seen.finished(Err(error), at);
                self.print(entries);
                self.ended();
            }
        }
    }

    fn mei(&mut self, event: MeiEvent) {
        match event {
            MeiEvent::Line(line) => {
                let at = self.now();
                let entries = self.seen.line(&line, at);
                self.print(entries);
            }
            MeiEvent::Finished { result, cancelled } => {
                if let Some(process) = self.process.as_mut()
                    && let Err(error) = process.reap()
                {
                    self.print(vec![console_live::note(&error)]);
                }
                self.process = None;
                let at = self.now();
                let entries = if cancelled {
                    self.seen.cancelled(at)
                } else {
                    self.seen.finished(result, at)
                };
                self.print(entries);
                self.ended();
            }
        }
    }

    fn stop(&mut self) {
        if let Some(process) = self.process.as_mut()
            && let Err(error) = process.request_stop()
        {
            self.print(vec![console_live::note(&error)]);
        }
    }

    fn ended(&mut self) {
        self.status(program_status::Event::Ended(self.seen.rest()));
        self.status(program_status::Event::Counted(self.seen.counts()));
    }

    fn status(&mut self, event: program_status::Event) {
        if self.reporting == Reporting::Off {
            return;
        }
        let (held, report) = program_status::step(self.held, event);
        self.held = held;
        if let Some(report) = report {
            let written = self.screen.report(report);
            self.drawn(written);
        }
    }

    fn now(&mut self) -> TimeMs {
        match SystemClock.now() {
            Ok(at) => at,
            Err(err) => {
                self.print(vec![console_live::note(&format!(
                    "sample the console clock: {err}"
                ))]);
                TimeMs::new(0)
            }
        }
    }

    fn print(&mut self, entries: Vec<Entry>) {
        self.redraw(&entries);
    }

    fn redraw(&mut self, entries: &[Entry]) {
        let typed = self.editor.text();
        let working = self.seen.since().map(|since| Working {
            resident: "Mei",
            since: local_time::local(since),
        });
        let calling = if self.seen.calling() {
            vec![Calling {
                name: "python",
                subject: "工具波执行中",
            }]
        } else {
            Vec::new()
        };
        let live = Live {
            waiting: None,
            working,
            calling,
            asking: None,
            composer: Composer {
                typed: &typed,
                cursor: self.editor.cursor(),
                placeholder: if self.replay.is_some() {
                    "replay; Esc exits"
                } else {
                    "to Mei…"
                },
                room: "Mei",
                offer: self.seen.model(),
            },
            menu: None,
        };
        let written = self
            .screen
            .frame(entries, Some(live), super::super::screen::size());
        self.drawn(written);
    }

    fn drawn(&mut self, written: Written) {
        if written == Written::Lost {
            self.lost = true;
        }
    }

    fn give_back(&mut self, from: &Receiver<Show>, code: ExitCode) -> ExitCode {
        self.stop();
        while self.process.is_some() {
            match from.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(Show::Mei(event @ MeiEvent::Finished { .. })) => self.mei(event),
                Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if self.process.as_ref().is_some_and(MeiProcess::is_finished) {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        if let Some(process) = self.process.as_mut()
            && let Err(error) = process.reap()
        {
            self.print(vec![console_live::note(&error)]);
        }
        self.process = None;
        self.status(program_status::Event::Closed);
        let written = self.screen.give_back();
        self.drawn(written);
        code
    }
}

fn gateway() -> String {
    std::env::var_os("ANTHROPIC_BASE_URL")
        .map(|base| base.to_string_lossy().into_owned())
        .unwrap_or_else(|| "http://127.0.0.1:3425".to_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn replay_is_drained_before_the_writer_waits_for_a_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trace.jsonl");
        std::fs::write(
            &path,
            concat!(
                "{\"role\":\"assistant\",\"content\":\"first\"}\n",
                "{\"role\":\"assistant\",\"content\":\"second\"}\n",
                "{\"role\":\"assistant\",\"content\":\"third\"}\n"
            ),
        )
        .unwrap();
        let replay = BufReader::new(std::fs::File::open(path).unwrap());
        let (to, _from) = std::sync::mpsc::sync_channel(UI_DEPTH);
        let mut ui = Ui::new(to, Some(replay));
        ui.screen.cut();
        ui.advance_replay();
        assert!(ui.replay_done);
    }

    #[test]
    fn replay_advances_in_bounded_batches_so_escape_stays_live() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trace.jsonl");
        let mut text = String::new();
        for index in 0..REPLAY_BATCH + 4 {
            text.push_str(&format!(
                "{{\"role\":\"assistant\",\"content\":\"line {index}\"}}\n"
            ));
        }
        std::fs::write(&path, text).unwrap();
        let replay = BufReader::new(std::fs::File::open(path).unwrap());
        let (to, _from) = std::sync::mpsc::sync_channel(UI_DEPTH);
        let mut ui = Ui::new(to, Some(replay));
        ui.screen.cut();
        ui.advance_replay();
        assert!(!ui.replay_done, "one batch must not pin the whole trace");
        ui.advance_replay();
        assert!(ui.replay_done);
    }

    #[test]
    fn shift_enter_inserts_a_newline_instead_of_submitting() {
        let (to, _from) = std::sync::mpsc::sync_channel(UI_DEPTH);
        let mut ui = Ui::new(to, None);
        ui.screen.cut();
        ui.key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        ui.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        assert_eq!(ui.editor.text(), "x\n");
        assert!(ui.process.is_none());
    }
}

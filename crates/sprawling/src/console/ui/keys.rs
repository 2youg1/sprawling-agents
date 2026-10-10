// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What each key does on each face (`crates/sprawling/spec/Console.lean`
//! §8-11): no Ctrl chord is bound; Esc closes the menu, else clears the
//! line, else stops the room's run; a trailing backslash then Enter is a
//! newline; `y` and `n` on an empty line answer the waiting request; Tab
//! completes a slash verb or opens the menu of those that fit, and Enter
//! in the menu takes the one under the cursor.

use console_ffi::scene::Entry;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::cli::Next;
use super::super::editor::Edit;
use super::super::lifecycle::{Asker, Event, Face};
use super::super::local_time;
use super::menu::{self, Fit, Menu, Then};
use super::{CTRL_C_HINT, Ui};
use wire::CloseMode;

impl Ui {
    pub(super) fn key(&mut self, key: KeyEvent) {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        if control {
            if key.code == KeyCode::Char('c') && !self.hinted {
                self.hinted = true;
                self.notice(CTRL_C_HINT.to_owned());
            }
            return;
        }
        match self.shown {
            Face::QuietHost => {
                if key.code == KeyCode::Esc {
                    self.inside.tell(Event::Back);
                } else if key.code == KeyCode::Enter {
                    self.open_web();
                }
            }
            // A close begun from the quiet host stays there, showing its
            // progress; the keys that could still escalate it are signals.
            Face::Stopping { .. } if self.screen.is_quiet() => {}
            Face::Cli | Face::Stopping { .. } => self.typing(key, alt),
            Face::Opening | Face::Headless | Face::Gone(_) => {}
        }
    }

    /// Text pasted whole, put into the line as it was copied.
    pub(super) fn paste(&mut self, pasted: &str) {
        if let Face::Cli | Face::Stopping { .. } = self.shown {
            self.editor.paste(pasted);
            self.refilter();
            self.redraw(&[]);
        }
    }

    fn typing(&mut self, key: KeyEvent, alt: bool) {
        if let Some(going) = self.asking {
            return self.answer_quit(key, going);
        }
        if self.menu.is_some() && self.in_menu(key) {
            return;
        }
        let edit = match (key.code, alt) {
            (KeyCode::Enter, _) if key.modifiers.contains(KeyModifiers::SHIFT) => Edit::Newline,
            (KeyCode::Enter, _) => return self.enter(),
            (KeyCode::Esc, _) => return self.escape(),
            (KeyCode::Tab, _) => return self.complete(),
            (KeyCode::Char(answer @ ('y' | 'n')), false) if self.editor.is_empty() => {
                let verdict = if answer == 'y' {
                    kernel::Ruling::Allow
                } else {
                    kernel::Ruling::Deny
                };
                let answered = match self.session.as_mut() {
                    Some(session) => session.answer_waiting(&self.inside, verdict, &self.say),
                    None => false,
                };
                if answered {
                    return self.redraw(&[]);
                }
                Edit::Insert(answer)
            }
            (KeyCode::Char(ch), false) => Edit::Insert(ch),
            (KeyCode::Backspace, true) => Edit::DeleteWord,
            (KeyCode::Left, true) => Edit::WordLeft,
            (KeyCode::Right, true) => Edit::WordRight,
            (KeyCode::Backspace, false) => Edit::Backspace,
            (KeyCode::Delete, _) => Edit::Delete,
            (KeyCode::Left, false) => Edit::Left,
            (KeyCode::Right, false) => Edit::Right,
            (KeyCode::Home, _) => Edit::Home,
            (KeyCode::End, _) => Edit::End,
            (KeyCode::Up, _) => Edit::Older,
            (KeyCode::Down, _) => Edit::Newer,
            _ => return,
        };
        self.editor.apply(edit);
        self.refilter();
        self.redraw(&[]);
    }

    /// `/quit` asked how the runs under way end: Enter waits for them, n
    /// stops them now, Esc keeps serving.
    fn answer_quit(&mut self, key: KeyEvent, going: u64) {
        let mode = if key.code == KeyCode::Enter {
            Some(CloseMode::Drain)
        } else if key.code == KeyCode::Char('n') {
            Some(CloseMode::Interrupt)
        } else if key.code == KeyCode::Esc {
            None
        } else {
            return;
        };
        self.asking = None;
        match mode {
            Some(mode) => {
                self.redraw(&[]);
                self.inside.tell(Event::Quit(Asker::Console, mode));
            }
            None => self.line(&format!("  the city keeps serving; {going} runs go on")),
        }
    }

    /// A key while the menu is open: Tab and Down move on, Shift+Tab and
    /// Up move back, Enter takes the fit under the cursor, Esc closes it.
    /// Any other key edits the line, and the menu follows what it says.
    fn in_menu(&mut self, key: KeyEvent) -> bool {
        let Some(open) = self.menu.as_mut() else {
            return false;
        };
        if matches!(key.code, KeyCode::Tab | KeyCode::Down) {
            open.next();
        } else if matches!(key.code, KeyCode::BackTab | KeyCode::Up) {
            open.previous();
        } else if key.code == KeyCode::Esc {
            self.menu = None;
        } else if key.code == KeyCode::Enter {
            self.choose();
            return true;
        } else {
            return false;
        }
        self.redraw(&[]);
        true
    }

    /// Enter in the menu: the fit under the cursor goes on the line, and
    /// a line that is complete is carried at once.
    fn choose(&mut self) {
        let Some(fit) = self.menu.take().and_then(|open| open.chosen().cloned()) else {
            return;
        };
        self.editor.set(&fit.line);
        match fit.then {
            Then::Carry => self.enter(),
            Then::Wait => self.redraw(&[]),
        }
    }

    /// Esc with the menu closed: clear the line, else stop the run
    /// working in this room.
    fn escape(&mut self) {
        if !self.editor.is_empty() {
            self.editor.apply(Edit::Clear);
            return self.redraw(&[]);
        }
        if let Some(session) = self.session.as_mut() {
            session.interrupt(&self.inside, &self.say);
        }
    }

    /// Tab: the one fit replaces the line, or the menu opens on the first
    /// of several.
    fn complete(&mut self) {
        let mut fits = self.fits();
        if fits.len() == 1 {
            if let Some(Fit { line, .. }) = fits.pop() {
                self.editor.set(&line);
            }
        } else {
            self.menu = Menu::of(fits);
        }
        self.redraw(&[]);
    }

    /// While the menu is open, it follows the line as it is edited, and
    /// closes when nothing fits any more.
    fn refilter(&mut self) {
        if self.menu.is_some() {
            self.menu = Menu::of(self.fits());
        }
    }

    /// What fits the line as typed: slash verbs while the verb is being
    /// typed, the offered arguments after `/model` or `/effort`.
    fn fits(&self) -> Vec<Fit> {
        let typed = self.editor.text();
        if !typed.starts_with('/') {
            return Vec::new();
        }
        match typed.split_once(' ') {
            None => menu::verbs(&typed),
            Some((spelled, begun)) if !begun.contains(char::is_whitespace) => {
                let Some(verb) =
                    super::super::language::slash_verbs().find(|verb| verb.spelling() == spelled)
                else {
                    return Vec::new();
                };
                let offered = match self.session.as_ref() {
                    Some(session) => session.arguments(&self.inside, verb),
                    None => Vec::new(),
                };
                menu::arguments(spelled, begun, &offered)
            }
            Some(_) => Vec::new(),
        }
    }

    pub(super) fn enter(&mut self) {
        let Some(typed) = self.editor.enter() else {
            return self.redraw(&[]);
        };
        self.menu = None;
        let at = (self.inside.clock)().ok().and_then(local_time::local);
        self.print(vec![Entry::You {
            at,
            said: typed.clone(),
        }]);
        if let Face::Stopping { .. } = self.shown {
            if typed.trim() == wire::Slash::Quit.spelling() {
                self.inside
                    .tell(Event::Quit(Asker::Console, CloseMode::Interrupt));
            } else {
                self.line("  the city is closing; /quit again stops the runs now");
            }
            return;
        }
        let next = match self.session.as_mut() {
            Some(session) => session.carry(&self.terminal, &self.inside, &typed, &self.say),
            None => Next::Stay,
        };
        if let Some(session) = self.session.as_ref() {
            self.offer = session.offer(&self.inside);
        }
        match next {
            Next::Stay => self.redraw(&[]),
            Next::Web => {
                self.open_web();
                self.inside.tell(Event::Web);
            }
            Next::Quit => self
                .inside
                .tell(Event::Quit(Asker::Console, CloseMode::Drain)),
            Next::AskBeforeQuit(going) => {
                self.asking = Some(going);
                self.redraw(&[]);
            }
        }
    }

    /// Opens the page already paired: an open code through a file only
    /// this account reads, never a key on a command line. A browser that
    /// does not open is said once; the address is on the screen anyway.
    fn open_web(&mut self) {
        if let Err(unopened) = crate::firstrun::open_paired(&self.terminal.url, &self.inside.door) {
            self.notice(format!("  {}: {}", unopened.action(), unopened.recovery()));
        }
    }
}

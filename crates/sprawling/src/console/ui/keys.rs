// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What each key does on each face (`crates/sprawling/spec/Console.lean`
//! §8-11): no Ctrl chord is bound; Esc closes the menu, else clears the
//! line, else stops the room's run; a trailing backslash then Enter is a
//! newline; `y` and `n` on an empty line answer the waiting request.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::cli::Next;
use super::super::editor::Edit;
use super::super::lifecycle::{Asker, Event, Face};
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
                    drop(crate::firstrun::open_in_browser(&self.terminal.url));
                }
            }
            // A close begun from the quiet host stays there, showing its
            // progress; the keys that could still escalate it are signals.
            Face::Stopping { .. } if self.screen.is_quiet() => {}
            Face::Cli | Face::Stopping { .. } => self.typing(key, alt),
            Face::Opening | Face::Headless | Face::Gone(_) => {}
        }
    }

    fn typing(&mut self, key: KeyEvent, alt: bool) {
        if self.asking {
            let answer = if key.code == KeyCode::Enter {
                Some(Some(CloseMode::Drain))
            } else if key.code == KeyCode::Char('n') {
                Some(Some(CloseMode::Interrupt))
            } else if key.code == KeyCode::Esc {
                Some(None)
            } else {
                None
            };
            if let Some(mode) = answer {
                self.asking = false;
                match mode {
                    Some(mode) => self.inside.tell(Event::Quit(Asker::Console, mode)),
                    None => self.line("  the city keeps serving"),
                }
            }
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
                    return;
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
        self.erase_input();
        self.editor.apply(edit);
        self.draw_input();
    }

    /// Esc, in order: close the menu, else clear the line, else stop the
    /// run working in this room.
    fn escape(&mut self) {
        if self.menu {
            self.menu = false;
            return;
        }
        if !self.editor.is_empty() {
            self.erase_input();
            self.editor.apply(Edit::Clear);
            self.draw_input();
            return;
        }
        if let Some(session) = self.session.as_mut() {
            session.interrupt(&self.inside, &self.say);
        }
    }

    /// Tab: a slash verb completed, or the argument after `/model` or
    /// `/effort`; several that fit are listed instead.
    fn complete(&mut self) {
        let typed = self.editor.text();
        if !typed.starts_with('/') {
            return;
        }
        match typed.split_once(' ') {
            None => {
                let fits: Vec<(String, &str)> = super::super::language::slash_verbs()
                    .filter(|verb| verb.spelling().starts_with(&typed))
                    .map(|verb| {
                        let spaced = if verb.takes().is_empty() { "" } else { " " };
                        (format!("{}{spaced}", verb.spelling()), verb.spelling())
                    })
                    .collect();
                self.offer(&fits);
            }
            Some((spelled, begun)) if !begun.contains(char::is_whitespace) => {
                let Some(verb) =
                    super::super::language::slash_verbs().find(|verb| verb.spelling() == spelled)
                else {
                    return;
                };
                let offered = match self.session.as_ref() {
                    Some(session) => session.arguments(&self.inside, verb),
                    None => Vec::new(),
                };
                let fits: Vec<(String, &str)> = offered
                    .iter()
                    .filter(|argument| argument.starts_with(begun))
                    .map(|argument| (format!("{spelled} {argument}"), argument.as_str()))
                    .collect();
                self.offer(&fits);
            }
            Some(_) => {}
        }
    }

    /// One fit replaces the line; several are listed as a menu Esc closes.
    /// Each fit is the line it would leave and the word the menu shows.
    fn offer(&mut self, fits: &[(String, &str)]) {
        match fits {
            [] => {}
            [(line, _)] => {
                self.erase_input();
                self.editor.set(line);
                self.draw_input();
            }
            many => {
                let listed: Vec<&str> = many.iter().map(|(_, shown)| *shown).collect();
                self.menu = true;
                self.line(&format!("  {}", listed.join("   ")));
            }
        }
    }

    fn enter(&mut self) {
        let Some(typed) = self.editor.enter() else {
            self.erase_input();
            self.draw_input();
            return;
        };
        self.menu = false;
        let submitted = self.screen.submitted();
        self.drawn(submitted);
        if let Face::Stopping { .. } = self.shown {
            if typed.trim() == wire::Slash::Quit.spelling() {
                self.inside
                    .tell(Event::Quit(Asker::Console, CloseMode::Interrupt));
            } else {
                self.line("  the city is closing; /quit again stops the runs now");
            }
            self.draw_input();
            return;
        }
        let next = match self.session.as_mut() {
            Some(session) => session.carry(&self.terminal, &self.inside, &typed, &self.say),
            None => Next::Stay,
        };
        match next {
            Next::Stay => {}
            Next::Web => {
                drop(crate::firstrun::open_in_browser(&self.terminal.url));
                self.inside.tell(Event::Web);
            }
            Next::Quit => self
                .inside
                .tell(Event::Quit(Asker::Console, CloseMode::Drain)),
            Next::AskBeforeQuit(going) => {
                self.asking = true;
                self.line(&format!(
                    "  {going} runs are going   Enter wait for them   n stop them now   Esc keep serving"
                ));
            }
        }
        self.draw_input();
    }
}

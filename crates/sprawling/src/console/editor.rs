// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The line a person is typing in the CLI: its text, its cursor, its
//! words, the session's history, and what Enter means for it
//! (`crates/sprawling/spec/Console.lean` §8-11).
//!
//! Pure: no terminal, no clock. `bin::console::ui` turns keys into
//! [`Edit`]s and hands the text and the cursor to the renderer, which
//! lays them out, so the whole of line editing is judged here without a
//! terminal.
//!
//! The point a reader most often gets wrong: a newline is a trailing
//! backslash then Enter on every platform, because Shift+Enter reaches a
//! program only where the terminal reports it, and no Ctrl chord is
//! bound at all. A paste, where the terminal hands it over whole, keeps
//! its line breaks instead of sending each line.

/// One change to the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edit {
    Insert(char),
    /// Shift+Enter, where the terminal tells it from Enter.
    Newline,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    /// Alt+Left.
    WordLeft,
    /// Alt+Right.
    WordRight,
    /// Alt+Backspace.
    DeleteWord,
    /// The line before, from this session's history.
    Older,
    /// The line after, back to what was being typed.
    Newer,
    Clear,
}

/// The line being typed, and the lines typed before it.
#[derive(Debug, Default)]
pub(crate) struct Editor {
    text: Vec<char>,
    cursor: usize,
    history: Vec<String>,
    /// Which history line is shown, counted back from the newest.
    recalled: Option<usize>,
    /// What was being typed before the history was walked.
    draft: Vec<char>,
}

impl Editor {
    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub(crate) fn text(&self) -> String {
        self.text.iter().collect()
    }

    /// Where the cursor stands, as a count of characters before it.
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    /// Text pasted whole, inserted at the cursor with its line breaks.
    pub(crate) fn paste(&mut self, pasted: &str) {
        for ch in pasted.chars().filter(|ch| *ch != '\r') {
            self.insert(ch);
        }
    }

    /// Replaces the line, cursor at its end; Tab completion uses this.
    pub(crate) fn set(&mut self, text: &str) {
        self.text = text.chars().collect();
        self.cursor = self.text.len();
    }

    pub(crate) fn apply(&mut self, edit: Edit) {
        match edit {
            Edit::Insert(ch) => self.insert(ch),
            Edit::Newline => self.insert('\n'),
            Edit::Backspace => {
                if let Some(before) = self.cursor.checked_sub(1) {
                    self.text.remove(before);
                    self.cursor = before;
                }
            }
            Edit::Delete => {
                if self.cursor < self.text.len() {
                    self.text.remove(self.cursor);
                }
            }
            Edit::Left => self.cursor = self.cursor.saturating_sub(1),
            Edit::Right => self.cursor = self.cursor.saturating_add(1).min(self.text.len()),
            Edit::Home => self.cursor = 0,
            Edit::End => self.cursor = self.text.len(),
            Edit::WordLeft => self.cursor = self.word_start(),
            Edit::WordRight => self.cursor = self.word_end(),
            Edit::DeleteWord => {
                let start = self.word_start();
                self.text.drain(start..self.cursor);
                self.cursor = start;
            }
            Edit::Older => self.recall(self.recalled.map_or(0, |n| n.saturating_add(1))),
            Edit::Newer => match self.recalled {
                None => {}
                Some(0) => {
                    self.recalled = None;
                    self.text = std::mem::take(&mut self.draft);
                    self.cursor = self.text.len();
                }
                Some(n) => self.recall(n.saturating_sub(1)),
            },
            Edit::Clear => {
                self.text.clear();
                self.cursor = 0;
                self.recalled = None;
            }
        }
    }

    /// Enter: a trailing backslash becomes a newline and the line goes
    /// on; otherwise the line is handed back and remembered.
    pub(crate) fn enter(&mut self) -> Option<String> {
        if self.cursor == self.text.len() && self.text.last() == Some(&'\\') {
            self.text.pop();
            self.cursor = self.text.len();
            self.insert('\n');
            return None;
        }
        let line = self.text();
        self.apply(Edit::Clear);
        if !line.trim().is_empty() && self.history.last() != Some(&line) {
            self.history.push(line.clone());
        }
        Some(line)
    }

    fn insert(&mut self, ch: char) {
        self.text.insert(self.cursor, ch);
        self.cursor = self.cursor.saturating_add(1);
    }

    fn recall(&mut self, back: usize) {
        let Some(index) = self.history.len().checked_sub(back.saturating_add(1)) else {
            return;
        };
        let Some(line) = self.history.get(index) else {
            return;
        };
        if self.recalled.is_none() {
            self.draft = std::mem::take(&mut self.text);
        }
        self.text = line.chars().collect();
        self.cursor = self.text.len();
        self.recalled = Some(back);
    }

    /// The start of the word before the cursor: spaces skipped first,
    /// then everything up to the previous space.
    fn word_start(&self) -> usize {
        let before = self.text.get(..self.cursor).unwrap_or_default();
        let trimmed = before.iter().rposition(|c| !c.is_whitespace());
        trimmed.map_or(0, |last| {
            before
                .get(..last)
                .unwrap_or_default()
                .iter()
                .rposition(|c| c.is_whitespace())
                .map_or(0, |space| space.saturating_add(1))
        })
    }

    /// The end of the word after the cursor.
    fn word_end(&self) -> usize {
        let after = self.text.get(self.cursor..).unwrap_or_default();
        let first = after.iter().position(|c| !c.is_whitespace());
        first.map_or(self.text.len(), |first| {
            let rest = after.get(first..).unwrap_or_default();
            let length = rest
                .iter()
                .position(|c| c.is_whitespace())
                .unwrap_or(rest.len());
            self.cursor.saturating_add(first).saturating_add(length)
        })
    }
}

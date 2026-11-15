// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edit {
    Insert(char),
    Newline,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    WordLeft,
    WordRight,
    DeleteWord,
    Older,
    Newer,
    Clear,
}

#[derive(Debug, Default)]
pub(crate) struct Editor {
    text: Vec<char>,
    cursor: usize,
    history: Vec<String>,
    recalled: Option<usize>,
    draft: Vec<char>,
}

impl Editor {
    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub(crate) fn text(&self) -> String {
        self.text.iter().collect()
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn paste(&mut self, pasted: &str) {
        for ch in pasted.chars().filter(|ch| *ch != '\r') {
            self.insert(ch);
        }
    }

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

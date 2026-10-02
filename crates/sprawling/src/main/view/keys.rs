// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which key does what in the person's face of `sprawling view`
//! (`crates/sprawling/spec/Main.lean` §8-117). The run board of the WebUI uses the same
//! keys, so this table is the one place they are decided.

/// A key press, already read off the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Tab,
    Esc,
    PageUp,
    PageDown,
    Home,
    End,
    /// Ctrl-C, which raw mode delivers as a key instead of a signal.
    Interrupt,
}

/// Everything a person can ask the viewer to do. None of it changes the
/// city: the viewer is read-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Up,
    Down,
    PageUp,
    PageDown,
    First,
    Last,
    Collapse,
    Expand,
    SwitchLens,
    OpenDetail,
    CloseDetail,
    Quit,
}

/// The action `key` asks for; `None` for a key that asks for nothing.
pub(super) fn action_for(key: Key) -> Option<Action> {
    match key {
        Key::Up | Key::Char('k') => Some(Action::Up),
        Key::Down | Key::Char('j') => Some(Action::Down),
        Key::PageUp => Some(Action::PageUp),
        Key::PageDown => Some(Action::PageDown),
        Key::Home | Key::Char('g') => Some(Action::First),
        Key::End | Key::Char('G') => Some(Action::Last),
        Key::Left | Key::Char('h') => Some(Action::Collapse),
        Key::Right | Key::Char('l') => Some(Action::Expand),
        Key::Tab => Some(Action::SwitchLens),
        Key::Enter => Some(Action::OpenDetail),
        Key::Esc => Some(Action::CloseDetail),
        Key::Char('q') | Key::Interrupt => Some(Action::Quit),
        Key::Char(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Key, action_for};

    #[test]
    fn every_key_the_viewer_reads_names_its_action() {
        let table = [
            (Key::Char('k'), Some(Action::Up)),
            (Key::Up, Some(Action::Up)),
            (Key::Char('j'), Some(Action::Down)),
            (Key::Down, Some(Action::Down)),
            (Key::PageUp, Some(Action::PageUp)),
            (Key::PageDown, Some(Action::PageDown)),
            (Key::Char('g'), Some(Action::First)),
            (Key::Home, Some(Action::First)),
            (Key::Char('G'), Some(Action::Last)),
            (Key::End, Some(Action::Last)),
            (Key::Char('h'), Some(Action::Collapse)),
            (Key::Left, Some(Action::Collapse)),
            (Key::Char('l'), Some(Action::Expand)),
            (Key::Right, Some(Action::Expand)),
            (Key::Tab, Some(Action::SwitchLens)),
            (Key::Enter, Some(Action::OpenDetail)),
            (Key::Esc, Some(Action::CloseDetail)),
            (Key::Char('q'), Some(Action::Quit)),
            (Key::Interrupt, Some(Action::Quit)),
            (Key::Char('x'), None),
        ];
        for (key, action) in table {
            assert_eq!(action_for(key), action, "{key:?}");
        }
    }
}

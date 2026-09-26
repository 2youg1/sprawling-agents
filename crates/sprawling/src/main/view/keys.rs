// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which key does what in the person's face of `sprawling view`
//! (sprawling-SPEC.md 8-91). The run board of the WebUI uses the same
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
        Key::Char('q') => Some(Action::Quit),
        Key::Char(_) => None,
    }
}

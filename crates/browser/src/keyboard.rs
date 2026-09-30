// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Pressing a key on a page: the keys a run may press, the modifiers it
//! may hold, and the BiDi key-source frame that presses one
//! (browser-SPEC.md section 19-11).
//!
//! Every name is spelled once, in the one `spelling` of its variant: the
//! argument reader and the frame builder both read it, so a key cannot
//! be accepted under one name and sent under another. The names are the
//! DOM `KeyboardEvent.key` values, because those are what a page's own
//! key table is written in, and the code points are the WebDriver ones a
//! driver reads.

use std::collections::BTreeSet;

use kernel::{AxCode, AxError};
use serde_json::{Value, json};

use crate::port::Frame;
use crate::session::{ContextId, Session};

/// A key that has a name rather than a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamedKey {
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
}

impl NamedKey {
    /// Every named key, in the order a refusal lists them.
    const ALL: [NamedKey; 13] = [
        NamedKey::Enter,
        NamedKey::Tab,
        NamedKey::Escape,
        NamedKey::Backspace,
        NamedKey::Delete,
        NamedKey::ArrowUp,
        NamedKey::ArrowDown,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::Home,
        NamedKey::End,
        NamedKey::PageUp,
        NamedKey::PageDown,
    ];

    /// The DOM name of this key and the WebDriver code point that
    /// presses it.
    fn spelling(self) -> (&'static str, char) {
        match self {
            NamedKey::Enter => ("Enter", '\u{E007}'),
            NamedKey::Tab => ("Tab", '\u{E004}'),
            NamedKey::Escape => ("Escape", '\u{E00C}'),
            NamedKey::Backspace => ("Backspace", '\u{E003}'),
            NamedKey::Delete => ("Delete", '\u{E017}'),
            NamedKey::ArrowUp => ("ArrowUp", '\u{E013}'),
            NamedKey::ArrowDown => ("ArrowDown", '\u{E015}'),
            NamedKey::ArrowLeft => ("ArrowLeft", '\u{E012}'),
            NamedKey::ArrowRight => ("ArrowRight", '\u{E014}'),
            NamedKey::Home => ("Home", '\u{E011}'),
            NamedKey::End => ("End", '\u{E010}'),
            NamedKey::PageUp => ("PageUp", '\u{E00E}'),
            NamedKey::PageDown => ("PageDown", '\u{E00F}'),
        }
    }
}

/// One key: a named key, or the character a character key types. A
/// capital letter is its own character; `Shift` is a separate modifier,
/// and nothing here turns one into the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Named(NamedKey),
    Char(char),
}

impl Key {
    /// Reads a key by its DOM name, or as the one character it types.
    ///
    /// # Errors
    /// Refuses a name that is neither a named key nor one character.
    pub fn parse(name: &str) -> Result<Key, AxError> {
        if let Some(named) = NamedKey::ALL
            .into_iter()
            .find(|key| key.spelling().0 == name)
        {
            return Ok(Key::Named(named));
        }
        let mut chars = name.chars();
        match (chars.next(), chars.next()) {
            (Some(only), None) => Ok(Key::Char(only)),
            (None, _) | (Some(_), Some(_)) => {
                Err(
                    AxError::failure(AxCode::InvalidArgs, "read a key to press", name.to_owned())
                        .with_recovery(format!(
                            "pass one character, or one of {}",
                            NamedKey::ALL.map(|key| key.spelling().0).join(", ")
                        )),
                )
            }
        }
    }

    /// What the driver is sent for this key.
    fn value(self) -> String {
        match self {
            Key::Named(named) => named.spelling().1.to_string(),
            Key::Char(typed) => typed.to_string(),
        }
    }
}

/// A key held down while another is pressed. The declaration order is
/// the order they go down, so one press is one byte sequence however the
/// caller listed them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Modifier {
    Control,
    Shift,
    Alt,
    Meta,
}

impl Modifier {
    /// Every modifier, in the order a refusal lists them.
    const ALL: [Modifier; 4] = [
        Modifier::Control,
        Modifier::Shift,
        Modifier::Alt,
        Modifier::Meta,
    ];

    /// The DOM name of this modifier and the WebDriver code point that
    /// holds it.
    fn spelling(self) -> (&'static str, char) {
        match self {
            Modifier::Control => ("Control", '\u{E009}'),
            Modifier::Shift => ("Shift", '\u{E008}'),
            Modifier::Alt => ("Alt", '\u{E00A}'),
            Modifier::Meta => ("Meta", '\u{E03D}'),
        }
    }

    /// Reads a modifier by its DOM name.
    ///
    /// # Errors
    /// Refuses a name that is not one of the four.
    pub fn parse(name: &str) -> Result<Modifier, AxError> {
        Modifier::ALL
            .into_iter()
            .find(|modifier| modifier.spelling().0 == name)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "read a modifier to hold",
                    name.to_owned(),
                )
                .with_recovery(format!(
                    "pass any of {}",
                    Modifier::ALL
                        .map(|modifier| modifier.spelling().0)
                        .join(", ")
                ))
            })
    }
}

/// One key press on whatever the page has focused: each modifier down in
/// declaration order, the key down and up, the modifiers up in reverse.
///
/// # Errors
/// Propagates the frame's own refusal.
pub fn key_frame(
    session: &mut Session,
    context: &ContextId,
    key: Key,
    modifiers: &BTreeSet<Modifier>,
) -> Result<Frame, AxError> {
    let held: Vec<String> = modifiers
        .iter()
        .map(|modifier| modifier.spelling().1.to_string())
        .collect();
    let pressed = key.value();
    let actions: Vec<Value> = held
        .iter()
        .map(|value| json!({ "type": "keyDown", "value": value }))
        .chain([
            json!({ "type": "keyDown", "value": pressed }),
            json!({ "type": "keyUp", "value": pressed }),
        ])
        .chain(
            held.iter()
                .map(|value| json!({ "type": "keyUp", "value": value })),
        )
        .collect();
    session.frame(
        "input.performActions",
        json!({
            "context": context.as_str(),
            "actions": [{ "type": "key", "id": "keyboard", "actions": actions }],
        }),
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn steps(frame: &Frame) -> Vec<(String, String)> {
        frame.params()["actions"][0]["actions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|step| {
                (
                    step["type"].as_str().unwrap().to_owned(),
                    step["value"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn a_press_holds_its_modifiers_around_the_key_in_one_order() {
        let mut session = Session::new();
        let frame = key_frame(
            &mut session,
            &ContextId::parse("c1").unwrap(),
            Key::Char('k'),
            &BTreeSet::from([Modifier::Shift, Modifier::Control]),
        )
        .unwrap();
        let pair = |kind: &str, value: &str| (kind.to_owned(), value.to_owned());
        assert_eq!(
            (
                frame.method().to_owned(),
                frame.params()["actions"][0]["type"].clone(),
                steps(&frame)
            ),
            (
                "input.performActions".to_owned(),
                json!("key"),
                vec![
                    pair("keyDown", "\u{E009}"),
                    pair("keyDown", "\u{E008}"),
                    pair("keyDown", "k"),
                    pair("keyUp", "k"),
                    pair("keyUp", "\u{E008}"),
                    pair("keyUp", "\u{E009}"),
                ]
            )
        );
    }

    #[test]
    fn a_press_reads_dom_names_and_one_character_and_refuses_the_rest() {
        assert_eq!(
            (
                Key::parse("ArrowDown").unwrap(),
                Key::parse("/").unwrap(),
                Modifier::parse("Meta").unwrap(),
            ),
            (
                Key::Named(NamedKey::ArrowDown),
                Key::Char('/'),
                Modifier::Meta
            )
        );
        for refused in ["Down", "", "Ctrl"] {
            let key = Key::parse(refused).err().map(|err| *err.code());
            let modifier = Modifier::parse(refused).err().map(|err| *err.code());
            assert_eq!(
                (key, modifier),
                (Some(AxCode::InvalidArgs), Some(AxCode::InvalidArgs)),
                "{refused:?}"
            );
        }
    }
}

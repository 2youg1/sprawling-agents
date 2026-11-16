// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use kernel::{AxCode, AxError};
use serde_json::{Value, json};

use crate::port::Frame;
use crate::session::{ContextId, Session};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Named(NamedKey),
    Char(char),
}

impl Key {
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

    fn value(self) -> String {
        match self {
            Key::Named(named) => named.spelling().1.to_string(),
            Key::Char(typed) => typed.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Modifier {
    Control,
    Shift,
    Alt,
    Meta,
}

impl Modifier {
    const ALL: [Modifier; 4] = [
        Modifier::Control,
        Modifier::Shift,
        Modifier::Alt,
        Modifier::Meta,
    ];

    fn spelling(self) -> (&'static str, char) {
        match self {
            Modifier::Control => ("Control", '\u{E009}'),
            Modifier::Shift => ("Shift", '\u{E008}'),
            Modifier::Alt => ("Alt", '\u{E00A}'),
            Modifier::Meta => ("Meta", '\u{E03D}'),
        }
    }

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
                .rev()
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

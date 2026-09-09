// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The key names this server accepts, and the virtual-key code each one
//! is.
//!
//! Editing this table is editing behaviour: a name that is not in it
//! cannot be pressed, and that is deliberate. A key name is a thing a
//! model writes from memory, so an unknown one is refused **with the
//! whole table in the refusal** rather than guessed at — pressing the
//! wrong key on somebody's desktop is not a mistake a retry undoes.
//!
//! One printable character is not a key here: it is what `type` types.
//! Two ways to enter the letter `a` would be two behaviours to keep
//! agreeing, and only one of them handles a keyboard layout correctly.
//!
//! The codes are Microsoft's own
//! (<https://learn.microsoft.com/windows/win32/inputdev/virtual-key-codes>),
//! written as their documented hexadecimal so a reader can check a row
//! against that page without arithmetic.

use crate::refusal::{Refusal, RefusalCode};

/// One modifier a call may hold down. Exhaustive, and its own type
/// rather than a string, so an action that holds a modifier this
/// machine has no key for cannot be spelled past the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Win,
}

impl Modifier {
    /// # Errors
    /// Refuses a modifier outside the four the tool schema offers.
    pub(crate) fn parse(named: &str) -> Result<Modifier, Refusal> {
        match named {
            "ctrl" => Ok(Modifier::Ctrl),
            "alt" => Ok(Modifier::Alt),
            "shift" => Ok(Modifier::Shift),
            "win" => Ok(Modifier::Win),
            other => Err(Refusal::new(
                RefusalCode::InvalidArgs,
                "act on a window",
                format!("`{other}` is not a modifier this server holds down"),
                "use `ctrl`, `alt`, `shift` or `win`",
            )),
        }
    }

    pub(crate) fn code(self) -> u16 {
        match self {
            Modifier::Ctrl => 0x11,
            Modifier::Alt => 0x12,
            Modifier::Shift => 0x10,
            Modifier::Win => 0x5B,
        }
    }
}

/// Every key name, with the virtual-key code it is. Matched without
/// regard to ASCII case, because `Enter` and `enter` are one key and a
/// refusal over capitalisation teaches nothing.
const TABLE: [(&str, u16); 36] = [
    ("backspace", 0x08),
    ("tab", 0x09),
    ("enter", 0x0D),
    ("escape", 0x1B),
    ("space", 0x20),
    ("pageup", 0x21),
    ("pagedown", 0x22),
    ("end", 0x23),
    ("home", 0x24),
    ("left", 0x25),
    ("up", 0x26),
    ("right", 0x27),
    ("down", 0x28),
    ("insert", 0x2D),
    ("delete", 0x2E),
    ("f1", 0x70),
    ("f2", 0x71),
    ("f3", 0x72),
    ("f4", 0x73),
    ("f5", 0x74),
    ("f6", 0x75),
    ("f7", 0x76),
    ("f8", 0x77),
    ("f9", 0x78),
    ("f10", 0x79),
    ("f11", 0x7A),
    ("f12", 0x7B),
    ("f13", 0x7C),
    ("f14", 0x7D),
    ("f15", 0x7E),
    ("f16", 0x7F),
    ("f17", 0x80),
    ("f18", 0x81),
    ("f19", 0x82),
    ("f20", 0x83),
    ("f21", 0x84),
];

/// The spellings this table also answers to, each pointing at the row
/// above rather than carrying a second code. One key, one code; the
/// aliases exist because a model writes `esc` as often as `escape`.
const ALIASES: [(&str, &str); 5] = [
    ("esc", "escape"),
    ("return", "enter"),
    ("del", "delete"),
    ("ins", "insert"),
    ("pgup", "pageup"),
];

/// The virtual-key code `named` is.
///
/// # Errors
/// Refuses a name this table does not hold, listing every name it does.
/// A model that guessed once will otherwise guess again.
pub(crate) fn code(named: &str) -> Result<u16, Refusal> {
    let lowered = named.to_ascii_lowercase();
    let canonical = ALIASES
        .iter()
        .find(|(spelling, _row)| *spelling == lowered)
        .map_or(lowered.as_str(), |(_spelling, row)| *row);
    TABLE
        .iter()
        .find(|(name, _code)| *name == canonical)
        .map(|(_name, code)| *code)
        .ok_or_else(|| {
            Refusal::new(
                RefusalCode::InvalidArgs,
                "act on a window",
                format!("`{named}` is not a key this server presses"),
                &format!(
                    "press one of: {}; one printable character is `type`, not `key`",
                    accepted()
                ),
            )
        })
}

/// Every name this table answers to, for the refusal to carry.
fn accepted() -> String {
    TABLE
        .iter()
        .map(|(name, _code)| *name)
        .collect::<Vec<&str>>()
        .join(", ")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn every_name_in_the_table_presses_something_and_no_two_share_a_code() {
        let mut seen: Vec<u16> = Vec::new();
        for (name, expected) in TABLE {
            assert_eq!(code(name).unwrap(), expected, "{name}");
            assert!(!seen.contains(&expected), "{name} repeats a code");
            seen.push(expected);
        }
        assert_eq!(seen.len(), TABLE.len());
    }

    /// Capitalisation is not a decision a caller has to get right, and
    /// the aliases point at a row rather than carrying a second code.
    #[test]
    fn case_and_the_short_spellings_reach_the_same_key() {
        assert_eq!(code("Enter").unwrap(), code("enter").unwrap());
        assert_eq!(code("ENTER").unwrap(), code("return").unwrap());
        assert_eq!(code("Esc").unwrap(), code("escape").unwrap());
        assert_eq!(code("F5").unwrap(), 0x74);
        for (spelling, row) in ALIASES {
            assert_eq!(code(spelling).unwrap(), code(row).unwrap(), "{spelling}");
        }
    }

    /// The rule this module exists for: a key nobody listed is refused,
    /// and the refusal carries the list so the next call is answerable.
    #[test]
    fn a_key_this_table_does_not_hold_is_refused_with_the_whole_table() {
        for guess in ["Meta", "Cmd", "AnyKey", "f99", ""] {
            let refusal = code(guess).expect_err("an unknown key is never guessed at");
            let error = refusal.as_error();
            assert_eq!(error["data"]["code"], "E_INVALID_ARGS", "{guess}");
            let recovery = error["data"]["recovery"].as_str().unwrap();
            assert!(recovery.contains("enter"), "{guess}");
            assert!(recovery.contains("pagedown"), "{guess}");
            assert!(recovery.contains("`type`"), "{guess}");
        }
    }

    /// One printable character has one door, and it is not this one.
    #[test]
    fn a_single_letter_is_typed_rather_than_pressed() {
        assert!(code("a").is_err());
        assert!(code("7").is_err());
    }

    #[test]
    fn the_four_modifiers_parse_and_a_fifth_does_not() {
        for (spelling, expected) in [
            ("ctrl", Modifier::Ctrl),
            ("alt", Modifier::Alt),
            ("shift", Modifier::Shift),
            ("win", Modifier::Win),
        ] {
            assert_eq!(Modifier::parse(spelling).unwrap(), expected);
        }
        let refusal = Modifier::parse("meta").unwrap_err();
        assert!(
            refusal.as_error()["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("ctrl")
        );
        // Four modifiers, four distinct codes: two that shared one
        // would hold down the wrong key and nothing would say so.
        let mut codes: Vec<u16> = [
            Modifier::Ctrl,
            Modifier::Alt,
            Modifier::Shift,
            Modifier::Win,
        ]
        .into_iter()
        .map(Modifier::code)
        .collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 4);
    }
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use winsafe::co;

use crate::refusal::{Refusal, RefusalCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Win,
}

impl Modifier {
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

    pub(crate) fn code(self) -> co::VK {
        match self {
            Modifier::Ctrl => co::VK::CONTROL,
            Modifier::Alt => co::VK::MENU,
            Modifier::Shift => co::VK::SHIFT,
            Modifier::Win => co::VK::LWIN,
        }
    }
}

const TABLE: [(&str, co::VK); 36] = [
    ("backspace", co::VK::BACK),
    ("tab", co::VK::TAB),
    ("enter", co::VK::RETURN),
    ("escape", co::VK::ESCAPE),
    ("space", co::VK::SPACE),
    ("pageup", co::VK::PRIOR),
    ("pagedown", co::VK::NEXT),
    ("end", co::VK::END),
    ("home", co::VK::HOME),
    ("left", co::VK::LEFT),
    ("up", co::VK::UP),
    ("right", co::VK::RIGHT),
    ("down", co::VK::DOWN),
    ("insert", co::VK::INSERT),
    ("delete", co::VK::DELETE),
    ("f1", co::VK::F1),
    ("f2", co::VK::F2),
    ("f3", co::VK::F3),
    ("f4", co::VK::F4),
    ("f5", co::VK::F5),
    ("f6", co::VK::F6),
    ("f7", co::VK::F7),
    ("f8", co::VK::F8),
    ("f9", co::VK::F9),
    ("f10", co::VK::F10),
    ("f11", co::VK::F11),
    ("f12", co::VK::F12),
    ("f13", co::VK::F13),
    ("f14", co::VK::F14),
    ("f15", co::VK::F15),
    ("f16", co::VK::F16),
    ("f17", co::VK::F17),
    ("f18", co::VK::F18),
    ("f19", co::VK::F19),
    ("f20", co::VK::F20),
    ("f21", co::VK::F21),
];

const ALIASES: [(&str, &str); 5] = [
    ("esc", "escape"),
    ("return", "enter"),
    ("del", "delete"),
    ("ins", "insert"),
    ("pgup", "pageup"),
];

pub(crate) fn code(named: &str) -> Result<co::VK, Refusal> {
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
        let mut seen: Vec<co::VK> = Vec::new();
        for (name, expected) in TABLE {
            assert_eq!(code(name).unwrap(), expected, "{name}");
            assert!(!seen.contains(&expected), "{name} repeats a code");
            seen.push(expected);
        }
        assert_eq!(seen.len(), TABLE.len());
    }

    #[test]
    fn case_and_the_short_spellings_reach_the_same_key() {
        assert_eq!(code("Enter").unwrap(), code("enter").unwrap());
        assert_eq!(code("ENTER").unwrap(), code("return").unwrap());
        assert_eq!(code("Esc").unwrap(), code("escape").unwrap());
        assert_eq!(code("F5").unwrap(), co::VK::F5);
        for (spelling, row) in ALIASES {
            assert_eq!(code(spelling).unwrap(), code(row).unwrap(), "{spelling}");
        }
    }

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

        let mut codes: Vec<co::VK> = [
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

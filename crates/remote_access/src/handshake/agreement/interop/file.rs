// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One file of remote_access-SPEC.md §8-12: its spelling, one named
//! field of lowercase hex per line, and the two ways a test holds it,
//! byte for byte or by what it still proves.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use std::path::PathBuf;

/// A file with no randomness in it: written again under `GOLDEN_WRITE=1`,
/// and otherwise held byte for byte to what this build computes.
pub(super) fn known(name: &str, rebuilt: &Fields) -> Fields {
    if writing() {
        write(name, rebuilt);
    }
    let committed = committed(name);
    assert_eq!(
        committed,
        rebuilt.text(),
        "{name} is not what this build derives"
    );
    Fields::parse(&committed).unwrap()
}

/// A file with randomness in it: written again under `GOLDEN_WRITE=1` only
/// when the committed copy is missing or no longer holds.
pub(super) fn kept(
    name: &str,
    build: fn() -> Fields,
    holds: fn(&Fields) -> Result<(), String>,
) -> Fields {
    if writing() && read(name).and_then(|fields| holds(&fields)).is_err() {
        write(name, &build());
    }
    let fields = Fields::parse(&committed(name)).unwrap();
    assert_eq!(holds(&fields), Ok(()), "{name} no longer holds");
    fields
}

fn writing() -> bool {
    std::env::var("GOLDEN_WRITE").as_deref() == Ok("1")
}

pub(super) fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/fixtures/remote-handshake")
}

fn write(name: &str, fields: &Fields) {
    std::fs::create_dir_all(dir()).unwrap();
    std::fs::write(dir().join(name), fields.text()).unwrap();
}

fn committed(name: &str) -> String {
    let path = dir().join(name);
    assert!(
        path.is_file(),
        "{} is missing: run the vector and fixture tests once with GOLDEN_WRITE=1",
        path.display()
    );
    std::fs::read_to_string(path).unwrap()
}

pub(super) fn read(name: &str) -> Result<Fields, String> {
    std::fs::read_to_string(dir().join(name))
        .map_err(text)
        .and_then(|contents| Fields::parse(&contents))
}

pub(super) fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// One file of §8-12: named fields of bytes, in order.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Fields(Vec<(String, Vec<u8>)>);

impl Fields {
    pub(super) fn of<const N: usize>(names: [&str; N], values: [Vec<u8>; N]) -> Self {
        Self(names.into_iter().map(str::to_owned).zip(values).collect())
    }

    /// One line per field: the name, one space, lowercase hex, LF.
    fn text(&self) -> String {
        self.0
            .iter()
            .map(|(name, bytes)| {
                let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                format!("{name} {hex}\n")
            })
            .collect()
    }

    fn parse(contents: &str) -> Result<Self, String> {
        let body = contents
            .strip_suffix('\n')
            .ok_or("the last line has no LF")?;
        body.split('\n')
            .map(|line| {
                let (name, hex) = line
                    .split_once(' ')
                    .ok_or_else(|| format!("{line:?} is not a name and a value"))?;
                let named = !name.is_empty()
                    && name
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
                let spelled = hex.len() % 2 == 0
                    && hex
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
                if !(named && spelled) {
                    return Err(format!("{line:?} is not a name and lowercase hex"));
                }
                let bytes = (0..hex.len())
                    .step_by(2)
                    .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).map_err(text))
                    .collect::<Result<Vec<u8>, String>>()?;
                Ok((name.to_owned(), bytes))
            })
            .collect::<Result<Vec<_>, String>>()
            .map(Self)
    }

    /// Holds when the fields are exactly `names`, in that order.
    pub(super) fn named<const N: usize>(&self, names: [&str; N]) -> Result<(), String> {
        let found: Vec<&str> = self.0.iter().map(|(name, _)| name.as_str()).collect();
        if found == names {
            Ok(())
        } else {
            Err(format!("fields {found:?}, expected {names:?}"))
        }
    }

    pub(super) fn try_get(&self, name: &str) -> Result<&[u8], String> {
        self.0
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, bytes)| bytes.as_slice())
            .ok_or_else(|| format!("no field {name}"))
    }

    pub(super) fn get(&self, name: &str) -> &[u8] {
        self.try_get(name).unwrap()
    }
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Presence {

    Present { at: PathBuf, version: Version },

    Broken { at: PathBuf, fault: Fault },

    Absent(Absence),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Version {
    Said(String),

    Silent,

    Unreadable,

    Late,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Fault {

    WillNotStart(String),

    HalfWritten,

    Unreadable(String),

    DigestMismatch {
        expected: kernel::B3Hash,
        found: kernel::B3Hash,
    },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Absence {
    NotOnSearchPath,

    VariableNamesNothing {
        variable: &'static str,
        path: PathBuf,
    },
    NoComponent {
        dir: PathBuf,
    },
    NoHome,
    NotInThisBuild,

    NotInstalled,
}

impl Presence {

    pub(crate) fn usable(&self) -> bool {
        matches!(self, Presence::Present { .. })
    }

    pub(crate) fn at(&self) -> Option<&Path> {
        match self {
            Presence::Present { at, .. } | Presence::Broken { at, .. } => Some(at),
            Presence::Absent(_) => None,
        }
    }

    pub(crate) fn describe(&self) -> String {
        match self {
            Presence::Present { at, version } => {
                format!("present {} ({})", at.display(), version.describe())
            }
            Presence::Broken { at, fault } => {
                format!("broken at {}: {}", at.display(), fault.describe())
            }
            Presence::Absent(absence) => format!("absent: {}", absence.describe()),
        }
    }
}

impl Version {

    pub(crate) fn number(&self) -> String {
        let Version::Said(text) = self else {
            return self.describe();
        };
        text.split_whitespace()
            .find_map(dotted)
            .or_else(|| {
                text.split_whitespace()
                    .find(|token| token.chars().any(|glyph| glyph.is_ascii_digit()))
                    .map(|token| token.chars().take(BUILD_ID).collect())
            })
            .unwrap_or_else(|| text.clone())
    }

    pub(crate) fn describe(&self) -> String {
        match self {
            Version::Said(text) => text.clone(),
            Version::Silent => "no version".to_owned(),
            Version::Unreadable => "unreadable version".to_owned(),
            Version::Late => "no version within the deadline".to_owned(),
        }
    }
}

const BUILD_ID: usize = 12;

fn dotted(token: &str) -> Option<String> {
    let run: String = token
        .trim_start_matches('v')
        .chars()
        .take_while(|glyph| glyph.is_ascii_digit() || *glyph == '.')
        .collect();
    let numbered = run.split('.').filter(|part| !part.is_empty()).count();
    (numbered > 1 && run.starts_with(|glyph: char| glyph.is_ascii_digit()))
        .then(|| run.trim_end_matches('.').to_owned())
}

impl Fault {
    pub(crate) fn describe(&self) -> String {
        match self {
            Fault::WillNotStart(err) => format!("will not start: {err}"),
            Fault::HalfWritten => "half-written; delete it and install again".to_owned(),
            Fault::Unreadable(err) => err.clone(),
            Fault::DigestMismatch { expected, found } => {
                format!("pinned guest digest mismatch: expected {expected}, found {found}")
            }
        }
    }
}

impl Absence {
    pub(crate) fn describe(&self) -> String {
        match self {
            Absence::NotOnSearchPath => "not on the search path".to_owned(),
            Absence::VariableNamesNothing { variable, path } => {
                format!("{variable} names {}, which is not there", path.display())
            }
            Absence::NoComponent { dir } => format!("no component at {}", dir.display()),
            Absence::NoHome => accounting::home::NO_HOME.to_owned(),
            Absence::NotInThisBuild => "not in this build".to_owned(),
            Absence::NotInstalled => "not installed in the platform's font directories".to_owned(),
        }
    }
}

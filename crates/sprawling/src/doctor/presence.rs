// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine's answer about one item (sprawling-SPEC.md section
//! 8-47).
//!
//! Three states rather than two. A binary that is on the search path
//! and will not start, a component directory with no component in it,
//! a variable that names a file which is not there: each of these used
//! to be reported as "absent", which is the one word that tells a
//! person nothing about what to do. Here every answer carries its own
//! cause, and the wording of that cause lives in this file alone.

use std::path::{Path, PathBuf};

/// Whether the item is here, and in what condition.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Presence {
    /// Here, and it started when asked its version.
    Present { at: PathBuf, version: Version },
    /// Here, and unusable. Counts as missing for a verdict; counts as
    /// its own fault for a person.
    Broken { at: PathBuf, fault: Fault },
    /// Not here, in one of the ways of not being here.
    Absent(Absence),
}

/// What a present program said when asked its version. Only the first
/// arm carries text; the other three are facts about how it did not
/// answer, and none of them makes the program absent.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Version {
    Said(String),
    /// It started, wrote nothing, and exited.
    Silent,
    /// Its first line was not text.
    Unreadable,
    /// It had not written a line when the deadline passed.
    Late,
}

/// Why a thing that is here cannot be used.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Fault {
    /// The operating system refused to start it: permission, a file that
    /// is not a program, a program for another machine.
    WillNotStart(String),
    /// The component directory exists and the component file does not:
    /// an install that stopped half way.
    HalfWritten,
    /// The directory or file could not be read.
    Unreadable(String),
}

/// Which kind of not being here.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Absence {
    NotOnSearchPath,
    /// A person pointed at a file, and the file is not there. Never
    /// falls through to the component directory: a wrong pointer that
    /// silently lost would stay wrong forever.
    VariableNamesNothing {
        variable: &'static str,
        path: PathBuf,
    },
    NoComponent {
        dir: PathBuf,
    },
    NoHome,
    NotInThisBuild,
}

impl Presence {
    /// Whether a verdict may count this item as here.
    pub(crate) fn usable(&self) -> bool {
        matches!(self, Presence::Present { .. })
    }

    /// Where it is, when it is somewhere.
    pub(crate) fn at(&self) -> Option<&Path> {
        match self {
            Presence::Present { at, .. } | Presence::Broken { at, .. } => Some(at),
            Presence::Absent(_) => None,
        }
    }

    /// The one sentence that says what this machine answered.
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
    /// The version alone, out of whatever the program printed.
    ///
    /// A tool's first line is often a banner - `ffmpeg version
    /// N-125649-g8d3942 Copyright (c) 2000-2026 the FFmpeg
    /// developers` - and a report that pastes the whole line pushes
    /// every other column off the screen. The dotted run is what a
    /// person compares against a requirement; a build that has none
    /// falls back to its first token carrying a digit, cut short,
    /// because that token is the build identifier.
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
            Version::Silent => "said nothing".to_owned(),
            Version::Unreadable => "unreadable version".to_owned(),
            Version::Late => "no version within the deadline".to_owned(),
        }
    }
}

/// How much of a build identifier the report keeps when a tool prints
/// no dotted version.
const BUILD_ID: usize = 12;

/// The dotted decimal run inside one token, when it has one: `133.0.3`
/// out of `133.0.3`, and nothing out of `2000-2026`.
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
            Absence::NoHome => "neither USERPROFILE nor HOME is set".to_owned(),
            Absence::NotInThisBuild => "not in this build".to_owned(),
        }
    }
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one building's capability bits call for, and which of it this
//! machine lacks (sprawling-SPEC.md section 8-48).
//!
//! No I/O: the bits arrive read, the findings arrive examined, and what
//! comes out is the list of things a person would otherwise learn from
//! a refusal in the middle of a run.

use kernel::Address;

use super::family::{GECKO, WEBKIT};
use super::table::{CHROMEDRIVER, MSEDGEDRIVER, SHELL, SPRAWLING_DESKTOP};
use super::{Finding, Presence};

/// The bits a building declares. Two live in `BUILDING.md`, one in the
/// building's frozen `CONFIG.toml`; they are one value here because a
/// building is judged whole.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Bits {
    pub(crate) browser: bool,
    pub(crate) desktop: bool,
    pub(crate) shell: bool,
}

/// One thing a building can ask this machine for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Capability {
    Browser,
    Desktop,
    Shell,
}

impl Capability {
    pub(crate) const ALL: [Capability; 3] =
        [Capability::Browser, Capability::Desktop, Capability::Shell];

    /// The items of which any one satisfies this capability, first
    /// choice first. The Gecko family leads because it needs no driver;
    /// a Chromium browser is reached only through the driver beside it,
    /// so the driver rather than the browser is what is tried here.
    pub(crate) fn any_of(self) -> &'static [&'static str] {
        match self {
            Capability::Browser => &[GECKO, CHROMEDRIVER, MSEDGEDRIVER, WEBKIT],
            Capability::Desktop => &[SPRAWLING_DESKTOP],
            Capability::Shell => &[SHELL],
        }
    }

    /// The line in the building's own files that asked for it.
    pub(crate) fn declared_as(self) -> &'static str {
        match self {
            Capability::Browser => "browser: true",
            Capability::Desktop => "desktop: true",
            Capability::Shell => "sandbox.shell = true",
        }
    }

    fn asked_by(self, bits: &Bits) -> bool {
        match self {
            Capability::Browser => bits.browser,
            Capability::Desktop => bits.desktop,
            Capability::Shell => bits.shell,
        }
    }
}

/// One capability a building asked for and this machine cannot give,
/// with every item that was tried and what each one answered.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Lack {
    pub(crate) building: Address,
    pub(crate) capability: Capability,
    pub(crate) tried: Vec<(&'static str, Presence)>,
}

/// Everything this building asked for that this machine lacks, in the
/// order of `Capability::ALL`.
pub(crate) fn lacks(building: &Address, bits: &Bits, findings: &[Finding]) -> Vec<Lack> {
    Capability::ALL
        .into_iter()
        .filter(|capability| capability.asked_by(bits))
        .filter_map(|capability| {
            let tried: Vec<(&'static str, Presence)> = capability
                .any_of()
                .iter()
                .map(|item| (*item, answer_for(findings, item)))
                .collect();
            if tried.iter().any(|(_, presence)| presence.usable()) {
                return None;
            }
            Some(Lack {
                building: building.clone(),
                capability,
                tried,
            })
        })
        .collect()
}

/// The one line a person reads about one lack: the building, the line
/// that asked, and each road with why it is closed.
pub(crate) fn lack_line(lack: &Lack) -> String {
    let roads: Vec<String> = lack
        .tried
        .iter()
        .map(|(item, presence)| match presence {
            Presence::Absent(absence) => format!("no {item} ({})", absence.describe()),
            other => format!("{item} {}", other.describe()),
        })
        .collect();
    format!(
        "  {}: {}, and this machine has {}",
        lack.building.as_str(),
        lack.capability.declared_as(),
        roads.join(" and ")
    )
}

/// What the findings say about one item; an item the table does not
/// carry is reported as not on the search path rather than skipped, so
/// a capability can never be satisfied by an item nobody looked for.
fn answer_for(findings: &[Finding], item: &str) -> Presence {
    findings
        .iter()
        .find(|finding| finding.requirement.name == item)
        .map_or(
            Presence::Absent(super::Absence::NotOnSearchPath),
            |finding| finding.presence.clone(),
        )
}

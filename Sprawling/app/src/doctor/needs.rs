// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Address;

use super::family::{GECKO, WEBKIT};
use super::table::{CHROMEDRIVER, MSEDGEDRIVER, PWSH, SHELL};
use super::{Finding, Presence};

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Bits {
    pub(crate) browser: bool,
    pub(crate) shell: bool,

    pub(crate) interpreter: kernel::Interpreter,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Capability {
    Browser,

    Shell,

    Pwsh,
}

impl Capability {
    pub(crate) const ALL: [Capability; 3] =
        [Capability::Browser, Capability::Shell, Capability::Pwsh];

    pub(crate) fn any_of(self) -> &'static [&'static str] {
        match self {
            Capability::Browser => &[GECKO, CHROMEDRIVER, MSEDGEDRIVER, WEBKIT],
            Capability::Shell => &[SHELL],
            Capability::Pwsh => &[PWSH],
        }
    }

    pub(crate) fn declared_as(self) -> &'static str {
        match self {
            Capability::Browser => "browser = true",
            Capability::Shell => "sandbox.shell = true",
            Capability::Pwsh => "sandbox.interpreter = \"pwsh\"",
        }
    }

    fn asked_by(self, bits: &Bits) -> bool {
        match self {
            Capability::Browser => bits.browser,
            Capability::Shell => bits.shell && bits.interpreter == kernel::Interpreter::System,
            Capability::Pwsh => bits.shell && bits.interpreter == kernel::Interpreter::Pwsh,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Lack {
    pub(crate) building: Address,
    pub(crate) capability: Capability,
    pub(crate) tried: Vec<(&'static str, Presence)>,
}

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

pub(crate) fn lack_line(lack: &Lack) -> String {
    let roads: Vec<String> = lack
        .tried
        .iter()
        .map(|(item, presence)| match presence {
            Presence::Absent(absence) => format!("no {item} ({})", absence.describe()),
            other @ (Presence::Present { .. } | Presence::Broken { .. }) => {
                format!("{item} {}", other.describe())
            }
        })
        .collect();
    format!(
        "  {}: {}, and this machine has {}",
        lack.building.as_str(),
        lack.capability.declared_as(),
        roads.join(" and ")
    )
}

fn answer_for(findings: &[Finding], item: &str) -> Presence {
    findings
        .iter()
        .find(|finding| finding.requirement.name == item)
        .map_or(
            Presence::Absent(super::Absence::NotOnSearchPath),
            |finding| finding.presence.clone(),
        )
}

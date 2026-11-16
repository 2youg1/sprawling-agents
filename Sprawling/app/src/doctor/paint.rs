// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsString;

use super::family::member_at;
use super::{Detection, Finding, Need, Presence, Requirement, Tier};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Ink {
    Colour,
    Plain,
}

impl Ink {
    pub(crate) fn or_plain(self, no_color: Option<OsString>) -> Ink {
        match no_color {
            Some(_) => Ink::Plain,
            None => self,
        }
    }

    fn wrap(self, colour: &str, text: &str) -> String {
        match self {
            Ink::Plain => text.to_owned(),
            Ink::Colour => format!("\u{1b}[{colour}m{text}\u{1b}[0m"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Status {
    Present,

    Missing,

    Broken,

    Optional,
}

impl Status {
    pub(crate) fn of(finding: &Finding) -> Status {
        match (&finding.presence, finding.requirement.need) {
            (Presence::Present { .. }, _) => Status::Present,
            (Presence::Broken { .. }, _) => Status::Broken,
            (Presence::Absent(_), Need::Optional) => Status::Optional,
            (Presence::Absent(_), Need::Required | Need::OneOf(_)) => Status::Missing,
        }
    }

    pub(crate) fn word(self) -> &'static str {
        match self {
            Status::Present => "present",
            Status::Missing => "missing",
            Status::Broken => "broken",
            Status::Optional => "optional",
        }
    }

    fn colour(self) -> &'static str {
        match self {
            Status::Present => "32",
            Status::Missing => "31",
            Status::Broken => "33",
            Status::Optional => "2",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Part {
    Required,
    Recommended,
}

impl Part {
    pub(crate) const ALL: [Part; 2] = [Part::Required, Part::Recommended];

    pub(crate) fn of(requirement: &Requirement) -> Part {
        match (requirement.tier, requirement.need) {
            (Tier::Use, Need::Required | Need::OneOf(_)) => Part::Required,
            (Tier::Use, Need::Optional) | (Tier::Develop, _) => Part::Recommended,
        }
    }

    pub(crate) fn heading(self) -> &'static str {
        match self {
            Part::Required => "  required - to run a city here",
            Part::Recommended => "  recommended - what each one adds",
        }
    }
}

const NAME: usize = 18;

const STATUS: usize = 9;

pub(crate) fn row(finding: &Finding, ink: Ink) -> String {
    let status = Status::of(finding);
    let name = format!("{:<NAME$}", finding.requirement.name);
    let word = format!("{:<STATUS$}", status.word());
    format!(
        "  {name} {} {}",
        ink.wrap(status.colour(), &word),
        detail(finding, status)
    )
}

fn detail(finding: &Finding, status: Status) -> String {
    match (&finding.presence, status) {
        (Presence::Present { at, version }, _) => {
            format!("{} at {}{}", version.number(), at.display(), brand(finding))
        }
        (Presence::Broken { at, fault }, _) => format!("{} at {}", fault.describe(), at.display()),
        (Presence::Absent(absence), Status::Optional) => format!(
            "{} - it enables {}",
            absence.describe(),
            finding.requirement.enables
        ),
        (Presence::Absent(absence), Status::Present | Status::Missing | Status::Broken) => {
            absence.describe()
        }
    }
}

fn brand(finding: &Finding) -> String {
    let Detection::Family(family) = &finding.requirement.detect else {
        return String::new();
    };
    let Some(at) = finding.presence.at() else {
        return String::new();
    };
    member_at(*family, at).map_or_else(String::new, |member| {
        format!(" ({}{})", member.name, member.confidence.caveat())
    })
}

pub(crate) fn summary(findings: &[Finding], ink: Ink) -> Vec<String> {
    let mut lines: Vec<String> = Part::ALL
        .into_iter()
        .map(|part| {
            let counted = count(findings, part);
            let word = match part {
                Part::Required => "required",
                Part::Recommended => "recommended",
            };
            let colour = if counted.here == counted.wanted {
                "32"
            } else {
                "31"
            };
            format!(
                "  {word:<13} {} ready",
                ink.wrap(colour, &format!("{} / {}", counted.here, counted.wanted))
            )
        })
        .collect();
    lines.push(if count(findings, Part::Required).whole() {
        "  next: sprawling up".to_owned()
    } else {
        "  next: sprawling doctor --install".to_owned()
    });
    lines
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Counted {
    pub(crate) here: usize,
    pub(crate) wanted: usize,
}

impl Counted {
    fn whole(self) -> bool {
        self.here == self.wanted
    }
}

pub(crate) fn count(findings: &[Finding], part: Part) -> Counted {
    let mut here: usize = 0;
    let mut wanted: usize = 0;
    let mut groups: Vec<(super::Group, bool)> = Vec::new();
    for finding in findings
        .iter()
        .filter(|found| Part::of(found.requirement) == part)
    {
        let usable = finding.presence.usable();
        match finding.requirement.need {
            Need::Required | Need::Optional => {
                wanted = wanted.saturating_add(1);
                if usable {
                    here = here.saturating_add(1);
                }
            }
            Need::OneOf(group) => match groups.iter_mut().find(|(seen, _)| *seen == group) {
                Some(seen) => seen.1 = seen.1 || usable,
                None => groups.push((group, usable)),
            },
        }
    }
    for (_, satisfied) in groups {
        wanted = wanted.saturating_add(1);
        if satisfied {
            here = here.saturating_add(1);
        }
    }
    Counted { here, wanted }
}

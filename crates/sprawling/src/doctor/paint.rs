// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The report as a table: four status words, two columns of fixed
//! width, and colour a terminal may refuse (sprawling-SPEC.md section
//! 8-58).
//!
//! A report that is left-aligned prose makes a person read every line
//! to find the one that is red. Here the name column is fixed, the
//! second column is one of exactly four words, and everything after it
//! is the shortest true detail: a version number rather than a vendor's
//! whole banner, a fault rather than the word "absent".
//!
//! Nothing here writes to a terminal or reads the environment: the ink
//! arrives as a value that `screen` decided, so a test asks for either
//! answer without setting a variable on the machine it runs on.

use std::ffi::OsString;

use super::family::member_at;
use super::{Detection, Finding, Need, Presence, Tier};

/// Whether this report may use colour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Ink {
    Colour,
    Plain,
}

impl Ink {
    /// Plain when the environment asks for plain, whatever the command
    /// line said.
    ///
    /// `NO_COLOR` is honoured whatever its value is, including an empty
    /// one: the convention is that the variable's presence is the
    /// request. `--no-color` says the same thing for one run, and the
    /// two never disagree because either one is enough.
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

/// The four words a person reads down the second column.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Status {
    /// Here, and it answered.
    Present,
    /// Needed, and not here.
    Missing,
    /// Here, and unusable.
    Broken,
    /// Not here, and nothing is waiting for it.
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

    /// The colour a terminal that accepts colour gets: green for here,
    /// red for what is standing in the way, yellow for what is here and
    /// will not work, dim for what nobody is waiting for.
    fn colour(self) -> &'static str {
        match self {
            Status::Present => "32",
            Status::Missing => "31",
            Status::Broken => "33",
            Status::Optional => "2",
        }
    }
}

/// Which of the report's two sections an item belongs to.
///
/// Required is what stands between a person and a running city;
/// everything else - the optional items and the whole develop tier - is
/// a recommendation. This is the one authority on that split, and the
/// page's two columns read the same fields.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Part {
    Required,
    Recommended,
}

impl Part {
    pub(crate) const ALL: [Part; 2] = [Part::Required, Part::Recommended];

    pub(crate) fn of(finding: &Finding) -> Part {
        match (finding.requirement.tier, finding.requirement.need) {
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

/// The width of the name column. Wide enough for the longest row name
/// this table carries, so no name pushes its status word out of line.
const NAME: usize = 18;

/// The width of the status column, one space wider than its longest
/// word.
const STATUS: usize = 9;

/// One row: name, status word, and the shortest true detail.
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

/// What the row says after its status word.
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

/// Which brand of a family was found, and what a person has to know
/// about that brand. Empty for a row that is not a family: nothing is
/// added to a line about `git`.
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

/// The closing three lines: how much of each section is here, and the
/// one command that moves a person forward from where they are.
///
/// A family of browsers counts once. Four ways into a session, three of
/// them closed and one open, is one thing a person has rather than a
/// quarter of one, and a fraction that said `1 / 4` beside a verdict
/// that said `ready to use` would make a reader distrust both.
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

/// How many of one section's items a person has, and how many there are
/// to have. A group is one of each.
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
    for finding in findings.iter().filter(|found| Part::of(found) == part) {
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

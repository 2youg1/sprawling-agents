// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine's answer, in the shape a page reads.
//!
//! The terminal report and this one fold the same findings: `screen`
//! turns them into one machine's prose, and this turns them into
//! values a browser in either language can label. Neither asks the
//! machine a second time, and neither decides anything - what is here
//! and what a tier needs are settled in `doctor` and in `table`.

use super::{Absence, Fault, Version};
use super::{Finding, Need, PATIENCE, Platform, Presence, ThisMachine, Tier, Verdict};
use super::{examine, verdict};

/// Asks this machine once and folds what it said into the answer the
/// wire carries.
///
/// Called at the point a city is served and nowhere else: every item is
/// a program started and asked its version, which is seconds rather
/// than milliseconds, and a query that did that would hold the one
/// thread every other read is answered on.
pub(crate) fn report() -> channels::DoctorAnswer {
    let platform = Platform::current();
    fold(&examine(&ThisMachine::new(platform, PATIENCE)), platform)
}

/// The findings, in the shape the wire carries. Separate from the ask
/// so that a test says what this machine answered instead of having
/// one.
fn fold(findings: &[Finding], platform: Option<Platform>) -> channels::DoctorAnswer {
    channels::DoctorAnswer {
        items: findings.iter().map(|found| item(found, platform)).collect(),
        tiers: Tier::ALL
            .into_iter()
            .map(|tier| channels::DoctorVerdict {
                tier: named(tier),
                missing: match verdict(findings, tier) {
                    Verdict::Ready => Vec::new(),
                    Verdict::Missing(names) => names.into_iter().map(str::to_owned).collect(),
                },
            })
            .collect(),
    }
}

fn item(found: &Finding, platform: Option<Platform>) -> channels::DoctorItem {
    channels::DoctorItem {
        name: found.requirement.name.to_owned(),
        tier: named(found.requirement.tier),
        // A family carries `OneOf`, and the page is told `Required`:
        // the wire says whether an item stands between a person and a
        // running city, and the tier's `missing` says whether the group
        // as a whole is satisfied. A third word here would be a word no
        // reader could act on without also reading the verdict.
        need: match found.requirement.need {
            Need::Required | Need::OneOf(_) => channels::DoctorNeed::Required,
            Need::Optional => channels::DoctorNeed::Optional,
        },
        enables: found.requirement.enables.to_owned(),
        homepage: homepage(found),
        state: state(&found.presence),
        install: match platform {
            None => channels::DoctorInstall::UnknownPlatform,
            Some(platform) => install(found.requirement.recipe.at(platform)),
        },
    }
}

/// The site a page links the item's name to: the brand's own when this
/// machine answered with a member of a browser family, so a person who
/// runs Zen is not sent to Mozilla, and the row's otherwise.
fn homepage(found: &Finding) -> Option<String> {
    if let super::Detection::Family(family) = &found.requirement.detect
        && let Some(member) = found
            .presence
            .at()
            .and_then(|at| super::family::member_at(*family, at))
    {
        return Some(member.homepage.to_owned());
    }
    found.requirement.homepage.map(str::to_owned)
}

fn named(tier: Tier) -> channels::DoctorTier {
    match tier {
        Tier::Use => channels::DoctorTier::Use,
        Tier::Develop => channels::DoctorTier::Develop,
    }
}

fn state(presence: &Presence) -> channels::DoctorState {
    match presence {
        Presence::Present { at, version } => channels::DoctorState::Present {
            at: at.display().to_string(),
            version: match version {
                Version::Said(text) => channels::DoctorVersion::Said { text: text.clone() },
                Version::Silent => channels::DoctorVersion::Silent,
                Version::Unreadable => channels::DoctorVersion::Unreadable,
                Version::Late => channels::DoctorVersion::Late,
            },
        },
        Presence::Broken { at, fault } => channels::DoctorState::Broken {
            at: at.display().to_string(),
            fault: match fault {
                Fault::WillNotStart(said) => {
                    channels::DoctorFault::WillNotStart { said: said.clone() }
                }
                Fault::HalfWritten => channels::DoctorFault::HalfWritten,
                Fault::Unreadable(said) => channels::DoctorFault::Unreadable { said: said.clone() },
            },
        },
        Presence::Absent(absence) => channels::DoctorState::Absent {
            absence: match absence {
                Absence::NotOnSearchPath => channels::DoctorAbsence::NotOnSearchPath,
                Absence::VariableNamesNothing { variable, path } => {
                    channels::DoctorAbsence::VariableNamesNothing {
                        variable: (*variable).to_owned(),
                        path: path.display().to_string(),
                    }
                }
                Absence::NoComponent { dir } => channels::DoctorAbsence::NoComponent {
                    dir: dir.display().to_string(),
                },
                Absence::NoHome => channels::DoctorAbsence::NoHome,
                Absence::NotInThisBuild => channels::DoctorAbsence::NotInThisBuild,
            },
        },
    }
}

fn install(recipe: &super::Recipe) -> channels::DoctorInstall {
    let spelled = recipe.spelled();
    match recipe {
        super::Recipe::Command { .. } => channels::DoctorInstall::Command { spelled },
        super::Recipe::Print(_) => channels::DoctorInstall::Print { spelled },
        super::Recipe::Manual(how) => channels::DoctorInstall::Manual {
            how: (*how).to_owned(),
        },
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "test code")]
mod tests {
    use super::fold;
    use crate::doctor::tests::ScriptedMachine;
    use crate::doctor::{Platform, examine};

    /// The page is told the same thing the terminal is told, item for
    /// item and verdict for verdict - and told it in values rather than
    /// in one machine's prose, because the browser has two languages
    /// and only one of them is this one.
    #[test]
    fn the_answer_says_what_is_missing_and_what_would_get_it() {
        let machine =
            ScriptedMachine::missing(&["gecko", "webkit", "chromedriver", "msedgedriver"]);
        let answer = fold(&examine(&machine), Some(Platform::Windows));

        let gecko = answer
            .items
            .iter()
            .find(|item| item.name == "gecko")
            .expect("the table carries the Gecko family");
        assert_eq!(
            gecko.state,
            channels::DoctorState::Absent {
                absence: channels::DoctorAbsence::NotOnSearchPath,
            },
            "an absence is a value the page labels, never a sentence"
        );
        assert!(
            matches!(
                gecko.install,
                channels::DoctorInstall::Command { .. } | channels::DoctorInstall::Print { .. }
            ),
            "a missing item says what would get it: {:?}",
            gecko.install
        );

        let using = answer
            .tiers
            .iter()
            .find(|verdict| verdict.tier == channels::DoctorTier::Use)
            .expect("every tier answers");
        assert_eq!(
            using.missing,
            vec!["a browser engine".to_owned()],
            "the tier a person needs names what stands between them and it"
        );
        let developing = answer
            .tiers
            .iter()
            .find(|verdict| verdict.tier == channels::DoctorTier::Develop)
            .expect("every tier answers");
        assert!(
            developing.missing.is_empty(),
            "one tier's absence is not the other's: {:?}",
            developing.missing
        );
    }

    /// A machine this project names no recipes for says so, rather than
    /// spelling a command from another platform.
    #[test]
    fn a_platform_with_no_recipes_offers_none() {
        let machine =
            ScriptedMachine::missing(&["gecko", "webkit", "chromedriver", "msedgedriver"]);
        let answer = fold(&examine(&machine), None);
        assert!(
            answer
                .items
                .iter()
                .all(|item| item.install == channels::DoctorInstall::UnknownPlatform),
            "nothing can be said about installing on a platform nobody wrote recipes for"
        );
    }
}

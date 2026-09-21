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
    let findings = examine(&ThisMachine::new(platform, PATIENCE));
    fold(&findings, platform, confinement(), custody())
}

/// The findings, in the shape the wire carries. Separate from the ask
/// so that a test says what this machine answered instead of having
/// one, and separate from the two machine-wide reads below so that a
/// verdict is judged without a machine that has a keyring and a search
/// path.
fn fold(
    findings: &[Finding],
    platform: Option<Platform>,
    sandbox: channels::DoctorSandbox,
    custody: channels::DoctorCustody,
) -> channels::DoctorAnswer {
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
        sandbox,
        custody,
    }
}

/// The arm a host command runs under here, as the wire carries it.
///
/// Read from the module that decides it rather than decided again here:
/// a page naming one arm while commands run under another would be
/// worse than no page at all.
fn confinement() -> channels::DoctorSandbox {
    let arm = runtime::tools::Confinement::detect();
    channels::DoctorSandbox {
        arm: arm_name(&arm),
        coverage: runtime::tools::Guarantee::ALL
            .iter()
            .map(|axis| channels::DoctorGuarantee {
                axis: axis_name(*axis),
                kept: match arm.assurances().of(*axis) {
                    runtime::tools::Kept::Yes => channels::DoctorCoverage::Kept,
                    runtime::tools::Kept::No => channels::DoctorCoverage::NotKept,
                },
            })
            .collect(),
    }
}

/// Where this machine's credentials rest, from the one startup probe
/// there is.
///
/// The probe writes a value, reads it back and deletes it, which is what
/// makes the answer about this machine rather than about a configuration
/// file. It also builds the `provider_degraded` notice the ledger
/// carries; that notice belongs to the caller that keeps the custodian,
/// and this report has no ledger to write it to.
fn custody() -> channels::DoctorCustody {
    let (custodian, _notice) = gateway::Custodian::probe();
    let custody = custodian.custody();
    channels::DoctorCustody {
        store: match custody.store {
            gateway::Store::PlatformService => channels::DoctorCustodyStore::PlatformService,
            gateway::Store::EncryptedFile => channels::DoctorCustodyStore::EncryptedFile,
            gateway::Store::SessionMemory => channels::DoctorCustodyStore::SessionMemory,
        },
        keeps: match custody.persistence {
            gateway::Persistence::AcrossReboots => channels::DoctorCustodyLifetime::AcrossReboots,
            gateway::Persistence::AcrossRebootsWithPassphrase => {
                channels::DoctorCustodyLifetime::WithPassphrase
            }
            gateway::Persistence::ThisBoot => channels::DoctorCustodyLifetime::UntilReboot,
            gateway::Persistence::ThisProcess => channels::DoctorCustodyLifetime::ThisProcess,
        },
        refusal: custody.refusal,
    }
}

fn arm_name(arm: &runtime::tools::Confinement) -> channels::DoctorSandboxArm {
    match arm {
        runtime::tools::Confinement::LinuxNamespaces { .. } => {
            channels::DoctorSandboxArm::LinuxNamespaces
        }
        runtime::tools::Confinement::WindowsJobObject => {
            channels::DoctorSandboxArm::WindowsJobObject
        }
        runtime::tools::Confinement::CopiedTree => channels::DoctorSandboxArm::CopiedTree,
        runtime::tools::Confinement::Unavailable { missing } => {
            channels::DoctorSandboxArm::Unavailable {
                missing: match missing {
                    runtime::tools::Missing::ScratchDirectory => {
                        channels::DoctorSandboxMissing::ScratchDirectory
                    }
                },
            }
        }
    }
}

fn axis_name(axis: runtime::tools::Guarantee) -> channels::DoctorGuaranteeAxis {
    match axis {
        runtime::tools::Guarantee::Filesystem => channels::DoctorGuaranteeAxis::Filesystem,
        runtime::tools::Guarantee::Network => channels::DoctorGuaranteeAxis::Network,
        runtime::tools::Guarantee::ProcessTree => channels::DoctorGuaranteeAxis::ProcessTree,
        runtime::tools::Guarantee::User => channels::DoctorGuaranteeAxis::User,
        runtime::tools::Guarantee::Resources => channels::DoctorGuaranteeAxis::Resources,
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
    use super::axis_name;
    use super::fold;
    use crate::doctor::tests::ScriptedMachine;
    use crate::doctor::{Platform, examine};

    /// The two machine-wide reads a fold is given, so that a verdict is
    /// judged without a machine that has a search path and a keyring.
    fn stated() -> (channels::DoctorSandbox, channels::DoctorCustody) {
        (
            channels::DoctorSandbox {
                arm: channels::DoctorSandboxArm::CopiedTree,
                coverage: runtime::tools::Guarantee::ALL
                    .iter()
                    .map(|axis| channels::DoctorGuarantee {
                        axis: axis_name(*axis),
                        kept: match runtime::tools::Confinement::CopiedTree
                            .assurances()
                            .of(*axis)
                        {
                            runtime::tools::Kept::Yes => channels::DoctorCoverage::Kept,
                            runtime::tools::Kept::No => channels::DoctorCoverage::NotKept,
                        },
                    })
                    .collect(),
            },
            channels::DoctorCustody {
                store: channels::DoctorCustodyStore::SessionMemory,
                keeps: channels::DoctorCustodyLifetime::ThisProcess,
                refusal: None,
            },
        )
    }

    /// The page is told the same thing the terminal is told, item for
    /// item and verdict for verdict - and told it in values rather than
    /// in one machine's prose, because the browser has two languages
    /// and only one of them is this one.
    #[test]
    fn the_answer_says_what_is_missing_and_what_would_get_it() {
        let machine =
            ScriptedMachine::missing(&["gecko", "webkit", "chromedriver", "msedgedriver"]);
        let (sandbox, custody) = stated();
        let answer = fold(
            &examine(&machine),
            Some(Platform::Windows),
            sandbox,
            custody,
        );

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
        let (sandbox, custody) = stated();
        let answer = fold(&examine(&machine), None, sandbox, custody);
        assert!(
            answer
                .items
                .iter()
                .all(|item| item.install == channels::DoctorInstall::UnknownPlatform),
            "nothing can be said about installing on a platform nobody wrote recipes for"
        );
    }

    /// What the page shows about the sandbox is the arm's own list, axis
    /// for axis, rather than a second opinion about it: a report that
    /// said "sandboxed" while the network stayed open is the failure the
    /// arm's list exists to prevent.
    #[test]
    fn the_sandbox_rows_are_the_arms_own_assurances() {
        let machine = ScriptedMachine::missing(&[]);
        let (sandbox, custody) = stated();
        let answer = fold(
            &examine(&machine),
            Some(Platform::Windows),
            sandbox,
            custody,
        );
        assert_eq!(answer.sandbox.arm, channels::DoctorSandboxArm::CopiedTree);
        assert_eq!(answer.sandbox.coverage.len(), 5, "one row per axis");
        let network = answer
            .sandbox
            .coverage
            .iter()
            .find(|row| row.axis == channels::DoctorGuaranteeAxis::Network)
            .expect("every axis is reported");
        assert_eq!(
            network.kept,
            channels::DoctorCoverage::NotKept,
            "the copied tree does not isolate the network, and the page is told so"
        );
    }
}

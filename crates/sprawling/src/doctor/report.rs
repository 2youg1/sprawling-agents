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

use super::scanning::{Drive, Exclusion, Scanning, Untold};
use super::{Absence, Fault, Version};
use super::{Finding, Machine, Need, Platform, Presence, Tier, Verdict};
use super::{examine, verdict};
use crate::serving::standing::{Held, Standing};
use kernel::AxError;
use std::path::Path;

/// Asks `machine` once and folds what it said into the answer the
/// wire carries. `ThisMachine`'s `accounting::Machine::report` is this.
///
/// Reached through `accounting::Machine::report`, which the worker calls
/// from `DoctorRefresh` and nowhere else: every item but the
/// browsers is a program started and asked its version, which is
/// seconds rather than milliseconds, so neither a serve nor a query
/// waits for it - a person opening the page that shows it asks for it,
/// and the city holds the answer until they ask again.
///
/// `city` is the directory whose disk the scanning reading judges.
pub(crate) fn answer(machine: &dyn Machine, city: &Path) -> wire::DoctorAnswer {
    let wide = MachineWide {
        sandbox: confinement(),
        custody: custody(),
    };
    fold(machine, Platform::current(), wide, city)
}

/// The two machine-wide reads that are not asked of `Machine`: the box
/// a command runs in, and where credentials rest.
struct MachineWide {
    sandbox: wire::DoctorSandbox,
    custody: wire::DoctorCustody,
}

/// The findings, in the shape the wire carries. Separate from the ask
/// so that a test says what this machine answered instead of having
/// one, and separate from the two machine-wide reads below so that a
/// verdict is judged without a machine that has a keyring and a search
/// path.
fn fold(
    machine: &dyn Machine,
    platform: Option<Platform>,
    wide: MachineWide,
    city: &Path,
) -> wire::DoctorAnswer {
    let findings = &examine(machine);
    wire::DoctorAnswer {
        items: findings.iter().map(|found| item(found, platform)).collect(),
        tiers: Tier::ALL
            .into_iter()
            .map(|tier| wire::DoctorVerdict {
                tier: named(tier),
                missing: match verdict(findings, tier) {
                    Verdict::Ready => Vec::new(),
                    Verdict::Missing(names) => names.into_iter().map(str::to_owned).collect(),
                },
            })
            .collect(),
        sandbox: wide.sandbox,
        custody: wide.custody,
        core: core_level(machine.core_standing()),
        scanning: scanned(machine.scanning(city)),
    }
}

/// The scanning reading, arm for arm as the terminal's part reads it
/// (`crates/wire/spec/Answer/Doctor.lean` D25).
fn scanned(scanning: Scanning) -> wire::DoctorScanning {
    match scanning {
        Scanning::DoesNotApply => wire::DoctorScanning::DoesNotApply,
        Scanning::Stopped => wire::DoctorScanning::Stopped,
        Scanning::Read {
            city,
            drive,
            exclusion,
        } => wire::DoctorScanning::Read {
            city: city.display().to_string(),
            drive: match drive {
                Drive::Trusted => wire::DoctorDrive::Trusted,
                Drive::Untrusted { volume } => wire::DoctorDrive::Untrusted { volume },
                Drive::Not {
                    volume,
                    file_system,
                } => wire::DoctorDrive::Not {
                    volume,
                    file_system,
                },
                Drive::Untold(why) => wire::DoctorDrive::Untold { why: untold(why) },
            },
            exclusion: match exclusion {
                Exclusion::Inside { under } => wire::DoctorExclusion::Inside { under },
                Exclusion::Outside => wire::DoctorExclusion::Outside,
                Exclusion::Untold(why) => wire::DoctorExclusion::Untold { why: untold(why) },
            },
        },
    }
}

fn untold(why: Untold) -> wire::DoctorUntold {
    match why {
        Untold::NoDisk => wire::DoctorUntold::NoDisk,
        Untold::AdminOnly => wire::DoctorUntold::AdminOnly,
        Untold::Unread { command } => wire::DoctorUntold::Unread {
            command: command.to_owned(),
        },
        Untold::Failed { command, code } => wire::DoctorUntold::Failed {
            command: command.to_owned(),
            code,
        },
        Untold::Unstarted { command } => wire::DoctorUntold::Unstarted {
            command: command.to_owned(),
        },
        Untold::Unanswered { command, stopping } => wire::DoctorUntold::Unanswered {
            command: command.to_owned(),
            stopping,
        },
    }
}

/// The level the terminal's priority part names, as the wire carries it.
fn core_level(standing: Result<Standing, AxError>) -> wire::DoctorCore {
    match standing {
        Ok(Standing::Raised) => wire::DoctorCore::Raised,
        Ok(Standing::Normal(Held::ByTheSetting)) => wire::DoctorCore::HeldBySetting,
        Ok(Standing::Normal(Held::Refused(said))) => wire::DoctorCore::Refused { said },
        Ok(Standing::Normal(Held::ByTheValve)) => wire::DoctorCore::LoweredByValve,
        Err(err) => wire::DoctorCore::Unasked {
            said: err.to_string(),
        },
    }
}

/// The arm a host command runs under here, as the wire carries it.
///
/// Read from the module that decides it rather than decided again here:
/// a page naming one arm while commands run under another would be
/// worse than no page at all.
fn confinement() -> wire::DoctorSandbox {
    let arm = runtime::tools::Confinement::detect();
    wire::DoctorSandbox {
        arm: arm_name(&arm),
        named: chosen_name(&arm),
        coverage: runtime::tools::Guarantee::ALL
            .iter()
            .map(|axis| wire::DoctorGuarantee {
                axis: axis_name(*axis),
                kept: match arm.assurances().of(*axis) {
                    runtime::tools::Kept::Yes => wire::DoctorCoverage::Kept,
                    runtime::tools::Kept::No => wire::DoctorCoverage::NotKept,
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
fn custody() -> wire::DoctorCustody {
    let (custodian, _notice) = gateway::Custodian::probe();
    let custody = custodian.custody();
    wire::DoctorCustody {
        store: match custody.store {
            gateway::Store::PlatformService => wire::DoctorCustodyStore::PlatformService,
            gateway::Store::EncryptedFile => wire::DoctorCustodyStore::EncryptedFile,
            gateway::Store::SessionMemory => wire::DoctorCustodyStore::SessionMemory,
        },
        keeps: match custody.persistence {
            gateway::Persistence::AcrossReboots => wire::DoctorCustodyLifetime::AcrossReboots,
            gateway::Persistence::AcrossRebootsWithPassphrase => {
                wire::DoctorCustodyLifetime::WithPassphrase
            }
            gateway::Persistence::ThisBoot => wire::DoctorCustodyLifetime::UntilReboot,
            gateway::Persistence::ThisProcess => wire::DoctorCustodyLifetime::ThisProcess,
        },
        refusal: custody.refusal,
    }
}

fn arm_name(arm: &runtime::tools::Confinement) -> wire::DoctorSandboxArm {
    match arm {
        runtime::tools::Confinement::LinuxNamespaces { .. } => {
            wire::DoctorSandboxArm::LinuxNamespaces
        }
        runtime::tools::Confinement::WindowsJobObject => wire::DoctorSandboxArm::WindowsJobObject,
        runtime::tools::Confinement::CopiedTree => wire::DoctorSandboxArm::CopiedTree,
        runtime::tools::Confinement::Unavailable { missing } => {
            wire::DoctorSandboxArm::Unavailable {
                missing: match missing {
                    runtime::tools::Missing::ScratchDirectory => {
                        wire::DoctorSandboxMissing::ScratchDirectory
                    }
                },
            }
        }
    }
}

/// The arm's name in the set a setting chooses from (wire D26). An
/// unavailable arm runs a command on the host itself, which is `None`.
fn chosen_name(arm: &runtime::tools::Confinement) -> wire::SandboxArm {
    match arm {
        runtime::tools::Confinement::LinuxNamespaces { .. }
        | runtime::tools::Confinement::WindowsJobObject => wire::SandboxArm::Native,
        runtime::tools::Confinement::CopiedTree => wire::SandboxArm::CopiedTree,
        runtime::tools::Confinement::Unavailable { .. } => wire::SandboxArm::None,
    }
}

fn axis_name(axis: runtime::tools::Guarantee) -> wire::DoctorGuaranteeAxis {
    match axis {
        runtime::tools::Guarantee::Filesystem => wire::DoctorGuaranteeAxis::Filesystem,
        runtime::tools::Guarantee::Network => wire::DoctorGuaranteeAxis::Network,
        runtime::tools::Guarantee::ProcessTree => wire::DoctorGuaranteeAxis::ProcessTree,
        runtime::tools::Guarantee::User => wire::DoctorGuaranteeAxis::User,
        runtime::tools::Guarantee::Resources => wire::DoctorGuaranteeAxis::Resources,
    }
}

fn item(found: &Finding, platform: Option<Platform>) -> wire::DoctorItem {
    wire::DoctorItem {
        name: found.requirement.name.to_owned(),
        tier: named(found.requirement.tier),
        // A family carries `OneOf`, and the page is told `Required`:
        // the wire says whether an item stands between a person and a
        // running city, and the tier's `missing` says whether the group
        // as a whole is satisfied. A third word here would be a word no
        // reader could act on without also reading the verdict.
        need: match found.requirement.need {
            Need::Required | Need::OneOf(_) => wire::DoctorNeed::Required,
            Need::Optional => wire::DoctorNeed::Optional,
        },
        homepage: homepage(found),
        state: state(&found.presence),
        install: match platform {
            None => wire::DoctorInstall::UnknownPlatform,
            Some(platform) => install(found.requirement.recipe.at(platform)),
        },
        pinned: super::pin::pinned(found.requirement.pin),
        pack: found.requirement.pack.map(super::Pack::wire),
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

fn named(tier: Tier) -> wire::DoctorTier {
    match tier {
        Tier::Use => wire::DoctorTier::Use,
        Tier::Develop => wire::DoctorTier::Develop,
    }
}

fn state(presence: &Presence) -> wire::DoctorState {
    match presence {
        Presence::Present { at, version } => wire::DoctorState::Present {
            at: at.display().to_string(),
            version: match version {
                Version::Said(text) => wire::DoctorVersion::Said { text: text.clone() },
                Version::Silent => wire::DoctorVersion::Silent,
                Version::Unreadable => wire::DoctorVersion::Unreadable,
                Version::Late => wire::DoctorVersion::Late,
            },
        },
        Presence::Broken { at, fault } => wire::DoctorState::Broken {
            at: at.display().to_string(),
            fault: match fault {
                Fault::WillNotStart(said) => wire::DoctorFault::WillNotStart { said: said.clone() },
                Fault::HalfWritten => wire::DoctorFault::HalfWritten,
                Fault::Unreadable(said) => wire::DoctorFault::Unreadable { said: said.clone() },
            },
        },
        Presence::Absent(absence) => wire::DoctorState::Absent {
            absence: match absence {
                Absence::NotOnSearchPath => wire::DoctorAbsence::NotOnSearchPath,
                Absence::VariableNamesNothing { variable, path } => {
                    wire::DoctorAbsence::VariableNamesNothing {
                        variable: (*variable).to_owned(),
                        path: path.display().to_string(),
                    }
                }
                Absence::NoComponent { dir } => wire::DoctorAbsence::NoComponent {
                    dir: dir.display().to_string(),
                },
                Absence::NoHome => wire::DoctorAbsence::NoHome,
                Absence::NotInThisBuild => wire::DoctorAbsence::NotInThisBuild,
            },
        },
    }
}

fn install(recipe: &super::Recipe) -> wire::DoctorInstall {
    let spelled = recipe.spelled();
    match recipe {
        super::Recipe::Command { .. } => wire::DoctorInstall::Command { spelled },
        super::Recipe::Print(_) => wire::DoctorInstall::Print { spelled },
        super::Recipe::Manual(how) => wire::DoctorInstall::Manual {
            how: (*how).to_owned(),
        },
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "test code")]
mod tests {
    use super::axis_name;
    use super::{MachineWide, fold};
    use crate::doctor::Platform;
    use crate::doctor::tests::ScriptedMachine;
    use crate::serving::standing::{Held, Standing};
    use std::path::Path;

    /// The two machine-wide reads a fold is given, so that a verdict is
    /// judged without a machine that has a search path and a keyring.
    fn stated() -> MachineWide {
        MachineWide {
            sandbox: wire::DoctorSandbox {
                arm: wire::DoctorSandboxArm::CopiedTree,
                named: wire::SandboxArm::CopiedTree,
                coverage: runtime::tools::Guarantee::ALL
                    .iter()
                    .map(|axis| wire::DoctorGuarantee {
                        axis: axis_name(*axis),
                        kept: match runtime::tools::Confinement::CopiedTree
                            .assurances()
                            .of(*axis)
                        {
                            runtime::tools::Kept::Yes => wire::DoctorCoverage::Kept,
                            runtime::tools::Kept::No => wire::DoctorCoverage::NotKept,
                        },
                    })
                    .collect(),
            },
            custody: wire::DoctorCustody {
                store: wire::DoctorCustodyStore::SessionMemory,
                keeps: wire::DoctorCustodyLifetime::ThisProcess,
                refusal: None,
            },
        }
    }

    /// The page is told the same thing the terminal is told, item for
    /// item and verdict for verdict - and told it in values rather than
    /// in one machine's prose, because the browser has two languages
    /// and only one of them is this one.
    #[test]
    fn the_answer_says_what_is_missing_and_what_would_get_it() {
        let machine =
            ScriptedMachine::missing(&["gecko", "webkit", "chromedriver", "msedgedriver"]);
        let answer = fold(
            &machine,
            Some(Platform::Windows),
            stated(),
            Path::new("city"),
        );

        let gecko = answer
            .items
            .iter()
            .find(|item| item.name == "gecko")
            .expect("the table carries the Gecko family");
        assert_eq!(
            gecko.state,
            wire::DoctorState::Absent {
                absence: wire::DoctorAbsence::NotOnSearchPath,
            },
            "an absence is a value the page labels, never a sentence"
        );
        assert!(
            matches!(
                gecko.install,
                wire::DoctorInstall::Command { .. } | wire::DoctorInstall::Print { .. }
            ),
            "a missing item says what would get it: {:?}",
            gecko.install
        );

        let using = answer
            .tiers
            .iter()
            .find(|verdict| verdict.tier == wire::DoctorTier::Use)
            .expect("every tier answers");
        assert_eq!(
            using.missing,
            vec!["a browser engine".to_owned()],
            "the tier a person needs names what stands between them and it"
        );
        let developing = answer
            .tiers
            .iter()
            .find(|verdict| verdict.tier == wire::DoctorTier::Develop)
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
        let answer = fold(&machine, None, stated(), Path::new("city"));
        assert!(
            answer
                .items
                .iter()
                .all(|item| item.install == wire::DoctorInstall::UnknownPlatform),
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
        let answer = fold(
            &machine,
            Some(Platform::Windows),
            stated(),
            Path::new("city"),
        );
        assert_eq!(answer.sandbox.arm, wire::DoctorSandboxArm::CopiedTree);
        assert_eq!(answer.sandbox.coverage.len(), 5, "one row per axis");
        let network = answer
            .sandbox
            .coverage
            .iter()
            .find(|row| row.axis == wire::DoctorGuaranteeAxis::Network)
            .expect("every axis is reported");
        assert_eq!(
            network.kept,
            wire::DoctorCoverage::NotKept,
            "the copied tree does not isolate the network, and the page is told so"
        );
    }

    /// The page is told the level the terminal report names, from the
    /// same reading, and the platform's reason with it: a machine that
    /// refuses the raise is the one a person has to act on.
    #[test]
    fn the_page_is_told_where_the_core_stands() {
        let machine = ScriptedMachine::missing(&[]).standing(Standing::Normal(Held::Refused(
            "the raise needs CAP_SYS_NICE".to_owned(),
        )));
        let answer = fold(&machine, None, stated(), Path::new("city"));
        assert_eq!(
            answer.core,
            wire::DoctorCore::Refused {
                said: "the raise needs CAP_SYS_NICE".to_owned(),
            }
        );
    }
}

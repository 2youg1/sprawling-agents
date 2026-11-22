// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::scanning::{Drive, Exclusion, Scanning, Untold};
use super::{Absence, Fault, Version};
use super::{Finding, Machine, Need, Platform, Presence, Tier, Verdict};
use super::{examine, verdict};
use crate::serving::standing::{Held, Standing};
use kernel::AxError;
use std::path::Path;

pub(crate) fn answer(machine: &dyn Machine, city: &Path) -> wire::DoctorAnswer {
    let wide = MachineWide {
        sandbox: confinement(),
        custody: custody(),
    };
    fold(machine, Platform::current(), wide, city)
}

struct MachineWide {
    sandbox: wire::DoctorSandbox,
    custody: wire::DoctorCustody,
}

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
        runtime::tools::Confinement::MacosSeatbelt { .. } => wire::DoctorSandboxArm::MacosSeatbelt,
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

fn chosen_name(arm: &runtime::tools::Confinement) -> wire::SandboxArm {
    match arm {
        runtime::tools::Confinement::LinuxNamespaces { .. }
        | runtime::tools::Confinement::WindowsJobObject
        | runtime::tools::Confinement::MacosSeatbelt { .. } => wire::SandboxArm::Native,
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
                Absence::NotInstalled => wire::DoctorAbsence::NoComponent {
                    dir: "the platform's font directories".to_owned(),
                },
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

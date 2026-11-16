// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::catalog::CatalogEntry;
use kernel::{AdmissionRequirement, AxCode, AxError, LandingPolicy, Mode, RunPolicy};
use std::sync::{Arc, PoisonError, RwLock};

pub const DEV_ENTRY: &str = "dev";

#[must_use]
pub fn dev_entry() -> CatalogEntry {
    CatalogEntry {
        name: DEV_ENTRY.to_owned(),
        disclosure: "when the work is to change this city's own code, SPECs or tools, open this \
                     entry before you touch anything"
            .to_owned(),
        expansion: "This city is built from small components, each with its SPEC beside it at \
                    `crates/<crate>/<crate>-SPEC.md`. Read that SPEC, then the code, then the \
                    tests next to the code - in that order, and before you change any of them. \
                    Where the implementation would differ from the SPEC, the SPEC changes first \
                    and says why.\n\nThe User chooses what a change must prove before it is \
                    merged:\n- tested: the asset's own tests ran and passed.\n- contract_kept: \
                    the asset's observable contract did not move.\n- double_validated: the change \
                    holds on held-in and on held-out evidence.\nAnd where it lands: \
                    ordinary work takes the building's own road, while an experiment works in a \
                    tree of its own and nothing in it is merged.\n\nYour next step: say which \
                    evidence this work needs and why, and wait for the User to dispatch it \
                    with that requirement. Choosing a requirement does not provide the evidence."
            .to_owned(),

        hash: None,
        document: String::new(),
    }
}

#[must_use]
pub fn catalog_entry(mode: Mode) -> CatalogEntry {
    let (disclosure, expansion) = match mode {
        Mode::Chat => (
            "chat mode: focus on conversing with the User; answer what they said, in their language",
            "Reply in the conversation. Start work, plans or dispatches only when the User asks for them.",
        ),
        Mode::Work => (
            "work mode: carry out the task towards the stated goal; report when the goal is met",
            "When the task asks for a plan first, write it into Roadmap.md with the `plan` tool \
             before you change anything, then work through it. Report what you did and the \
             evidence that the goal is met.",
        ),
    };
    CatalogEntry {
        name: format!("mode:{}", mode.as_str()),
        disclosure: disclosure.to_owned(),
        expansion: expansion.to_owned(),
        hash: None,
        document: String::new(),
    }
}

#[must_use]
pub fn core_tools(mode: Mode) -> &'static [&'static str] {
    match mode {
        Mode::Chat => &[
            crate::tools::ReadTool::NAME,
            crate::tools::SearchTool::NAME,
            crate::tools::StatusTool::NAME,
            crate::tools::ToolTool::NAME,
        ],
        Mode::Work => &[
            crate::tools::EditTool::NAME,
            kernel::ToolName::EXEC,
            crate::tools::ReadTool::NAME,
            crate::tools::SearchTool::NAME,
            crate::tools::StatusTool::NAME,
            crate::tools::ToolTool::NAME,
        ],
    }
}

#[must_use]
pub fn in_some_core(name: &str) -> bool {
    Mode::ALL
        .iter()
        .any(|mode| core_tools(*mode).contains(&name))
}

pub fn mode_admits(policy: &RunPolicy, name: &str) -> Result<(), AxError> {
    if !in_some_core(name) || core_tools(policy.mode).contains(&name) {
        return Ok(());
    }
    Err(AxError::failure(
        AxCode::GateDenied,
        format!("run {name}"),
        format!("{name} does not run in {} mode", policy.mode.as_str()),
    )
    .with_recovery("answer in the conversation, or ask the User to switch this run to work mode"))
}

#[derive(Debug)]
pub struct PolicyCell {
    shared: Arc<RwLock<RunPolicy>>,
    arrived: Option<RunPolicy>,
}

impl PolicyCell {
    #[must_use]
    pub fn new(start: RunPolicy) -> PolicyCell {
        PolicyCell {
            shared: Arc::new(RwLock::new(start)),
            arrived: None,
        }
    }

    #[must_use]
    pub fn reader(&self) -> PolicyReader {
        PolicyReader {
            shared: Arc::clone(&self.shared),
        }
    }

    pub(crate) fn arrive(&mut self, policy: RunPolicy) {
        self.arrived = Some(policy);
    }

    pub(crate) fn take_at_wave(&mut self) -> Option<RunPolicy> {
        let taken = self.arrived.take()?;

        *self.shared.write().unwrap_or_else(PoisonError::into_inner) = taken;
        Some(taken)
    }
}

#[derive(Debug, Clone)]
pub struct PolicyReader {
    shared: Arc<RwLock<RunPolicy>>,
}

impl PolicyReader {
    #[must_use]
    pub fn now(&self) -> RunPolicy {
        *self.shared.read().unwrap_or_else(PoisonError::into_inner)
    }
}

#[must_use]
pub(crate) fn policy_note(policy: &RunPolicy) -> String {
    format!(
        "The User changed this run's policy; from the next tool call on: mode {}, write {}, \
         admission {}, landing {}.",
        policy.mode.as_str(),
        policy.write.as_str(),
        policy.admit.as_str(),
        policy.landing.as_str()
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Produced {
    pub tests_passed: Option<bool>,

    pub contract_moved: bool,

    pub held_in: Option<bool>,

    pub held_out: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Lands,
    Refused {
        because: &'static str,
        alternative: &'static str,
    },
}

#[must_use]
pub fn admits(policy: &RunPolicy, produced: &Produced) -> Admission {
    match policy.landing {
        LandingPolicy::Experiment => Admission::Refused {
            because: "nothing produced in an experiment lands",
            alternative: "write what you learned into Memo.md, then ask for the work again \
                          with the ordinary landing",
        },
        LandingPolicy::Ordinary => admits_evidence(policy.admit, produced),
    }
}

fn admits_evidence(required: AdmissionRequirement, produced: &Produced) -> Admission {
    match required {
        AdmissionRequirement::Standing => Admission::Lands,
        AdmissionRequirement::Tested => match produced.tests_passed {
            Some(true) => Admission::Lands,
            Some(false) => Admission::Refused {
                because: "the asset's own tests did not pass",
                alternative: "fix the asset or the test, then offer it again",
            },
            None => Admission::Refused {
                because: "the asset has no tests of its own",
                alternative: "write the test that would fail if this asset broke",
            },
        },
        AdmissionRequirement::ContractKept => {
            if produced.contract_moved {
                Admission::Refused {
                    because: "the asset's observable contract moved",
                    alternative: "keep the contract and renovate behind it, or ask for the work \
                                  again under double_validated with held-out evidence",
                }
            } else {
                Admission::Lands
            }
        }
        AdmissionRequirement::DoubleValidated => match (produced.held_in, produced.held_out) {
            (Some(true), Some(true)) => Admission::Lands,
            (Some(false), _) => Admission::Refused {
                because: "it got worse on the held-in set",
                alternative: "a change that costs more than it buys is not adopted; revise it",
            },
            (_, Some(false)) => Admission::Refused {
                because: "it did not stand up on the held-out set",
                alternative: "a gain that only shows where it was built is a gain in fitting, \
                              not in ability",
            },
            _ => Admission::Refused {
                because: "one half of the double validation is missing",
                alternative: "run the suite on both sets; an unmeasured change is not adopted on \
                              confidence",
            },
        },
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[derive(Debug, Clone, Copy)]
    enum Step {
        Change(RunPolicy),
        Wave,
        Call,
    }

    fn any_policy() -> impl Strategy<Value = RunPolicy> {
        (
            prop::sample::select(Mode::ALL.to_vec()),
            prop::sample::select(kernel::WriteLimit::ALL.to_vec()),
            prop::sample::select(AdmissionRequirement::ALL.to_vec()),
            prop::sample::select(LandingPolicy::ALL.to_vec()),
        )
            .prop_map(|(mode, write, admit, landing)| RunPolicy {
                mode,
                write,
                admit,
                landing,
            })
    }

    fn any_step() -> impl Strategy<Value = Step> {
        prop_oneof![
            any_policy().prop_map(Step::Change),
            Just(Step::Wave),
            Just(Step::Call),
        ]
    }

    fn walk(start: RunPolicy, trace: &[Step]) -> Result<(), TestCaseError> {
        let mut cell = PolicyCell::new(start);
        let gate = cell.reader();
        let mut in_force = start;
        let mut last_arrival = None;
        for step in trace {
            match *step {
                Step::Change(policy) => {
                    cell.arrive(policy);
                    last_arrival = Some(policy);
                }
                Step::Wave => {
                    let taken = cell.take_at_wave();
                    prop_assert_eq!(taken, last_arrival.take());
                    in_force = taken.unwrap_or(in_force);
                }
                Step::Call => prop_assert_eq!(gate.now(), in_force),
            }
        }
        Ok(())
    }

    proptest! {
        #[test]
        fn policy_take_holds_on_every_trace(
            start in any_policy(),
            trace in prop::collection::vec(any_step(), 0..24),
        ) {
            walk(start, &trace)?;
        }
    }

    #[test]
    fn a_change_inside_a_wave_leaves_that_wave_alone() {
        let start = RunPolicy::of(Mode::Work);
        let tighter = RunPolicy {
            write: kernel::WriteLimit::Create,
            ..start
        };
        walk(start, &[Step::Call, Step::Change(tighter), Step::Call]).unwrap();
    }

    #[test]
    fn a_chat_run_is_refused_an_edit_and_the_disk_is_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let work = kernel::Address::parse("work").unwrap();
        std::fs::create_dir_all(tmp.path().join("work")).unwrap();
        let tool = crate::tools::EditTool::new(
            tmp.path(),
            work.clone(),
            kernel::WriteDomain::new(vec![work]).unwrap(),
            crate::tools::EditDesk {
                policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Chat)).reader(),
                seen: Default::default(),
                locks: Default::default(),
            },
        )
        .unwrap();
        let refused = kernel::Tool::invoke(&tool, &edit_call());
        assert_eq!(
            refused.err().map(|err| *err.code()),
            Some(AxCode::GateDenied)
        );
        assert!(!tmp.path().join("work/new.md").exists());
    }

    fn edit_call() -> kernel::ToolCall {
        let args = serde_json::json!({
            "file_path": "work/new.md", "old_string": "", "new_string": "text",
        });
        kernel::ToolCall {
            id: "call-1".to_owned(),
            name: kernel::ToolName::parse("edit").unwrap(),
            args: kernel::Payload::of(&args).unwrap(),
        }
    }

    fn work(admit: AdmissionRequirement) -> RunPolicy {
        RunPolicy {
            admit,
            ..RunPolicy::of(Mode::Work)
        }
    }

    #[test]
    fn a_work_run_without_the_evidence_it_chose_does_not_land() {
        let Admission::Refused { because, .. } =
            admits(&work(AdmissionRequirement::Tested), &Produced::default())
        else {
            panic!("work that chose `tested` and ran no test landed");
        };
        assert!(because.contains("no tests"), "{because}");
    }

    #[test]
    fn chat_is_one_line_about_the_conversation() {
        let entry = catalog_entry(Mode::Chat);
        assert_eq!(entry.name, "mode:chat");
        assert!(entry.disclosure.contains("convers"), "{}", entry.disclosure);
        assert!(!entry.disclosure.contains('\n'), "one line");
        assert!(
            !entry.expansion.is_empty(),
            "a read of the entry says something"
        );
        assert_eq!(catalog_entry(Mode::Work).name, "mode:work");
    }

    #[test]
    fn standing_adds_nothing_in_either_mode() {
        for mode in Mode::ALL {
            assert_eq!(
                admits(&RunPolicy::of(mode), &Produced::default()),
                Admission::Lands
            );
        }
    }

    #[test]
    fn tested_wants_the_asset_proven_and_says_which_way_it_failed() {
        let policy = work(AdmissionRequirement::Tested);
        let failed = Produced {
            tests_passed: Some(false),
            ..Produced::default()
        };
        let Admission::Refused { because, .. } = admits(&policy, &failed) else {
            panic!("a failing asset does not land");
        };
        assert!(
            because.contains("did not pass"),
            "not the untested sentence"
        );
        let passed = Produced {
            tests_passed: Some(true),
            ..Produced::default()
        };
        assert_eq!(admits(&policy, &passed), Admission::Lands);
    }

    #[test]
    fn contract_kept_refuses_a_renovation_that_moved_the_contract() {
        let policy = work(AdmissionRequirement::ContractKept);
        assert_eq!(admits(&policy, &Produced::default()), Admission::Lands);
        let moved = Produced {
            contract_moved: true,
            ..Produced::default()
        };
        let Admission::Refused { alternative, .. } = admits(&policy, &moved) else {
            panic!("a moved contract is not a renovation");
        };
        assert!(
            alternative.contains("double_validated"),
            "the refusal names the requirement that would take it"
        );
    }

    #[test]
    fn double_validated_takes_nothing_on_confidence() {
        let policy = work(AdmissionRequirement::DoubleValidated);
        let both = Produced {
            held_in: Some(true),
            held_out: Some(true),
            ..Produced::default()
        };
        assert_eq!(admits(&policy, &both), Admission::Lands);
        for missing in [
            Produced {
                held_in: Some(true),
                ..Produced::default()
            },
            Produced {
                held_out: Some(true),
                ..Produced::default()
            },
            Produced::default(),
        ] {
            assert!(
                matches!(admits(&policy, &missing), Admission::Refused { .. }),
                "half of a double validation is not a double validation"
            );
        }
        let regressed = Produced {
            held_in: Some(true),
            held_out: Some(false),
            ..Produced::default()
        };
        let Admission::Refused { because, .. } = admits(&policy, &regressed) else {
            panic!("a change that only holds where it was built does not land");
        };
        assert!(because.contains("held-out"));
    }

    #[test]
    fn an_experiment_lands_nothing_however_well_it_went() {
        let excellent = Produced {
            tests_passed: Some(true),
            contract_moved: false,
            held_in: Some(true),
            held_out: Some(true),
        };
        for admit in AdmissionRequirement::ALL {
            let policy = RunPolicy {
                landing: LandingPolicy::Experiment,
                write: kernel::WriteLimit::Create,
                ..work(admit)
            };
            assert!(matches!(
                admits(&policy, &excellent),
                Admission::Refused { .. }
            ));
        }
    }
}

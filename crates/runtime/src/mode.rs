// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Run modes and admission. A run sits in exactly one mode; the
//! catalog lists only that one (progressive disclosure — the other
//! mode's text stays out of the window).
//!
//! One exception, and it is one line long: [`dev_entry`] tells a run
//! that this city's own code is changeable, and names the evidence a
//! change can be asked to carry. Without it an agent working in this
//! city never learns that the city's own code and SPECs are changeable
//! at all, or under what discipline — and a capability nobody is told
//! about is one nobody uses.
//!
//! The half that decides is [`admits`]: it says whether what a run
//! produced may land, given the run policy it was dispatched under
//! (`crates/runtime/spec/Mode.lean` §8-54). The evidence arrives as plain answers rather
//! than as an instrument's type, because the instruments live in
//! citysim, outside this crate, and the question here is not how
//! evidence was gathered but whether enough of it exists.

use crate::catalog::CatalogEntry;
use kernel::{AdmissionRequirement, LandingPolicy, Mode, RunPolicy};
use std::sync::{Arc, PoisonError, RwLock};

/// The name of the catalog row that opens the developer discipline.
pub const DEV_ENTRY: &str = "dev";

/// The row that tells a run this city is changeable, and nothing more.
///
/// One line in the resident segment, and the whole discipline behind an
/// expansion. That split is the point: most sessions never change this
/// city, so they pay a line; a session that is about to change it asks
/// once and gets the evidence a change can be asked for, the reading
/// order and what to do next. The same progressive disclosure the tool
/// rows use, applied to the one capability an agent would otherwise
/// never learn it has.
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
        // Text this build holds, not a document on a shelf: there is
        // nothing behind it that could change while nobody is looking.
        hash: None,
        package: None,
    }
}

/// The catalog row for this mode: disclosure one-liner plus the
/// expansion text. The chat row is the whole of what chat mode does:
/// one line telling the resident that the person is talking with it.
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
        package: None,
    }
}

/// The tools a session in `mode` carries in its request; every other
/// admitted tool waits in the dormant index (`crates/runtime/Spec.lean`
/// §8-60). A name here travels only where the building admitted it: the
/// hall registers no `exec`, so `exec` is in no tier there.
///
/// The two doors of the truncation lock are in every mode, because a
/// dormant capability is reachable only through them. Chat answers the
/// person and reads what it is asked about; work also edits and runs
/// commands, which is what carrying out a task is made of. `status` is in
/// both because the city prompt sends every resident to it for its
/// situation.
#[must_use]
pub fn core_tools(mode: Mode) -> &'static [&'static str] {
    match mode {
        Mode::Chat => &[
            crate::tools::CallTool::NAME,
            crate::tools::DescribeTool::NAME,
            crate::tools::ReadTool::NAME,
            crate::tools::SearchTool::NAME,
            crate::tools::StatusTool::NAME,
        ],
        Mode::Work => &[
            crate::tools::CallTool::NAME,
            crate::tools::DescribeTool::NAME,
            crate::tools::EditTool::NAME,
            kernel::ToolName::EXEC,
            crate::tools::ReadTool::NAME,
            crate::tools::SearchTool::NAME,
            crate::tools::StatusTool::NAME,
        ],
    }
}

/// The run policy in force for one run (`crates/runtime/spec/PolicyTake.lean`
/// §8-62, runtime D28).
///
/// One writer and many readers. A change that reaches the run at any
/// safe point waits in the cell, and the run's driver loop puts the
/// last one in force at `SafePoint::BeforeWave` and nowhere else; the
/// write gates hold a [`PolicyReader`] that they ask once per write. So
/// every call of one wave is judged under one policy, the one in force
/// when the wave began.
#[derive(Debug)]
pub struct PolicyCell {
    shared: Arc<RwLock<RunPolicy>>,
    arrived: Option<RunPolicy>,
}

impl PolicyCell {
    /// The cell a run opens with, holding `run_started.policy`.
    #[must_use]
    pub fn new(start: RunPolicy) -> PolicyCell {
        PolicyCell {
            shared: Arc::new(RwLock::new(start)),
            arrived: None,
        }
    }

    /// A read-only view of the policy in force, for a write gate.
    #[must_use]
    pub fn reader(&self) -> PolicyReader {
        PolicyReader {
            shared: Arc::clone(&self.shared),
        }
    }

    /// A `run_policy_changed` reached the run at a safe point. It waits
    /// for the next `BeforeWave`; a later arrival overrides it.
    pub(crate) fn arrive(&mut self, policy: RunPolicy) {
        self.arrived = Some(policy);
    }

    /// At `SafePoint::BeforeWave`: puts the last arrival in force and
    /// hands it back so the driver can append the note that tells the
    /// model; `None` when nothing arrived, and then nothing is appended.
    pub(crate) fn take_at_wave(&mut self) -> Option<RunPolicy> {
        let taken = self.arrived.take()?;
        // A poisoned lock still holds a whole policy: the only write is
        // the assignment of a `Copy` value, which cannot stop halfway.
        *self.shared.write().unwrap_or_else(PoisonError::into_inner) = taken;
        Some(taken)
    }
}

/// What a write gate reads: the policy in force, never a change that
/// has arrived and not yet been taken.
#[derive(Debug, Clone)]
pub struct PolicyReader {
    shared: Arc<RwLock<RunPolicy>>,
}

impl PolicyReader {
    /// The policy in force now; asked once per write.
    #[must_use]
    pub fn now(&self) -> RunPolicy {
        *self.shared.read().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The note appended after a wave's results when a change was taken. It
/// is the only place a run's write limit is spelled to the model, so the
/// frozen prefix and the tool descriptions stay byte for byte as they
/// were sent (§8-62).
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

/// What a run has to show for itself.
///
/// `None` is not `Some(false)`: a suite that was never run and a suite
/// that failed are different facts, and a requirement that treated them
/// alike would let "we did not check" pass as "we checked and it was
/// fine".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Produced {
    /// The asset's own tests ran and passed.
    pub tests_passed: Option<bool>,
    /// The observable contract moved under the renovation.
    pub contract_moved: bool,
    /// Held-in evidence: it did not get worse on what it was built from.
    pub held_in: Option<bool>,
    /// Held-out evidence: it stands up away from what it was built from.
    pub held_out: Option<bool>,
}

/// Whether this may land, and if not, what to do about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Lands,
    Refused {
        because: &'static str,
        alternative: &'static str,
    },
}

/// Whether what a run produced may be merged, under the policy it was
/// dispatched with (`crates/runtime/spec/Mode.lean` §8-54).
///
/// The landing policy is asked first: an experiment lands nothing
/// however well it went. Then the evidence requirement, exhaustively,
/// so a new requirement has to say what it demands. The mode takes no
/// part: talk and work produce things that meet the same merge.
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

/// The evidence half of [`admits`].
///
/// `Standing` adds nothing: the building's own checks were made where
/// they are made. `Tested` wants the asset proven by its own tests.
/// `ContractKept` wants the contract to have stayed where it was,
/// because a renovation that moves it is not a renovation.
/// `DoubleValidated` wants both halves of the double validation: it is
/// asked for a change to how the city behaves, so confidence is not the
/// currency — evidence is.
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

    /// One thing a run meets, as `crates/runtime/spec/PolicyTake.lean`
    /// names it: a change reaching the run, a `BeforeWave`, or one call of
    /// the wave asking a write gate.
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

    /// The model's two properties over every trace: a wave's calls all
    /// read the policy in force when the wave began, and a `BeforeWave`
    /// takes the last change that arrived before it, and only then.
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

    /// The model's counterexample to reading the mailslot
    /// (`eager_reading_splits_a_wave`), kept as the trace that bit.
    #[test]
    fn a_change_inside_a_wave_leaves_that_wave_alone() {
        let start = RunPolicy::of(Mode::Work);
        let tighter = RunPolicy {
            write: kernel::WriteLimit::Create,
            ..start
        };
        walk(start, &[Step::Call, Step::Change(tighter), Step::Call]).unwrap();
    }

    fn work(admit: AdmissionRequirement) -> RunPolicy {
        RunPolicy {
            admit,
            ..RunPolicy::of(Mode::Work)
        }
    }

    /// A requirement chosen is not evidence held: work that asked for
    /// its own tests and ran none does not land.
    #[test]
    fn a_work_run_without_the_evidence_it_chose_does_not_land() {
        let Admission::Refused { because, .. } =
            admits(&work(AdmissionRequirement::Tested), &Produced::default())
        else {
            panic!("work that chose `tested` and ran no test landed");
        };
        assert!(because.contains("no tests"), "{because}");
    }

    /// A chat run is told one thing, to talk with the person.
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

    /// The building's own checks are made where they are made; the
    /// standing requirement adds none, in either mode.
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

    /// An experiment lands nothing, whatever it proved and whatever it
    /// was allowed to write.
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

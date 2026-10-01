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
//! (runtime-SPEC 8-54). The evidence arrives as plain answers rather
//! than as an instrument's type, because the instruments live in
//! citysim, outside this crate, and the question here is not how
//! evidence was gathered but whether enough of it exists.

use crate::catalog::CatalogEntry;
use kernel::{AdmissionRequirement, LandingPolicy, Mode, RunPolicy};

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
                    and says why.\n\nThe person chooses what a change must prove before it is \
                    merged:\n- tested: the asset's own tests ran and passed.\n- contract_kept: \
                    the asset's observable contract did not move.\n- double_validated: the change \
                    holds on held-in and on held-out evidence.\nAnd where it lands: \
                    ordinary work takes the building's own road, while an experiment works in a \
                    tree of its own and nothing in it is merged.\n\nYour next step: say which \
                    evidence this work needs and why, and wait for the person to dispatch it \
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
            "chat mode: focus on conversing with the person; answer what they said, in their language",
            "Reply in the conversation. Start work, plans or dispatches only when the person asks for them.",
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
/// dispatched with (runtime-SPEC 8-54).
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
        LandingPolicy::Ordinary => admits_evidence(AdmissionRequirement::Standing, produced),
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
}

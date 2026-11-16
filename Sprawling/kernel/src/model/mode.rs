// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The run policy: the four values one run works under (`crates/kernel/spec/Model.lean`
//! §8-77).
//!
//! Defined here rather than in `runtime` for the reason
//! [`DialectKind`](crate::DialectKind) is: the wire carries them and
//! `runtime` evaluates them, and neither of those crates may name the
//! other. `runtime::mode` holds what each value admits; this holds only
//! which values exist and how they are spelled.
//!
//! Every enum here is closed and carried in the spelling its `as_str`
//! writes. A word outside a set fails to deserialize at the process
//! boundary, which is where the sender can still be told; read as a
//! default, a misspelled word would become a run nobody asked for.

use serde::{Deserialize, Serialize};

use crate::write_domain::WriteLimit;

/// Whether a run is talking with the person or carrying out work.
///
/// It decides how the run's catalog row introduces it and nothing else:
/// what it may write, what it must prove and whether it lands are the
/// other three values of [`RunPolicy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Mode {
    /// Talk with the person: answer what they said. First, because a
    /// sentence typed into a conversation is talk before it is work.
    Chat,
    /// Carry out the task towards the goal the person stated.
    Work,
}

impl Mode {
    /// Every mode, in the order a control offers them.
    pub const ALL: [Mode; 2] = [Mode::Chat, Mode::Work];

    /// The word this mode travels under, everywhere it travels — the
    /// wire and the ledger.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Mode::Chat => "chat",
            Mode::Work => "work",
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The evidence a run's work must carry before a merge lets it into
/// the building.
///
/// Choosing one does not provide the evidence; `runtime::mode::admits`
/// compares what the run produced against it at the merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum AdmissionRequirement {
    /// What the building's own rules already demand, and nothing on
    /// top. A named value rather than an absence, because an absence
    /// reads as "no checks" and the building's checks still run.
    Standing,
    /// The asset's own tests ran and passed.
    Tested,
    /// The observable contract did not move.
    ContractKept,
    /// Both halves of the double validation: held-in and held-out.
    DoubleValidated,
}

impl AdmissionRequirement {
    /// Every requirement, from the one that adds nothing to the one
    /// that asks most.
    pub const ALL: [AdmissionRequirement; 4] = [
        AdmissionRequirement::Standing,
        AdmissionRequirement::Tested,
        AdmissionRequirement::ContractKept,
        AdmissionRequirement::DoubleValidated,
    ];

    /// The word this requirement travels under.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            AdmissionRequirement::Standing => "standing",
            AdmissionRequirement::Tested => "tested",
            AdmissionRequirement::ContractKept => "contract_kept",
            AdmissionRequirement::DoubleValidated => "double_validated",
        }
    }
}

/// Whether a run's work takes the ordinary road or stays in an
/// experiment that never lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum LandingPolicy {
    /// The building's own road: straight into its tree, or through
    /// review when the building asks for review.
    Ordinary,
    /// A worktree of the run's own, even in a building without review,
    /// from which nothing is ever merged.
    Experiment,
}

impl LandingPolicy {
    /// Every landing policy, the ordinary one first.
    pub const ALL: [LandingPolicy; 2] = [LandingPolicy::Ordinary, LandingPolicy::Experiment];

    /// The word this landing policy travels under.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            LandingPolicy::Ordinary => "ordinary",
            LandingPolicy::Experiment => "experiment",
        }
    }
}

/// The four values one run works under, chosen when it is dispatched
/// and written into its `run_started` line.
///
/// One value because the four always travel together — on the wire, in
/// the ledger and through the dispatch — and are chosen independently,
/// so every combination has a meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunPolicy {
    pub mode: Mode,
    /// What the run may do to a file that already exists.
    pub write: WriteLimit,
    /// The evidence its work must carry to be merged.
    pub admit: AdmissionRequirement,
    /// Whether its work takes the ordinary road.
    pub landing: LandingPolicy,
}

impl RunPolicy {
    /// The policy of a dispatch that chose only its mode: no narrowing
    /// of the write domain, the building's own checks, the ordinary
    /// road.
    #[must_use]
    pub const fn of(mode: Mode) -> RunPolicy {
        RunPolicy {
            mode,
            write: WriteLimit::Full,
            admit: AdmissionRequirement::Standing,
            landing: LandingPolicy::Ordinary,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    /// One spelling per enum: what `as_str` writes is what serde reads
    /// back, so a ledger payload and a wire frame cannot name one value
    /// two ways.
    #[test]
    fn every_value_reads_back_from_the_word_it_writes() {
        fn round_trips<T>(all: &[T], word: fn(T) -> &'static str)
        where
            T: Copy + PartialEq + std::fmt::Debug + Serialize + for<'de> Deserialize<'de>,
        {
            for value in all {
                let text = serde_json::to_string(value).unwrap();
                assert_eq!(text, format!("\"{}\"", word(*value)));
                assert_eq!(serde_json::from_str::<T>(&text).unwrap(), *value);
            }
        }
        round_trips(&Mode::ALL, Mode::as_str);
        round_trips(&AdmissionRequirement::ALL, AdmissionRequirement::as_str);
        round_trips(&LandingPolicy::ALL, LandingPolicy::as_str);
        round_trips(&WriteLimit::ALL, WriteLimit::as_str);
    }

    /// An unknown word is refused rather than read as some default,
    /// and so is a word this version no longer has.
    #[test]
    fn a_word_outside_the_set_is_refused() {
        assert!(serde_json::from_str::<Mode>("\"a-mode-we-have-never-heard-of\"").is_err());
        assert!(serde_json::from_str::<Mode>("\"plan_goal\"").is_err());
        assert!(serde_json::from_str::<AdmissionRequirement>("\"none\"").is_err());
        assert!(serde_json::from_str::<LandingPolicy>("\"merge\"").is_err());
    }

    /// A policy on the wire names all four values; one left out is a
    /// refused frame, not a default somebody did not choose.
    #[test]
    fn a_policy_missing_a_value_is_refused() {
        let whole: RunPolicy = serde_json::from_str(
            r#"{"mode":"work","write":"create","admit":"tested","landing":"experiment"}"#,
        )
        .unwrap();
        assert_eq!(
            whole,
            RunPolicy {
                mode: Mode::Work,
                write: WriteLimit::Create,
                admit: AdmissionRequirement::Tested,
                landing: LandingPolicy::Experiment,
            }
        );
        assert!(
            serde_json::from_str::<RunPolicy>(r#"{"mode":"work","write":"full","admit":"tested"}"#)
                .is_err()
        );
    }
}

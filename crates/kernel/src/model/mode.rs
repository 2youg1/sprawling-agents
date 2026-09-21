// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The discipline one run works under.

use serde::{Deserialize, Serialize};

/// Which discipline a run works under. A run sits in exactly one.
///
/// Closed, and carried on the wire in this spelling. A dispatch used to
/// name its mode as free text that the assembly layer matched against
/// four words and answered every other word with [`Mode::PlanGoal`], so
/// a client that misspelled `experiment` got a planning run and no
/// refusal. There is now nothing to misspell: a word outside this set
/// fails to deserialize at the process boundary, which is where the
/// sender can still be told.
///
/// Defined here rather than in `runtime` for the reason
/// [`DialectKind`](crate::DialectKind) is: the wire carries it and
/// `runtime` evaluates it, and neither of those crates may name the
/// other. `runtime::mode` holds what each one admits; this holds only
/// which ones exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Mode {
    /// Work out what to do and write it down; change nothing.
    PlanGoal,
    /// Build one asset that carries its own tests.
    Up,
    /// Renovate an existing asset without moving its observable
    /// contract.
    Sc,
    /// Change behaviour, carrying held-in and held-out evidence.
    Ud,
    /// Try something whose outcome nobody is promising.
    Experiment,
}

impl Mode {
    /// Every mode, in the order a control offers them.
    pub const ALL: [Mode; 5] = [
        Mode::PlanGoal,
        Mode::Up,
        Mode::Sc,
        Mode::Ud,
        Mode::Experiment,
    ];

    /// The word this mode travels under, everywhere it travels — the
    /// wire, the ledger and a person's configuration file.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Mode::PlanGoal => "plan_goal",
            Mode::Up => "up",
            Mode::Sc => "sc",
            Mode::Ud => "ud",
            Mode::Experiment => "experiment",
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    /// One spelling: what `as_str` writes is what serde reads back, so
    /// a ledger payload and a wire frame cannot name one mode two ways.
    #[test]
    fn every_mode_reads_back_from_the_word_it_writes() {
        for mode in Mode::ALL {
            let text = serde_json::to_string(&mode).unwrap();
            assert_eq!(text, format!("\"{}\"", mode.as_str()));
            assert_eq!(serde_json::from_str::<Mode>(&text).unwrap(), mode);
        }
    }

    /// The silent default this type exists to delete: an unknown word
    /// used to become a planning run.
    #[test]
    fn a_word_outside_the_set_is_refused_rather_than_read_as_planning() {
        assert!(serde_json::from_str::<Mode>("\"a-mode-we-have-never-heard-of\"").is_err());
        assert!(serde_json::from_str::<Mode>("\"plan\"").is_err());
    }
}

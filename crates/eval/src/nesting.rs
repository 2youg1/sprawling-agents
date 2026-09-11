// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which nested format a model edits with fewest mistakes, and how it
//! fails when it fails.
//!
//! **The product of this module is a number, not a preference.** The
//! plan tree has to live in a file a model edits every day, and three
//! formats were argued for on taste. Taste is not evidence: a format
//! that reads well and is edited wrongly one time in six is a worse
//! format than one nobody likes. So the question is settled by counting.
//!
//! **The failure distribution matters more than the rate.** A format
//! that fails by dropping a field is far worse here than one that fails
//! by mangling indentation, because a dropped field is a plan node that
//! silently stops existing while a broken indent refuses to parse. So a
//! run reports both: how often, and how.
//!
//! This grades a real edit against the text that came back. It does not
//! call a model — [`Attempt`] is what one produced — because a suite
//! that owned a provider could not run offline, could not be replayed,
//! and would be measuring the network as much as the model.

use std::collections::BTreeMap;

use kernel::{AxCode, AxError};

mod reading;

use reading::{read, stops_early};

/// A format the plan tree could be written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape {
    /// Nested tables, which is what the repository's configuration
    /// already uses.
    Toml,
    /// Nested objects, which every model has seen most of.
    Json,
    /// A nested list, which is what a person reads fastest.
    Markdown,
}

impl Shape {
    /// Every shape, in the order a report lists them.
    pub const ALL: [Shape; 3] = [Shape::Toml, Shape::Json, Shape::Markdown];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Toml => "toml",
            Self::Json => "json",
            Self::Markdown => "markdown",
        }
    }
}

/// How one edit went wrong. Exhaustive, and ordered by how much damage
/// each one does to a plan tree that nobody is watching.
///
/// The order is the finding this suite exists to produce: `LostField` is
/// first because it is the only one that leaves a *readable* file that
/// is missing a node, and a plan node that silently stops existing is
/// worse than a file that will not parse at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fault {
    /// The result parses and a field the original had is gone.
    LostField,
    /// The result parses and a value the edit was not asked to touch has
    /// changed.
    ChangedBystander,
    /// The result does not parse.
    Unparseable,
    /// The result stops early, mid-structure.
    Truncated,
    /// It parses, keeps every field, and the edit was not applied.
    NotApplied,
}

impl Fault {
    /// Every fault, worst first.
    pub const ALL: [Fault; 5] = [
        Fault::LostField,
        Fault::ChangedBystander,
        Fault::Unparseable,
        Fault::Truncated,
        Fault::NotApplied,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LostField => "lost a field",
            Self::ChangedBystander => "changed something it was not asked to",
            Self::Unparseable => "does not parse",
            Self::Truncated => "stops early",
            Self::NotApplied => "the edit is not there",
        }
    }
}

/// One real edit, and what a model returned for it.
///
/// `before` and `after` are the whole document, because the failure this
/// is looking for is what happened to the parts of the document nobody
/// asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub shape: Shape,
    /// The document as it was.
    pub before: String,
    /// The document as the model returned it.
    pub returned: String,
    /// The leaf paths the edit was allowed to change, as `a/b/c`.
    pub touching: Vec<String>,
    /// What those paths were supposed to become.
    pub wanted: BTreeMap<String, String>,
}

/// What one attempt came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub shape: Shape,
    /// `None` is a clean edit.
    pub fault: Option<Fault>,
}

/// Grades one attempt.
///
/// **Worst fault wins.** A result can be several kinds of wrong at once,
/// and reporting the mildest would flatter the format: a document that
/// lost a field *and* failed to apply the edit is counted as having lost
/// a field, because that is the one that survives into a plan tree.
///
/// # Errors
/// Refuses an attempt whose `before` this build cannot read. That is a
/// broken fixture rather than a model failure, and counting it as a
/// model failure would make the corpus grade itself.
pub fn grade(attempt: &Attempt) -> Result<Verdict, AxError> {
    let before = read(attempt.shape, &attempt.before).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "grade a nested edit",
            attempt.shape.as_str().to_owned(),
        )
        .with_recovery("the fixture's own `before` does not parse; fix the corpus, not the model")
    })?;
    let Some(after) = read(attempt.shape, &attempt.returned) else {
        // Told apart by whether the text stops inside the structure: a
        // model that ran out of tokens and one that wrote something
        // malformed call for different answers, and only one of them is
        // fixed by asking for more tokens.
        let fault = if stops_early(attempt.shape, &attempt.returned) {
            Fault::Truncated
        } else {
            Fault::Unparseable
        };
        return Ok(Verdict {
            shape: attempt.shape,
            fault: Some(fault),
        });
    };
    for path in before.keys() {
        if !after.contains_key(path) {
            return Ok(Verdict {
                shape: attempt.shape,
                fault: Some(Fault::LostField),
            });
        }
    }
    for (path, was) in &before {
        if attempt.touching.contains(path) {
            continue;
        }
        if after.get(path) != Some(was) {
            return Ok(Verdict {
                shape: attempt.shape,
                fault: Some(Fault::ChangedBystander),
            });
        }
    }
    for (path, wanted) in &attempt.wanted {
        if after.get(path) != Some(wanted) {
            return Ok(Verdict {
                shape: attempt.shape,
                fault: Some(Fault::NotApplied),
            });
        }
    }
    Ok(Verdict {
        shape: attempt.shape,
        fault: None,
    })
}

/// How one format did across a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grades {
    pub shape: Shape,
    pub tried: u32,
    pub wrong: u32,
    /// How it failed when it failed, worst first. Absent faults are
    /// absent rather than zero: a table of zeroes reads as a
    /// measurement, and these were not measured to be zero, they simply
    /// did not happen.
    pub faults: BTreeMap<Fault, u32>,
}

impl Grades {
    /// The rate, per mille. Per mille rather than a float, for the
    /// reason the rest of this crate uses it: an exact integer compares
    /// the same way twice.
    #[must_use]
    pub fn wrong_per_mille(&self) -> u32 {
        if self.tried == 0 {
            return 0;
        }
        self.wrong
            .saturating_mul(1000)
            .checked_div(self.tried)
            .unwrap_or(0)
    }
}

/// Every format's grades from one run, in [`Shape::ALL`] order.
///
/// A shape nobody tried is reported with `tried: 0` rather than left
/// out: a comparison missing one of its three columns is a comparison a
/// reader will misread as a clean sweep.
#[must_use]
pub fn tally(verdicts: &[Verdict]) -> Vec<Grades> {
    Shape::ALL
        .into_iter()
        .map(|shape| {
            let mine: Vec<&Verdict> = verdicts.iter().filter(|held| held.shape == shape).collect();
            let mut faults: BTreeMap<Fault, u32> = BTreeMap::new();
            for held in &mine {
                if let Some(fault) = held.fault {
                    let counted = faults.entry(fault).or_default();
                    *counted = counted.saturating_add(1);
                }
            }
            Grades {
                shape,
                tried: u32::try_from(mine.len()).unwrap_or(u32::MAX),
                wrong: faults.values().copied().fold(0u32, u32::saturating_add),
                faults,
            }
        })
        .collect()
}

/// The format to write the plan tree in, from one run's grades.
///
/// **Fewest mistakes wins, and a tie is broken by the worst fault each
/// one makes.** Two formats that fail equally often are not equally
/// good: the one whose failures leave a readable file with a node
/// missing costs more than the one whose failures refuse to parse,
/// because only one of them is noticed.
///
/// `None` when nothing was tried, which is not a recommendation to use
/// anything.
#[must_use]
pub fn recommended(grades: &[Grades]) -> Option<Shape> {
    grades
        .iter()
        .filter(|held| held.tried > 0)
        .min_by_key(|held| {
            // `Fault::ALL` is ordered worst first, so a lower position is
            // a worse failure and the winner is the one whose worst
            // failure sits *latest* in that list. Reversed for exactly
            // that reason: without it this picks the format that fails
            // most damagingly.
            let worst = Fault::ALL
                .iter()
                .position(|fault| held.faults.contains_key(fault))
                .unwrap_or(Fault::ALL.len());
            (held.wrong_per_mille(), std::cmp::Reverse(worst), held.shape)
        })
        .map(|held| held.shape)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;

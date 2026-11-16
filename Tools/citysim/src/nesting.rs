// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{AxCode, AxError};

mod reading;

use reading::{read, stops_early};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape {

    Toml,

    Json,

    Markdown,
}

impl Shape {

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fault {

    LostField,

    ChangedBystander,

    Unparseable,

    Truncated,

    NotApplied,
}

impl Fault {

    pub const ALL: [Fault; 5] = [
        Fault::LostField,
        Fault::ChangedBystander,
        Fault::Unparseable,
        Fault::Truncated,
        Fault::NotApplied,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub shape: Shape,

    pub before: String,

    pub returned: String,

    pub touching: Vec<String>,

    pub wanted: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub shape: Shape,

    pub fault: Option<Fault>,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grades {
    pub shape: Shape,
    pub tried: u32,
    pub wrong: u32,

    pub faults: BTreeMap<Fault, u32>,
}

impl Grades {

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

#[must_use]
pub fn recommended(grades: &[Grades]) -> Option<Shape> {
    grades
        .iter()
        .filter(|held| held.tried > 0)
        .min_by_key(|held| {

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

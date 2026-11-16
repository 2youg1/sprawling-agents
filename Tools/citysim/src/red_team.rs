// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use collab::{Citation, Reading};
use kernel::{B3Hash, Locator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arm {

    Unverified,

    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plant {
    Faithful,
    Misquote,
    OtherVersion,
    PastEnd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub citation: Citation,
    pub plant: Plant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub draft: String,
    pub claims: Vec<Claim>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub kept_faithful: usize,
    pub kept_planted: usize,
    pub dropped_faithful: usize,
    pub dropped_planted: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comparison {
    pub unverified: Tally,
    pub verified: Tally,
}

pub fn compare(cases: &[Case]) -> Comparison {
    Comparison {
        unverified: tally(cases, Arm::Unverified),
        verified: tally(cases, Arm::Verified),
    }
}

impl Tally {

    pub fn precision_per_mille(&self) -> Option<usize> {
        let kept = self.kept_faithful.checked_add(self.kept_planted)?;
        self.kept_faithful.checked_mul(1000)?.checked_div(kept)
    }

    fn count(&mut self, plant: Plant, kept: Kept) {
        let cell = match (kept, plant) {
            (Kept::Yes, Plant::Faithful) => &mut self.kept_faithful,
            (Kept::Yes, Plant::Misquote | Plant::OtherVersion | Plant::PastEnd) => {
                &mut self.kept_planted
            }
            (Kept::No, Plant::Faithful) => &mut self.dropped_faithful,
            (Kept::No, Plant::Misquote | Plant::OtherVersion | Plant::PastEnd) => {
                &mut self.dropped_planted
            }
        };
        *cell = cell.saturating_add(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kept {
    Yes,
    No,
}

fn tally(cases: &[Case], arm: Arm) -> Tally {
    let mut tally = Tally::default();
    for case in cases {
        let pinned = Locator::Cas {
            hash: B3Hash::digest(case.draft.as_bytes()),
            range: None,
        };
        for claim in &case.claims {
            tally.count(claim.plant, verdict(arm, claim, &pinned, &case.draft));
        }
    }
    tally
}

fn verdict(arm: Arm, claim: &Claim, pinned: &Locator, draft: &str) -> Kept {
    match arm {
        Arm::Unverified => Kept::Yes,
        Arm::Verified => match claim.citation.against(pinned, draft.as_bytes()) {
            Reading::Holds => Kept::Yes,
            Reading::OtherVersion | Reading::OutOfRange | Reading::Differs { .. } => Kept::No,
        },
    }
}

#[cfg(test)]
mod tests;

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one way a population decides what a value should have been.
//!
//! A population is sampled, the samples are gathered into clusters, and
//! the value the most members of a cluster hold is that cluster's fact;
//! every other member is off it by a distance the report carries. What
//! separates one population from another is [`Accord`] - whether the
//! design requires the members to agree, or whether a majority is what
//! makes one value the fact.
//!
//! **A correction is named where the page has a name for it.** Eight
//! pixels is not eight pixels if the page declares a step at eight: it
//! is one `snug` somebody applied twice, and saying so is the
//! difference between an edit a person makes and a number they have to
//! trace back to a word first.

use std::collections::BTreeMap;

use super::super::{Cohort, Declared, Deviation, Drawn, Finding, Population, SLACK};

/// How far past the agreement a reading in an observed population may
/// sit and still be a near miss.
///
/// Four pixels is a column that was laid out differently; three is a
/// number somebody typed twice. A tool that reported the first would
/// report every page as broken, which is how a person learns to ignore
/// it.
const REACH: i64 = 3;

/// The smallest population in which a majority means anything. Two
/// boxes that disagree are not a scale with an outlier; they are two
/// boxes.
const QUORUM: usize = 3;

/// Why the members of a population share a value.
#[derive(Clone, Copy)]
pub(super) enum Accord {
    /// The design requires them to agree, so any distance past the
    /// slack is a finding and no majority has to be established first.
    Required,
    /// Nothing requires them to agree. Only a majority makes one value
    /// the fact, and only a near miss is a mistake rather than a
    /// different column.
    Observed { reach: i64 },
}

impl Population {
    pub(super) fn accord(self) -> Accord {
        match self {
            Population::RegionsInTheMainColumn | Population::RowsOfANavigationColumn => {
                Accord::Required
            }
            Population::BoxesThatShareAHolder(_) => Accord::Observed { reach: REACH },
        }
    }
}

/// What one population is judged against: who is supposed to agree, and
/// the words the page declares for the distance a repair moves.
#[derive(Clone, Copy)]
pub(super) struct Against<'a> {
    pub(super) among: Population,
    pub(super) declared: &'a Declared,
}

/// The one way a population decides what a value should have been.
pub(super) fn agree<'a>(
    samples: &[(i64, &'a Drawn)],
    against: Against<'a>,
    out: &mut Vec<Deviation<'a>>,
) {
    let mut by_value: BTreeMap<i64, Vec<&'a Drawn>> = BTreeMap::new();
    for (reading, held) in samples {
        by_value.entry(*reading).or_default().push(held);
    }
    if by_value.len() < 2 {
        return;
    }
    match against.among.accord() {
        Accord::Required => required(&by_value, against, out),
        Accord::Observed { reach } => observed(by_value, against, reach, out),
    }
}

/// One agreement for the whole population: the value most of them hold,
/// and every member further from it than the slack.
fn required<'a>(
    by_value: &BTreeMap<i64, Vec<&'a Drawn>>,
    against: Against<'a>,
    out: &mut Vec<Deviation<'a>>,
) {
    let Some(anchor) = busiest(by_value) else {
        return;
    };
    let cohort = Cohort {
        with: within(by_value, anchor, SLACK),
        of: by_value.values().map(Vec::len).sum(),
    };
    for (reading, members) in by_value {
        if reading.abs_diff(anchor) <= SLACK.unsigned_abs() {
            continue;
        }
        tell(
            members,
            Off {
                against,
                is: *reading,
                should_be: anchor,
                cohort,
            },
            out,
        );
    }
}

/// As many agreements as the population turns out to have, each one a
/// cluster of readings within reach of the value most of them hold.
fn observed<'a>(
    mut by_value: BTreeMap<i64, Vec<&'a Drawn>>,
    against: Against<'a>,
    reach: i64,
    out: &mut Vec<Deviation<'a>>,
) {
    while let Some(anchor) = busiest(&by_value) {
        let near: Vec<i64> = by_value
            .keys()
            .copied()
            .filter(|reading| reading.abs_diff(anchor) <= reach.unsigned_abs())
            .collect();
        let mut cluster: BTreeMap<i64, Vec<&'a Drawn>> = BTreeMap::new();
        for reading in near {
            if let Some(members) = by_value.remove(&reading) {
                let replaced = cluster.insert(reading, members);
                debug_assert!(replaced.is_none(), "one value is inserted once");
            }
        }
        let of: usize = cluster.values().map(Vec::len).sum();
        // Aligned, not identical. `reach` decides whether two boxes are
        // in the same column at all; `SLACK` decides whether they line
        // up inside it, and they are different questions. This counted
        // only the boxes that matched the anchor exactly, so a column
        // whose members differed by the one pixel a border costs was
        // reported as a column with no agreement - and every reading in
        // it was printed. Eleven of the fifteen findings this
        // instrument carried on a clean tree were that, which is the
        // whole cost: a report a reader learns to skip is a report that
        // has stopped working.
        let with = within(&cluster, anchor, SLACK);
        if of < QUORUM || with.saturating_mul(2) <= of {
            continue;
        }
        for (reading, members) in &cluster {
            if reading.abs_diff(anchor) <= SLACK.unsigned_abs() {
                continue;
            }
            tell(
                members,
                Off {
                    against,
                    is: *reading,
                    should_be: anchor,
                    cohort: Cohort { with, of },
                },
                out,
            );
        }
    }
}

/// One value that is off its population's agreement: what was read,
/// what the population holds, and how many hold it.
#[derive(Clone, Copy)]
struct Off<'a> {
    against: Against<'a>,
    is: i64,
    should_be: i64,
    cohort: Cohort,
}

/// One reading per member of a value that is off the agreement.
fn tell<'a>(members: &[&'a Drawn], off: Off<'a>, out: &mut Vec<Deviation<'a>>) {
    let step = off
        .against
        .declared
        .spacing_named(off.should_be.saturating_sub(off.is).saturating_abs());
    for held in members {
        out.push(Deviation {
            at: held,
            finding: Finding::OutOfStep {
                among: off.against.among,
                is: off.is,
                should_be: off.should_be,
                cohort: off.cohort,
                step,
            },
        });
    }
}

/// The value the most members hold; the smallest of them when several
/// tie, so that two runs over one page report the same value.
fn busiest(by_value: &BTreeMap<i64, Vec<&Drawn>>) -> Option<i64> {
    by_value
        .iter()
        .max_by_key(|(reading, members)| (members.len(), std::cmp::Reverse(**reading)))
        .map(|(reading, _)| *reading)
}

/// How many members sit within a distance of one value.
fn within(by_value: &BTreeMap<i64, Vec<&Drawn>>, anchor: i64, distance: i64) -> usize {
    by_value
        .iter()
        .filter(|(reading, _)| reading.abs_diff(anchor) <= distance.unsigned_abs())
        .map(|(_, members)| members.len())
        .sum()
}

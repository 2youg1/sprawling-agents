// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use super::super::{Cohort, Declared, Deviation, Drawn, Finding, Population, SLACK};

const REACH: i64 = 3;

const QUORUM: usize = 3;

#[derive(Clone, Copy)]
pub(super) enum Accord {

    Required,

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

#[derive(Clone, Copy)]
pub(super) struct Against<'a> {
    pub(super) among: Population,
    pub(super) declared: &'a Declared,
}

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

#[derive(Clone, Copy)]
struct Off<'a> {
    against: Against<'a>,
    is: i64,
    should_be: i64,
    cohort: Cohort,
}

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

fn busiest(by_value: &BTreeMap<i64, Vec<&Drawn>>) -> Option<i64> {
    by_value
        .iter()
        .max_by_key(|(reading, members)| (members.len(), std::cmp::Reverse(**reading)))
        .map(|(reading, _)| *reading)
}

fn within(by_value: &BTreeMap<i64, Vec<&Drawn>>, anchor: i64, distance: i64) -> usize {
    by_value
        .iter()
        .filter(|(reading, _)| reading.abs_diff(anchor) <= distance.unsigned_abs())
        .map(|(_, members)| members.len())
        .sum()
}

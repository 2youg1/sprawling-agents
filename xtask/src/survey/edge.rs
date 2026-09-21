// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Alignment, and the one geometric property that is not alignment:
//! nothing is drawn outside the box that holds it.
//!
//! **There is one way of deciding what a value should have been**, and
//! [`agree`] is it. A population is sampled, the samples are gathered
//! into clusters, and the value the most members of a cluster hold is
//! that cluster's fact; every other member is off it by a distance the
//! report carries. Three populations are read through it, and what
//! separates them is [`Accord`] - whether the design requires the
//! members to agree, or whether a majority is what makes one value the
//! fact.
//!
//! **A correction is named where the page has a name for it.** Eight
//! pixels is not eight pixels if the page declares a step at eight: it
//! is one `snug` somebody applied twice, and saying so is the
//! difference between an edit a person makes and a number they have to
//! trace back to a word first.

use std::collections::{BTreeMap, BTreeSet};

use super::{Cohort, Declared, Deviation, Drawn, Finding, Overflow, Page, Population, SLACK, Side};

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
enum Accord {
    /// The design requires them to agree, so any distance past the
    /// slack is a finding and no majority has to be established first.
    Required,
    /// Nothing requires them to agree. Only a majority makes one value
    /// the fact, and only a near miss is a mistake rather than a
    /// different column.
    Observed { reach: i64 },
}

impl Population {
    fn accord(self) -> Accord {
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
struct Against<'a> {
    among: Population,
    declared: &'a Declared,
}

pub(super) fn every_population_agrees<'a>(page: &'a Page, out: &mut Vec<Deviation<'a>>) {
    regions_in_the_main_column(page, out);
    rows_of_each_navigation_column(page, out);
    for side in [Side::Left, Side::Right] {
        boxes_that_share_a_holder(page, side, out);
    }
}

/// Every region in the main column starts at the same x.
fn regions_in_the_main_column<'a>(page: &'a Page, out: &mut Vec<Deviation<'a>>) {
    let drawn = &page.drawn;
    let Some(main) = drawn.iter().position(|held| held.tag == "MAIN") else {
        return;
    };
    let Ok(index) = i64::try_from(main) else {
        return;
    };
    let samples: Vec<(i64, &Drawn)> = drawn
        .iter()
        .filter(|held| held.tag == "SECTION" && held.parent == index && held.shows())
        .map(|held| (held.left, held))
        .collect();
    agree(
        &samples,
        Against {
            among: Population::RegionsInTheMainColumn,
            declared: &page.declared,
        },
        out,
    );
}

/// Every clickable row in a navigation column starts its first mark at
/// the same x.
///
/// **A column, not a bar.** Rows that all share one top are a row of
/// tabs, and asking them to share an x would ask them to sit on top of
/// one another; a nav whose rows do not stack is left alone.
fn rows_of_each_navigation_column<'a>(page: &'a Page, out: &mut Vec<Deviation<'a>>) {
    let drawn = &page.drawn;
    for (position, _) in drawn
        .iter()
        .enumerate()
        .filter(|(_, held)| held.tag == "NAV" && held.drawn())
    {
        let Ok(index) = i64::try_from(position) else {
            continue;
        };
        let rows: Vec<&Drawn> = drawn
            .iter()
            .filter(|held| {
                held.operable()
                    && held.shows()
                    && held.first_mark >= 0
                    && descends(drawn, held, index)
            })
            .collect();
        let stacked: BTreeSet<i64> = rows.iter().map(|row| row.top).collect();
        if rows.len() < 2 || stacked.len() < 2 {
            continue;
        }
        let samples: Vec<(i64, &Drawn)> =
            rows.into_iter().map(|row| (row.first_mark, row)).collect();
        agree(
            &samples,
            Against {
                among: Population::RowsOfANavigationColumn,
                declared: &page.declared,
            },
            out,
        );
    }
}

/// Boxes laid out inside one container sit on one edge, or far enough
/// off it to be a different column.
///
/// The container is what makes this a population: one rule lays its
/// children out, so a child three pixels off the rest is a number
/// somebody typed rather than a decision somebody made. A box and its
/// own ancestor are not in one population, which is why the samples are
/// grouped by holder rather than swept off the whole page.
fn boxes_that_share_a_holder<'a>(page: &'a Page, side: Side, out: &mut Vec<Deviation<'a>>) {
    let mut families: BTreeMap<i64, Vec<(i64, &Drawn)>> = BTreeMap::new();
    for held in page
        .drawn
        .iter()
        .filter(|held| held.shows() && held.parent >= 0)
    {
        let reading = match side {
            Side::Left => held.left,
            Side::Right => held.right(),
            Side::Above => held.top,
            Side::Below => held.bottom(),
        };
        families
            .entry(held.parent)
            .or_default()
            .push((reading, held));
    }
    for samples in families.values() {
        agree(
            samples,
            Against {
                among: Population::BoxesThatShareAHolder(side),
                declared: &page.declared,
            },
            out,
        );
    }
}

/// The one way a population decides what a value should have been.
fn agree<'a>(samples: &[(i64, &'a Drawn)], against: Against<'a>, out: &mut Vec<Deviation<'a>>) {
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
        let with = cluster.get(&anchor).map_or(0, Vec::len);
        if of < QUORUM || with.saturating_mul(2) <= of {
            continue;
        }
        for (reading, members) in &cluster {
            if *reading == anchor {
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

/// Nothing is drawn outside the box that holds it.
///
/// This is the generalisation of the defect the render gate was written
/// for: a box that has floated out of its container is still painted,
/// still passes every test, and is the one thing a person sees
/// immediately.
pub(super) fn nothing_escapes_what_holds_it<'a>(drawn: &'a [Drawn], out: &mut Vec<Deviation<'a>>) {
    for held in drawn.iter().filter(|held| held.shows()) {
        let Some(holder) = parent_of(drawn, held) else {
            continue;
        };
        if !holder.shows() || holder.depth >= held.depth {
            continue;
        }
        // A container that does not show what it cannot fit paints
        // nothing outside itself, in the axis it does not show it: what
        // is below the fold of a scrolling column is reached by
        // scrolling, and what a clipping box cuts off was never
        // painted. The rule is about a box drawn over its neighbour, so
        // both are exempt and only `Shows` is judged.
        let across = holder.across == Overflow::Shows;
        let down = holder.down == Overflow::Shows;
        let sides = [
            (
                Side::Left,
                across && held.left.saturating_add(SLACK) < holder.left,
            ),
            (
                Side::Right,
                across && held.right() > holder.right().saturating_add(SLACK),
            ),
            (
                Side::Above,
                down && held.top.saturating_add(SLACK) < holder.top,
            ),
            (
                Side::Below,
                down && held.bottom() > holder.bottom().saturating_add(SLACK),
            ),
        ];
        if let Some((side, _)) = sides.into_iter().find(|(_, escapes)| *escapes) {
            out.push(Deviation {
                at: held,
                finding: Finding::Escapes { holder, side },
            });
        }
    }
}

fn parent_of<'a>(drawn: &'a [Drawn], held: &Drawn) -> Option<&'a Drawn> {
    usize::try_from(held.parent)
        .ok()
        .and_then(|at| drawn.get(at))
}

/// Whether an element sits under the measured element at `ancestor`.
fn descends(drawn: &[Drawn], held: &Drawn, ancestor: i64) -> bool {
    let mut at = held.parent;
    let mut steps = drawn.len();
    while at >= 0 {
        if at == ancestor {
            return true;
        }
        let Some(next) = usize::try_from(at).ok().and_then(|index| drawn.get(index)) else {
            return false;
        };
        if steps == 0 {
            return false;
        }
        steps = steps.saturating_sub(1);
        at = next.parent;
    }
    false
}

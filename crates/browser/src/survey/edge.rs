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

mod accord;

use accord::{Against, agree};

use super::{Deviation, Drawn, Finding, Overflow, Page, Population, SLACK, Side};

pub(super) fn every_population_agrees<'a>(page: &'a Page, out: &mut Vec<Deviation<'a>>) {
    regions_in_the_main_column(page, out);
    rows_of_each_navigation_column(page, out);
    boxes_that_share_a_holder(page, out);
}

/// The direction a container hands its children out in.
///
/// **A container may only be asked to agree on the axis it does not
/// distribute along.** Children stacked down a column share a left and
/// a right edge and are supposed to; children handed out across a row
/// share a top and a bottom. Asking a row to agree on its right edges
/// is a category error, and it is not a harmless one - two boxes in a
/// row that happen to end within a pixel or two of each other were
/// reported as a near miss, and every one of those was noise. Five of
/// the last five findings this instrument carried on a clean tree were
/// that.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    /// Children go down: compare the edges across.
    Column,
    /// Children go across: compare the edges down.
    Row,
}

impl Axis {
    /// Whether a container laid out this way is supposed to agree on
    /// this edge.
    ///
    /// **A column is asked about its left and right edges, and a row
    /// is asked about nothing.** The first half is the whole point of
    /// this reading: children stacked down a column share their
    /// inline edges and a child three pixels off them is a number
    /// somebody typed.
    ///
    /// The second half is a limit, and it was measured rather than
    /// assumed. A row does constrain its children in the other
    /// direction - they all sit in one band - but *where* in that band
    /// each one sits is `align-items`' decision, and under the centred
    /// alignment this library uses everywhere, two children of
    /// different heights have different top edges **by design**.
    /// Sweeping the cross axis of rows turned five false positives
    /// into fifteen: every one of them was a label beside a key, or a
    /// select beside its own caption, centred exactly as asked and
    /// reported as three pixels out.
    ///
    /// Reading the cross axis of a row correctly means comparing
    /// whichever of top, centre and bottom the majority actually
    /// holds, and that is a reading this instrument does not take yet.
    /// Saying nothing is the honest state until it does: a reader who
    /// learns to skip a section has lost the section.
    fn agrees_on(self, side: Side) -> bool {
        match (self, side) {
            (Axis::Column, Side::Left | Side::Right) => true,
            (Axis::Column, Side::Above | Side::Below) | (Axis::Row, _) => false,
        }
    }
}

/// Which way a container hands its children out, read off the boxes
/// rather than off the stylesheet.
///
/// Read from geometry because the question is about what the page did,
/// not about what it declared: a `flex-col` that wrapped and a grid
/// that placed two children on one line are both rows on the screen,
/// whatever their `display` says. Consecutive children in document
/// order are either stacked - the next one begins at or below the
/// bottom of the last - or handed out across. The majority decides,
/// and a container with no majority is one this reading says nothing
/// about.
fn stacking(children: &[(i64, &Drawn)]) -> Option<Axis> {
    let mut stacked: usize = 0;
    let mut across: usize = 0;
    for pair in children.windows(2) {
        let [(_, before), (_, after)] = pair else {
            continue;
        };
        if after.top >= before.bottom() {
            stacked = stacked.saturating_add(1);
        } else if after.left >= before.right() {
            across = across.saturating_add(1);
        }
    }
    if stacked > across {
        Some(Axis::Column)
    } else if across > stacked {
        Some(Axis::Row)
    } else {
        None
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
fn boxes_that_share_a_holder<'a>(page: &'a Page, out: &mut Vec<Deviation<'a>>) {
    let mut families: BTreeMap<i64, Vec<(i64, &'a Drawn)>> = BTreeMap::new();
    for held in page
        .drawn
        .iter()
        .filter(|held| held.shows() && held.parent >= 0)
    {
        // The reading is per side and is taken below; what a family
        // holds is its children in document order, because `stacking`
        // asks what consecutive pairs did.
        families.entry(held.parent).or_default().push((0, held));
    }
    for children in families.values() {
        let Some(axis) = stacking(children) else {
            continue;
        };
        for side in [Side::Left, Side::Right, Side::Above, Side::Below] {
            if !axis.agrees_on(side) {
                continue;
            }
            let samples: Vec<(i64, &'a Drawn)> = children
                .iter()
                .map(|(_, held)| {
                    let reading = match side {
                        Side::Left => held.left,
                        Side::Right => held.right(),
                        Side::Above => held.top,
                        Side::Below => held.bottom(),
                    };
                    (reading, *held)
                })
                .collect();
            agree(
                &samples,
                Against {
                    among: Population::BoxesThatShareAHolder(side),
                    declared: &page.declared,
                },
                out,
            );
        }
    }
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

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

mod accord;

use accord::{Against, agree};

use super::{Deviation, Drawn, Finding, Overflow, Page, Population, SLACK, Side};

pub(super) fn every_population_agrees<'a>(page: &'a Page, out: &mut Vec<Deviation<'a>>) {
    regions_in_the_main_column(page, out);
    rows_of_each_navigation_column(page, out);
    boxes_that_share_a_holder(page, out);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Column,

    Row,
}

impl Axis {
    fn agrees_on(self, side: Side) -> bool {
        match (self, side) {
            (Axis::Column, Side::Left | Side::Right) => true,
            (Axis::Column, Side::Above | Side::Below) | (Axis::Row, _) => false,
        }
    }
}

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

fn boxes_that_share_a_holder<'a>(page: &'a Page, out: &mut Vec<Deviation<'a>>) {
    let mut families: BTreeMap<i64, Vec<(i64, &'a Drawn)>> = BTreeMap::new();
    for held in page
        .drawn
        .iter()
        .filter(|held| held.shows() && held.parent >= 0)
    {
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

pub(super) fn nothing_escapes_what_holds_it<'a>(drawn: &'a [Drawn], out: &mut Vec<Deviation<'a>>) {
    for held in drawn.iter().filter(|held| held.shows()) {
        let Some(holder) = parent_of(drawn, held) else {
            continue;
        };
        if !holder.shows() || holder.depth >= held.depth {
            continue;
        }

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

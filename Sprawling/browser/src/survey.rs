// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod drawn;
mod echo;
mod edge;
mod legibility;
pub mod probe;
mod sheet;
mod source;
mod vocabulary;

pub use drawn::{Cut, Drawn, Marking, Overflow, Page, Paint, PaintSource, Sampled, TextRun};
pub use sheet::{Shape, Survey, remedy, rule, says, written};
pub use source::{DRAWN_IN, ENOUGH, Sources, literals};
pub use vocabulary::Declared;

const SHOWS_NOTHING: i64 = 1;

const SLACK: i64 = 1;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Population {

    RegionsInTheMainColumn,

    RowsOfANavigationColumn,

    BoxesThatShareAHolder(Side),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Left,
    Right,
    Above,
    Below,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Surface {

    Ink,

    Fill,
}

#[derive(Clone, Copy)]
pub struct Cohort {
    pub with: usize,
    pub of: usize,
}

pub struct Near<'a> {
    pub token: &'a str,
    pub apart: u16,
}

pub enum Finding<'a> {

    OutOfStep {
        among: Population,
        is: i64,
        should_be: i64,
        cohort: Cohort,

        step: Option<&'a str>,
    },

    Escapes { holder: &'a Drawn, side: Side },

    TextCut { shown: i64, needs: i64 },

    UndeclaredPaint {
        painted: Paint,
        on: Surface,
        nearest: Option<Near<'a>>,
    },

    UndeclaredSize {
        px_x100: u32,
        nearest: Option<Near<'a>>,
    },

    Echoed {

        other: &'a Drawn,

        text: &'a str,

        homes: usize,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Fact,
    Paint,
    Type,
    Text,
    Edge,
}

impl Group {

    pub const IN_ORDER: [Group; 5] = [
        Group::Fact,
        Group::Paint,
        Group::Type,
        Group::Text,
        Group::Edge,
    ];

    pub fn called(self) -> &'static str {
        match self {
            Group::Fact => "fact",
            Group::Paint => "paint",
            Group::Type => "type",
            Group::Text => "text",
            Group::Edge => "edge",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Refused,
    Noted,
}

pub struct Deviation<'a> {

    pub at: &'a Drawn,
    pub finding: Finding<'a>,
}

impl Deviation<'_> {

    pub fn standing(&self) -> Standing {
        match self.finding {
            Finding::Escapes { .. } => match self.at.sampled {
                Sampled::Frame => Standing::Refused,
                Sampled::Words => Standing::Noted,
            },
            Finding::OutOfStep { among, .. } => among.standing(),
            Finding::UndeclaredPaint { .. }
            | Finding::UndeclaredSize { .. }
            | Finding::TextCut { .. }
            | Finding::Echoed { .. } => Standing::Noted,
        }
    }
}

pub fn judge(page: &Page) -> Vec<Deviation<'_>> {
    let mut out = Vec::new();
    vocabulary::every_colour_is_a_declared_word(page, &mut out);
    vocabulary::every_size_is_a_declared_step(page, &mut out);
    legibility::no_box_cuts_its_own_text(&page.drawn, &mut out);
    echo::one_fact_has_one_home(&page.drawn, &mut out);
    edge::nothing_escapes_what_holds_it(&page.drawn, &mut out);
    edge::every_population_agrees(page, &mut out);

    out.sort_by(|left, right| {
        let seat = |one: &Deviation<'_>| {
            (
                one.finding.group(),
                one.finding.ordinal(),
                one.at.top,
                one.at.left,
            )
        };
        seat(left).cmp(&seat(right))
    });
    out
}

impl Finding<'_> {

    pub fn group(&self) -> Group {
        match *self {
            Finding::Echoed { .. } => Group::Fact,
            Finding::UndeclaredPaint { .. } => Group::Paint,
            Finding::UndeclaredSize { .. } => Group::Type,
            Finding::TextCut { .. } => Group::Text,
            Finding::OutOfStep { .. } | Finding::Escapes { .. } => Group::Edge,
        }
    }

    fn ordinal(&self) -> u8 {
        match *self {
            Finding::Echoed { .. } => 0,
            Finding::UndeclaredPaint { .. } => 1,
            Finding::UndeclaredSize { .. } => 2,
            Finding::TextCut { .. } => 3,
            Finding::Escapes { .. } => 4,
            Finding::OutOfStep { .. } => 5,
        }
    }

    pub fn id(&self) -> &'static str {
        match *self {
            Finding::Echoed { .. } => "fact.two-homes",
            Finding::UndeclaredPaint { .. } => "paint.undeclared",
            Finding::UndeclaredSize { .. } => "type.undeclared",
            Finding::TextCut { .. } => "text.cut",
            Finding::Escapes { .. } => "edge.escapes",
            Finding::OutOfStep { among, .. } => match among {
                Population::RegionsInTheMainColumn => "edge.main-column",
                Population::RowsOfANavigationColumn => "edge.nav-rows",
                Population::BoxesThatShareAHolder(_) => "edge.shares-holder",
            },
        }
    }
}

impl Population {
    fn standing(self) -> Standing {
        match self {
            Population::RegionsInTheMainColumn | Population::RowsOfANavigationColumn => {
                Standing::Refused
            }
            Population::BoxesThatShareAHolder(_) => Standing::Noted,
        }
    }
}

impl Side {

    pub fn of(self) -> &'static str {
        match self {
            Side::Left => "left edge",
            Side::Right => "right edge",
            Side::Above => "top edge",
            Side::Below => "bottom edge",
        }
    }
}

impl Surface {
    pub fn of(self) -> &'static str {
        match self {
            Surface::Ink => "the ink",
            Surface::Fill => "the fill",
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;

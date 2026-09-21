// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The grid instrument: what a page turns out to be once it has been
//! laid out (xtask-SPEC.md section 8-26).
//!
//! **It measures only what layout brings into existence.** Whether a
//! gap is on the scale and whether a colour is a token are decidable
//! from the source, and a type can refuse them before a browser is
//! opened; whether two boxes line up, whether a line of text survived
//! its container, and which of the declared words the page actually
//! painted are none of them knowable until the cascade has run. Those
//! three are what is here, and the definition names no stylesheet
//! language, which is why a second collector can fill the same record.
//!
//! **What it should be comes from the page, never from a design this
//! repository does not have.** Two sources, in this order: the
//! vocabulary the page declares about itself, read off the root element
//! as the engine resolved it, and - where nothing is declared - the
//! agreement of the population an element belongs to. Seven boxes at
//! x=56 and one at x=58 make 56 the fact and 58 the typing mistake, and
//! the report carries the count so that a reader can tell a typing
//! mistake from a column that never had a consensus.
//!
//! **A finding is an edit, not a complaint.** Every reading carries the
//! value to write, so the next step is one `edit` rather than a person
//! deciding which of two boxes moves.
//!
//! **Three of these readings were properties of the render gate.** The
//! left edge of the regions in the main column, the first mark of the
//! rows of a navigation column, and containment were three geometric
//! judgements written beside the gate that used them. Geometry with two
//! homes is the defect this instrument exists to prevent, so they were
//! moved here whole and the gate now reads them from here; what
//! separates a reading that stops a build from a reading a person acts
//! on is [`Standing`], stated once where each rule is written.

mod drawn;
mod echo;
mod edge;
mod legibility;
mod sheet;
mod source;
mod vocabulary;

pub(crate) use drawn::{Cut, Drawn, Marking, Overflow, Page, Paint, PaintSource, Sampled, TextRun};
pub(crate) use sheet::{Shape, Survey, remedy, rule, says, written};
pub(crate) use source::Sources;
pub(crate) use vocabulary::Declared;

/// A box no larger than this in either direction shows nothing to
/// anybody: it is the size a page clips a sentence to when the sentence
/// is for a screen reader alone.
const SHOWS_NOTHING: i64 = 1;

/// Two boxes may differ by this many pixels and still count as aligned.
///
/// Sub-pixel layout rounds, and a border that a rule paints on one side
/// only moves a box by one. Anything larger is a decision somebody made
/// twice.
const SLACK: i64 = 1;

/// The set a reading is compared against: who is supposed to agree, and
/// what the sentence is when they do not.
///
/// A population is the whole of the "should be" for geometry. The first
/// two are closed sets whose members the design requires to agree, so
/// any distance past the slack is a finding. The third is every box on
/// the page, where a large distance is a layout rather than a mistake,
/// so only near misses count.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Population {
    /// Every region in the main column, by its left edge.
    RegionsInTheMainColumn,
    /// Every clickable row of one navigation column, by the centre of
    /// the first mark inside it.
    RowsOfANavigationColumn,
    /// Every box one container lays out, by one of its four edges.
    BoxesThatShareAHolder(Side),
}

/// Which edge of a box a reading is about.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Side {
    Left,
    Right,
    Above,
    Below,
}

/// Which of an element's two colours a reading is about.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Surface {
    /// The colour its glyphs were painted in.
    Ink,
    /// The colour it fills its own box with.
    Fill,
}

/// How many of a population agreed on the value a reading should have
/// had.
///
/// Reported rather than hidden, because it decides whether a repair is
/// safe to make without asking: eleven of twelve is a typing mistake,
/// three of five is a column that never had a consensus, and one of two
/// is not a scale at all.
#[derive(Clone, Copy)]
pub(crate) struct Cohort {
    pub(crate) with: usize,
    pub(crate) of: usize,
}

/// The nearest word the page declares for a value it never declared.
pub(crate) struct Near<'a> {
    pub(crate) token: &'a str,
    pub(crate) apart: u16,
}

/// Every reading this instrument makes.
///
/// Exhaustive on purpose: a reading added here cannot be added without
/// saying which group it is repaired in and whether it stops a build.
pub(crate) enum Finding<'a> {
    /// A box out of step with the population it belongs to.
    OutOfStep {
        among: Population,
        is: i64,
        should_be: i64,
        cohort: Cohort,
        /// The word the page declares for the distance the repair
        /// moves, when it declares one. Eight pixels off a column is
        /// one `snug` applied twice, and a person edits a word.
        step: Option<&'a str>,
    },
    /// A box drawn outside the box that holds it.
    Escapes { holder: &'a Drawn, side: Side },
    /// Text its own box cut off.
    TextCut { shown: i64, needs: i64 },
    /// A colour the page painted and never declared.
    UndeclaredPaint {
        painted: Paint,
        on: Surface,
        nearest: Option<Near<'a>>,
    },
    /// A type size the page set and never declared.
    UndeclaredSize {
        px_x100: u32,
        nearest: Option<Near<'a>>,
    },
    /// One fact painted in two places that nothing links.
    Echoed {
        /// The other box saying the same thing.
        other: &'a Drawn,
        /// What both of them painted.
        text: &'a str,
        /// How many boxes said it, so a reader can tell a pair from a
        /// short column.
        homes: usize,
    },
}

/// The order a person repairs in, which is the order in which one
/// repair invalidates the next.
///
/// Paint moves no box, so it is free to go first. A type size moves the
/// lines under it. Text that was cut resizes the box that cut it. Every
/// alignment reading is downstream of all three, so a run that reported
/// alignment first would have a person fix a column, repaint it, and
/// find the column moved.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Group {
    Fact,
    Paint,
    Type,
    Text,
    Edge,
}

impl Group {
    /// Every group, in repair order. The array is the order.
    ///
    /// `Fact` is first and is not a geometry repair at all: giving two
    /// boxes one source deletes one of them or changes what it says,
    /// which moves every box after it. A run that repaired alignment
    /// first would align a box that is about to stop existing.
    pub(crate) const IN_ORDER: [Group; 5] = [
        Group::Fact,
        Group::Paint,
        Group::Type,
        Group::Text,
        Group::Edge,
    ];

    pub(crate) fn called(self) -> &'static str {
        match self {
            Group::Fact => "fact",
            Group::Paint => "paint",
            Group::Type => "type",
            Group::Text => "text",
            Group::Edge => "edge",
        }
    }
}

/// Whether a reading stops a build or is told to a person.
///
/// Three readings were properties of the render gate before this
/// instrument existed and remain properties of it. The readings the
/// instrument added are reported and gate nothing: an instrument that
/// turns a tree red on the day it is installed is an instrument
/// somebody switches off, and the repairs it asks for are a design
/// decision rather than a defect.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Standing {
    Refused,
    Noted,
}

/// One reading that is out of step, and everything a repair needs.
pub(crate) struct Deviation<'a> {
    /// The element that is out of step.
    pub(crate) at: &'a Drawn,
    pub(crate) finding: Finding<'a>,
}

impl Deviation<'_> {
    /// Whether this reading stops a build.
    ///
    /// Containment is the one reading whose standing depends on its
    /// subject as well as on itself: it refused the frame of the page
    /// before this instrument existed and goes on refusing it, and the
    /// boxes of words the instrument added to the sample are reported.
    pub(crate) fn standing(&self) -> Standing {
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

/// Every reading over one page, in repair order.
pub(crate) fn judge(page: &Page) -> Vec<Deviation<'_>> {
    let mut out = Vec::new();
    vocabulary::every_colour_is_a_declared_word(page, &mut out);
    vocabulary::every_size_is_a_declared_step(page, &mut out);
    legibility::no_box_cuts_its_own_text(&page.drawn, &mut out);
    echo::one_fact_has_one_home(&page.drawn, &mut out);
    edge::nothing_escapes_what_holds_it(&page.drawn, &mut out);
    edge::every_population_agrees(page, &mut out);
    // Stable, so two runs are a diff: repair order first, then the
    // reading, then where on the page it was found.
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
    /// The group this reading is repaired in.
    pub(crate) fn group(&self) -> Group {
        match *self {
            Finding::Echoed { .. } => Group::Fact,
            Finding::UndeclaredPaint { .. } => Group::Paint,
            Finding::UndeclaredSize { .. } => Group::Type,
            Finding::TextCut { .. } => Group::Text,
            Finding::OutOfStep { .. } | Finding::Escapes { .. } => Group::Edge,
        }
    }

    /// A stable seat within a group, so that two runs sort the same way.
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

    /// The stable key an agent suppresses, tracks or diffs on.
    ///
    /// Never the sentence: prose is written for a reader and gets
    /// rewritten whenever a reader is confused by it, and a key that
    /// moved every time somebody improved a sentence would be no key.
    pub(crate) fn id(&self) -> &'static str {
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
    /// What the reading is an edge of.
    pub(crate) fn of(self) -> &'static str {
        match self {
            Side::Left => "left edge",
            Side::Right => "right edge",
            Side::Above => "top edge",
            Side::Below => "bottom edge",
        }
    }
}

impl Surface {
    pub(crate) fn of(self) -> &'static str {
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

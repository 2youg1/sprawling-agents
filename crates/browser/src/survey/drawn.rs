// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The record: one element as something drew it, and the four small
//! vocabularies its fields are written in.
//!
//! **This is the boundary between collecting a page and judging one.**
//! Every judgement beside it is arithmetic over these fields and names
//! no document, no stylesheet and no window manager, so a second
//! collector - a desktop tree, a native view - fills the same record
//! and every reading holds. The fields are what the probe can answer
//! and what a reading needs, and nothing else earns a place here.

use std::fmt;

use super::{Declared, SHOWS_NOTHING, SLACK};

/// A colour as an engine painted it, after every alpha in front of it
/// has been composited away.
///
/// Eight bits a channel, because that is what a screen receives and
/// what `getComputedStyle` reports back; a wider representation here
/// would describe a colour nobody was shown.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Paint {
    red: u8,
    green: u8,
    blue: u8,
}

impl Paint {
    pub fn of(red: u8, green: u8, blue: u8) -> Paint {
        Paint { red, green, blue }
    }

    /// How far this colour sits from another, as the furthest any one
    /// channel travels.
    ///
    /// The largest channel distance rather than a perceptual metric:
    /// this number only ever orders candidates for "which declared
    /// token did somebody mean", and a nearest-token answer that is
    /// wrong by a shade is still the token they meant.
    pub(super) fn apart(self, other: Paint) -> u16 {
        let channels = [
            (self.red, other.red),
            (self.green, other.green),
            (self.blue, other.blue),
        ];
        channels
            .into_iter()
            .map(|(mine, theirs)| u16::from(mine.abs_diff(theirs)))
            .max()
            .unwrap_or(0)
    }
}

impl fmt::Display for Paint {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }
}

/// The text an element draws of its own, when it draws any.
pub struct TextRun {
    /// The colour the glyphs were painted in, or nothing when what is
    /// behind them cannot be resolved from the document.
    pub ink: Option<Paint>,
    /// The size they were set at, in hundredths of a pixel: the scale
    /// is declared in `calc()` against a density coefficient, so a step
    /// resolves to a fraction and a whole number would compare two
    /// different sizes as one.
    pub px_x100: u32,
    pub cut: Cut,
}

/// What became of the text a box could not fit.
///
/// Three outcomes rather than a flag, because the middle one is a
/// decision somebody made and the last one is a decision nobody made:
/// an ellipsis tells a person that something is being held back, and a
/// bare clip tells them nothing at all.
pub enum Cut {
    Fits,
    /// Cut, with a mark that says so.
    Marked,
    /// Cut, with no sign that anything is missing.
    Hidden {
        shown: i64,
        needs: i64,
    },
}

/// Whether the page draws a sign where it ran out of room, and whether
/// it ran out of room at all.
#[derive(Clone, Copy)]
pub enum Marking {
    Ellipsis,
    Clip,
    /// The box lets its contents show past its own edge, so nothing was
    /// hidden - what happens next is containment's reading, not this
    /// one's.
    Spills,
}

impl Cut {
    /// What the engine measured, read as one of the three outcomes.
    ///
    /// The slack is the same one alignment uses: a box one pixel
    /// narrower than its own text has rounded, not clipped.
    pub fn of(shown: i64, needs: i64, marking: Marking) -> Cut {
        if needs <= shown.saturating_add(SLACK) {
            return Cut::Fits;
        }
        match marking {
            Marking::Ellipsis => Cut::Marked,
            Marking::Spills => Cut::Fits,
            Marking::Clip => Cut::Hidden { shown, needs },
        }
    }
}

/// One element as an engine drew it.
///
/// The record is the boundary between collecting a page and judging one
/// (the design's layers ① and ③): a collector that reads a window
/// manager rather than a document fills the same fields, and every
/// judgement below is arithmetic over them.
pub struct Drawn {
    pub tag: String,
    pub role: String,
    pub name: String,
    pub left: i64,
    pub top: i64,
    pub width: i64,
    pub height: i64,
    pub depth: i64,
    /// Index of the nearest measured ancestor, or `-1` at the top.
    pub parent: i64,
    /// What this element does with contents larger than itself, across
    /// and down.
    pub across: Overflow,
    pub down: Overflow,
    pub sampled: Sampled,
    /// The centre x of the first painted element inside this one, or
    /// `-1` when it holds none. What a person's eye lands on first in a
    /// row: the dot, the glyph, the tick.
    pub first_mark: i64,
    /// Whether a line is drawn under this element's text, by its own
    /// rule or by one it inherits from a box that holds it.
    pub underlined: bool,
    /// The colour this element fills its own box with, or nothing when
    /// it fills it with nothing and whatever is behind shows through.
    pub fill: Option<Paint>,
    /// The class attribute, verbatim: the literal a reader greps for to
    /// reach the line that produced this element.
    pub class: String,
    pub text: Option<TextRun>,
}

impl Drawn {
    pub(super) fn right(&self) -> i64 {
        self.left.saturating_add(self.width)
    }

    pub(super) fn bottom(&self) -> i64 {
        self.top.saturating_add(self.height)
    }

    /// What a finding calls this element.
    pub fn called(&self) -> String {
        let tag = self.tag.to_lowercase();
        if self.anonymous() {
            return tag;
        }
        format!("{tag} `{}`", self.name)
    }

    /// An element with no area holds nothing on this page, and has no
    /// position worth comparing.
    pub fn drawn(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Whether this element shows anything a person could see.
    ///
    /// A box clipped to a single pixel is how a page hands a sentence
    /// to a screen reader and to no one else. It paints nothing, so it
    /// cannot be painted over its neighbour, it cannot have cut text
    /// anybody was reading, and it has no edge worth lining up. It is
    /// still a control that must say what it is, which is why this is
    /// a different question from having an area.
    pub fn shows(&self) -> bool {
        self.width > SHOWS_NOTHING && self.height > SHOWS_NOTHING
    }

    /// Controls a person operates, which are the ones that must be
    /// announceable.
    ///
    /// By role as well as by tag: this client draws a building as a `<g>`
    /// with `role="link"`, and an element that says it is a control is
    /// one, whatever it is made of.
    pub fn operable(&self) -> bool {
        matches!(
            self.tag.as_str(),
            "BUTTON" | "A" | "INPUT" | "TEXTAREA" | "SELECT"
        ) || matches!(
            self.role.as_str(),
            "button" | "link" | "textbox" | "checkbox" | "radio" | "tab" | "menuitem" | "switch"
        )
    }

    /// The regions a screen reader offers as a way to jump.
    pub fn landmark(&self) -> bool {
        matches!(
            self.tag.as_str(),
            "MAIN" | "NAV" | "ASIDE" | "HEADER" | "FOOTER"
        ) || matches!(
            self.role.as_str(),
            "navigation" | "main" | "complementary" | "banner" | "contentinfo" | "dialog"
        )
    }

    pub fn anonymous(&self) -> bool {
        self.name.is_empty() || self.name == "-"
    }
}

/// What put an element into the measurement.
///
/// **This is where the gate's reach is stated, once.** The render
/// gate's properties were written over the frame of the page, and this
/// instrument widened the sample to every box that carries words so
/// that paint and cut text could be read at all. A property that was
/// refusing one set must not start refusing a larger one on the day
/// the sample grew: the readings over the new elements are reported
/// until somebody acts on them, which is what [`Standing`] is for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sampled {
    /// A region, a control, a heading, or a box that does not show
    /// what it cannot fit.
    Frame,
    /// A box that carries words a person reads.
    Words,
}

/// What a box does with contents larger than itself, in one axis.
///
/// Three outcomes rather than the one question containment used to ask.
/// A box that scrolls and a box that clips both paint nothing outside
/// themselves, and the rule that exempted only the first reported every
/// clipped row as a box painted over its neighbour.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    /// Lets them show past its own edge, over whatever is beside it.
    Shows,
    /// Cuts them off at its edge, with no way to reach the rest.
    Clips,
    /// Cuts them off and gives a person a way to reach the rest.
    Scrolls,
}

/// Who chose the colours on the page that was measured.
///
/// A forced-colour mode replaces every colour the stylesheet declared
/// with the system's own. Asking whether the painted colours are the
/// declared ones then compares a page against a palette that was
/// deliberately overruled, and answers with the whole page.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaintSource {
    ThePage,
    TheSystem,
}

/// One page as an engine drew it: every element the instrument measures,
/// the vocabulary the page declares about itself, and whether the
/// colours on it are that vocabulary's.
pub struct Page {
    pub drawn: Vec<Drawn>,
    pub declared: Declared,
    pub painted_by: PaintSource,
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::fmt;

use super::{Declared, SHOWS_NOTHING, SLACK};

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

pub struct TextRun {

    pub ink: Option<Paint>,

    pub px_x100: u32,
    pub cut: Cut,
}

pub enum Cut {
    Fits,

    Marked,

    Hidden {
        shown: i64,
        needs: i64,
    },
}

#[derive(Clone, Copy)]
pub enum Marking {
    Ellipsis,
    Clip,

    Spills,
}

impl Cut {

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

pub struct Drawn {
    pub tag: String,
    pub role: String,
    pub name: String,
    pub left: i64,
    pub top: i64,
    pub width: i64,
    pub height: i64,
    pub depth: i64,

    pub parent: i64,

    pub across: Overflow,
    pub down: Overflow,
    pub sampled: Sampled,

    pub first_mark: i64,

    pub underlined: bool,

    pub fill: Option<Paint>,

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

    pub fn called(&self) -> String {
        let tag = self.tag.to_lowercase();
        if self.anonymous() {
            return tag;
        }
        format!("{tag} `{}`", self.name)
    }

    pub fn drawn(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    pub fn shows(&self) -> bool {
        self.width > SHOWS_NOTHING && self.height > SHOWS_NOTHING
    }

    pub fn operable(&self) -> bool {
        matches!(
            self.tag.as_str(),
            "BUTTON" | "A" | "INPUT" | "TEXTAREA" | "SELECT"
        ) || matches!(
            self.role.as_str(),
            "button" | "link" | "textbox" | "checkbox" | "radio" | "tab" | "menuitem" | "switch"
        )
    }

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sampled {

    Frame,

    Words,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Overflow {

    Shows,

    Clips,

    Scrolls,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaintSource {
    ThePage,
    TheSystem,
}

pub struct Page {
    pub drawn: Vec<Drawn>,
    pub declared: Declared,
    pub painted_by: PaintSource,
}

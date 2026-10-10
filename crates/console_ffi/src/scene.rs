// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a frame is drawn from, and the bytes the leaf reads it as
//! (`crates/console_ffi/Spec.lean` D2).
//!
//! The console says *what* is on the screen - a tool call and how long it
//! took, the words being typed and where the cursor stands in them - and
//! the leaf decides how it looks: the columns, the marks, the colours,
//! where a line wraps. So nothing here is laid out, and nothing in the
//! leaf decides what a record means.
//!
//! The encoding is little-endian, every text a `u32` length and its
//! bytes, every part a tag from [`Part`]. This module is the only writer,
//! so a scene the leaf cannot read is a defect here.

use crate::part::{Ending, Outcome, Part, Verdict};

/// The time of day a line carries: seconds since local midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay(u32);

impl TimeOfDay {
    /// A time of day, or nothing for a second count a day does not hold.
    pub fn from_seconds(seconds: u32) -> Option<TimeOfDay> {
        (seconds < 86_400).then_some(TimeOfDay(seconds))
    }
}

/// One line of the transcript, drawn once into the scrollback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// The first line: the city, and where its page is served.
    Banner {
        city: String,
        address: String,
        url: String,
    },
    /// A line the person submitted.
    You { at: Option<TimeOfDay>, said: String },
    /// A run of the chosen room beginning to answer: who, and the facts
    /// its requests froze.
    Head {
        at: Option<TimeOfDay>,
        resident: String,
        facts: Vec<String>,
    },
    /// The model reasoned before it answered; only the length is drawn.
    Reasoning { characters: u64 },
    /// A tool call that has answered.
    Tool {
        at: Option<TimeOfDay>,
        name: String,
        subject: String,
        took_us: Option<u64>,
        outcome: Outcome,
    },
    /// What the model said.
    Reply { said: String },
    /// A line from the console itself: an answer to a slash verb, a
    /// refusal, a notice.
    Note { said: String },
    /// A run of the chosen room ended.
    Ended {
        at: Option<TimeOfDay>,
        ending: Ending,
        took_s: Option<u64>,
    },
    /// A waiting request was answered.
    Resolved {
        at: Option<TimeOfDay>,
        verdict: Verdict,
        what: String,
    },
}

impl Entry {
    pub fn part(&self) -> Part {
        match self {
            Entry::Banner { .. } => Part::Banner,
            Entry::You { .. } => Part::You,
            Entry::Head { .. } => Part::Head,
            Entry::Reasoning { .. } => Part::Reasoning,
            Entry::Tool { .. } => Part::Tool,
            Entry::Reply { .. } => Part::Reply,
            Entry::Note { .. } => Part::Note,
            Entry::Ended { .. } => Part::Ended,
            Entry::Resolved { .. } => Part::Resolved,
        }
    }
}

/// The oldest request waiting for the person, and how many more do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waiting<'a> {
    pub what: &'a str,
    pub more: usize,
}

/// The run working in the chosen room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Working<'a> {
    pub resident: &'a str,
    pub since: Option<TimeOfDay>,
}

/// A tool call under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Calling<'a> {
    pub name: &'a str,
    pub subject: &'a str,
}

/// The line being typed: its text, the cursor as a character index into
/// it, what an empty line says, the room plain lines go to, and what
/// they ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Composer<'a> {
    pub typed: &'a str,
    pub cursor: usize,
    pub placeholder: &'a str,
    pub room: &'a str,
    pub offer: &'a str,
}

/// One entry of the slash menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice<'a> {
    pub spelling: &'a str,
    pub takes: &'a str,
    pub summary: &'a str,
}

/// The slash menu: the entries shown, which one the cursor is on, and
/// how many more there are past them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu<'a> {
    pub shown: Vec<Choice<'a>>,
    pub chosen: usize,
    pub more: usize,
}

/// The rows drawn again on every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Live<'a> {
    pub waiting: Option<Waiting<'a>>,
    pub working: Option<Working<'a>>,
    pub calling: Vec<Calling<'a>>,
    /// `/quit` asked how the runs under way end; how many there are.
    pub asking: Option<u64>,
    pub composer: Composer<'a>,
    pub menu: Option<Menu<'a>>,
}

/// One frame on the main screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inline<'a> {
    pub columns: u16,
    /// How many rows the cursor stands below the live region the last
    /// frame drew, which this frame erases first.
    pub erase: u16,
    /// The last transcript line drawn before this frame, which decides
    /// whether a blank line comes first.
    pub previous: Option<Part>,
    pub entries: &'a [Entry],
    pub live: Option<Live<'a>>,
}

/// One frame of the quiet host on the alternate screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quiet<'a> {
    pub columns: u16,
    pub rows: u16,
    pub url: &'a str,
    pub code: &'a str,
    pub key: Option<&'a str>,
    pub transient: Option<&'a str>,
}

/// A scene, written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scene(Vec<u8>);

impl Scene {
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    /// The scene of one frame on the main screen.
    pub fn inline(frame: &Inline<'_>) -> Scene {
        let mut scene = Scene::default();
        scene.tag(Part::Inline);
        scene.short(frame.columns);
        scene.short(frame.erase);
        scene.byte(frame.previous.map_or(0, Part::number));
        scene.count(frame.entries.len());
        for entry in frame.entries {
            scene.entry(entry);
        }
        if let Some(live) = &frame.live {
            scene.live(live);
        }
        scene
    }

    /// The scene of the quiet host.
    pub fn quiet(frame: &Quiet<'_>) -> Scene {
        let mut scene = Scene::default();
        scene.tag(Part::Quiet);
        scene.short(frame.columns);
        scene.short(frame.rows);
        scene.text(frame.url);
        scene.text(frame.code);
        scene.text(frame.key.unwrap_or_default());
        scene.text(frame.transient.unwrap_or_default());
        scene
    }

    fn entry(&mut self, entry: &Entry) {
        self.tag(entry.part());
        match entry {
            Entry::Banner { city, address, url } => {
                self.text(city);
                self.text(address);
                self.text(url);
            }
            Entry::You { at, said } => {
                self.time(*at);
                self.text(said);
            }
            Entry::Head {
                at,
                resident,
                facts,
            } => {
                self.time(*at);
                self.text(resident);
                self.count(facts.len());
                for fact in facts {
                    self.text(fact);
                }
            }
            Entry::Reasoning { characters } => self.long(Some(*characters)),
            Entry::Tool {
                at,
                name,
                subject,
                took_us,
                outcome,
            } => {
                self.time(*at);
                self.text(name);
                self.text(subject);
                self.long(*took_us);
                self.byte(outcome.number());
            }
            Entry::Reply { said } | Entry::Note { said } => self.text(said),
            Entry::Ended { at, ending, took_s } => {
                self.time(*at);
                self.byte(ending.number());
                self.long(*took_s);
            }
            Entry::Resolved { at, verdict, what } => {
                self.time(*at);
                self.byte(verdict.number());
                self.text(what);
            }
        }
    }

    fn live(&mut self, live: &Live<'_>) {
        if let Some(waiting) = live.waiting {
            self.tag(Part::Waiting);
            self.text(waiting.what);
            self.count(waiting.more);
        }
        if let Some(working) = live.working {
            self.tag(Part::Working);
            self.text(working.resident);
            self.time(working.since);
        }
        for calling in &live.calling {
            self.tag(Part::Calling);
            self.text(calling.name);
            self.text(calling.subject);
        }
        if let Some(runs) = live.asking {
            self.tag(Part::Asking);
            self.long(Some(runs));
        }
        let composer = live.composer;
        self.tag(Part::Composer);
        self.text(composer.typed);
        self.count(composer.cursor);
        self.text(composer.placeholder);
        self.text(composer.room);
        self.text(composer.offer);
        if let Some(menu) = &live.menu {
            self.tag(Part::Menu);
            self.count(menu.chosen);
            self.count(menu.more);
            self.count(menu.shown.len());
            for choice in &menu.shown {
                self.text(choice.spelling);
                self.text(choice.takes);
                self.text(choice.summary);
            }
        }
    }

    fn tag(&mut self, part: Part) {
        self.byte(part.number());
    }

    fn byte(&mut self, value: u8) {
        self.0.push(value);
    }

    fn short(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn word(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    /// A count the leaf reads as a `u32`; past that it reads the most a
    /// `u32` holds, which no screen reaches.
    fn count(&mut self, value: usize) {
        self.word(u32::try_from(value).unwrap_or(u32::MAX));
    }

    /// A number that may be absent, absent written as `u64::MAX`.
    fn long(&mut self, value: Option<u64>) {
        self.0
            .extend_from_slice(&value.unwrap_or(u64::MAX).to_le_bytes());
    }

    /// A time of day, absent written as `u32::MAX`.
    fn time(&mut self, at: Option<TimeOfDay>) {
        self.word(at.map_or(u32::MAX, |TimeOfDay(seconds)| seconds));
    }

    /// A text, cut at the most bytes a `u32` length counts; the leaf
    /// draws a sequence cut in the middle as one replacement character.
    fn text(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let length = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
        let kept = usize::try_from(length).map_or(bytes, |n| bytes.get(..n).unwrap_or(bytes));
        self.word(length);
        self.0.extend_from_slice(kept);
    }
}

#[cfg(test)]
impl Scene {
    /// A scene of whatever bytes a test needs, cut short or malformed.
    pub(crate) fn from_bytes_for_tests(bytes: &[u8]) -> Scene {
        Scene(bytes.to_vec())
    }
}

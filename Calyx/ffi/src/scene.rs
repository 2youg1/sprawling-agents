// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::part::{Ending, Outcome, Part, Verdict};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay(u32);

impl TimeOfDay {
    pub fn from_seconds(seconds: u32) -> Option<TimeOfDay> {
        (seconds < 86_400).then_some(TimeOfDay(seconds))
    }

    pub fn seconds(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Banner {
        city: String,
        address: String,
        url: String,
    },
    You {
        at: Option<TimeOfDay>,
        said: String,
    },
    Head {
        at: Option<TimeOfDay>,
        resident: String,
        facts: Vec<String>,
    },
    Reasoning {
        characters: u64,
    },
    Tool {
        at: Option<TimeOfDay>,
        name: String,
        subject: String,
        took_us: Option<u64>,
        outcome: Outcome,
    },
    Reply {
        said: String,
    },
    Note {
        said: String,
    },
    Ended {
        at: Option<TimeOfDay>,
        ending: Ending,
        took_s: Option<u64>,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Waiting<'a> {
    pub what: &'a str,
    pub more: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Working<'a> {
    pub resident: &'a str,
    pub since: Option<TimeOfDay>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Calling<'a> {
    pub name: &'a str,
    pub subject: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Composer<'a> {
    pub typed: &'a str,
    pub cursor: usize,
    pub placeholder: &'a str,
    pub room: &'a str,
    pub offer: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice<'a> {
    pub spelling: &'a str,
    pub takes: &'a str,
    pub summary: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu<'a> {
    pub shown: Vec<Choice<'a>>,
    pub chosen: usize,
    pub more: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Live<'a> {
    pub waiting: Option<Waiting<'a>>,
    pub working: Option<Working<'a>>,
    pub calling: Vec<Calling<'a>>,
    pub asking: Option<u64>,
    pub replying: Option<&'a str>,
    pub composer: Composer<'a>,
    pub menu: Option<Menu<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inline<'a> {
    pub columns: u16,
    pub rows: u16,
    pub erase: u16,
    pub previous: Option<Part>,
    pub entries: &'a [Entry],
    pub live: Option<Live<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quiet<'a> {
    pub columns: u16,
    pub rows: u16,
    pub url: &'a str,
    pub code: &'a str,
    pub key: Option<&'a str>,
    pub transient: Option<&'a str>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scene(Vec<u8>);

impl Scene {
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn inline(frame: &Inline<'_>) -> Scene {
        let mut scene = Scene::default();
        scene.tag(Part::Inline);
        scene.short(frame.columns);
        scene.short(frame.rows);
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

    pub fn entry_list(entries: &[Entry]) -> Scene {
        let mut scene = Scene::default();
        scene.count(entries.len());
        for entry in entries {
            scene.entry(entry);
        }
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
        if let Some(said) = live.replying {
            self.tag(Part::Replying);
            self.text(said);
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

    fn count(&mut self, value: usize) {
        self.word(u32::try_from(value).unwrap_or(u32::MAX));
    }

    fn long(&mut self, value: Option<u64>) {
        self.0
            .extend_from_slice(&value.unwrap_or(u64::MAX).to_le_bytes());
    }

    fn time(&mut self, at: Option<TimeOfDay>) {
        self.word(at.map_or(u32::MAX, |TimeOfDay(seconds)| seconds));
    }

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
    pub(crate) fn from_bytes_for_tests(bytes: &[u8]) -> Scene {
        Scene(bytes.to_vec())
    }
}

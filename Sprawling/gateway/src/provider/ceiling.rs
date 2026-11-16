// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::consts_policy::OUTPUT_CEILING_DEFAULT;
use kernel::event::record::ModelFacts;
use kernel::{Ceiling, DialectKind, Window};

use super::preset;
use crate::market::ModelEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigureSource {
    Person,

    Upstream,

    Registered,

    Preset,

    Policy,
}

impl FigureSource {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            FigureSource::Person => "person",
            FigureSource::Upstream => "upstream",
            FigureSource::Registered => "registered",
            FigureSource::Preset => "preset",
            FigureSource::Policy => "policy",
        }
    }

    #[must_use]
    pub fn read(word: &str) -> FigureSource {
        [
            FigureSource::Person,
            FigureSource::Upstream,
            FigureSource::Registered,
            FigureSource::Preset,
            FigureSource::Policy,
        ]
        .into_iter()
        .find(|source| source.as_str() == word)
        .unwrap_or(FigureSource::Registered)
    }

    const fn carried(self) -> bool {
        match self {
            FigureSource::Person | FigureSource::Registered => true,
            FigureSource::Upstream | FigureSource::Preset | FigureSource::Policy => false,
        }
    }
}

impl serde::Serialize for FigureSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for FigureSource {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let word = String::deserialize(deserializer)?;
        Ok(FigureSource::read(&word))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCeiling {
    Sent { tokens: Ceiling, from: FigureSource },

    ProviderDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowFigure {
    pub tokens: Window,
    pub from: FigureSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Figures {
    pub window: Option<WindowFigure>,

    pub ceiling: Option<OutputCeiling>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Statements<'a> {
    pub window: Option<Window>,

    pub ceiling: Option<Ceiling>,

    pub listed: Option<&'a ModelFacts>,

    pub registered: Option<&'a ModelEntry>,
}

enum Field {
    Required,
    Optional,
}

const fn field_on(wire: DialectKind) -> Field {
    match wire {
        DialectKind::Anthropic => Field::Required,
        DialectKind::OpenAi | DialectKind::OpenAiResponses => Field::Optional,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub base_url: &'a str,
    pub id: &'a str,
    pub wire: DialectKind,
}

#[derive(Debug, Clone, Copy)]
struct Stated<T> {
    person: Option<T>,
    upstream: Option<T>,
    registered: Option<T>,
}

impl<T: Copy> Stated<T> {
    fn first(self) -> Option<(T, FigureSource)> {
        self.person
            .map(|figure| (figure, FigureSource::Person))
            .or_else(|| self.upstream.map(|figure| (figure, FigureSource::Upstream)))
            .or_else(|| {
                self.registered
                    .map(|figure| (figure, FigureSource::Registered))
            })
    }
}

impl Figures {
    #[must_use]
    pub fn resolve(
        said: Statements<'_>,
        pinned: Option<&ModelEntry>,
        target: Target<'_>,
    ) -> Figures {
        let earlier = said.registered;
        let window = Stated {
            person: said.window,
            upstream: said
                .listed
                .and_then(|row| row.context_tokens)
                .and_then(Window::new),
            registered: earlier
                .and_then(|entry| carried(Window::new(entry.context_tokens), entry.window_from)),
        };
        let ceiling = Stated {
            person: said.ceiling,
            upstream: said.listed.and_then(|row| row.max_output_tokens),
            registered: earlier
                .and_then(|entry| carried(entry.max_output_tokens, entry.ceiling_from)),
        };
        Figures {
            window: window_from(
                window,
                pinned.and_then(|row| Window::new(row.context_tokens)),
                target,
            ),
            ceiling: OutputCeiling::resolve(
                ceiling,
                pinned.and_then(|row| row.max_output_tokens),
                target,
            ),
        }
    }
}

fn carried<T>(figure: Option<T>, from: Option<FigureSource>) -> Option<T> {
    figure.filter(|_| from.is_some_and(FigureSource::carried))
}

fn window_from(
    stated: Stated<Window>,
    pinned: Option<Window>,
    target: Target<'_>,
) -> Option<WindowFigure> {
    stated
        .first()
        .or_else(|| {
            pinned
                .or_else(|| preset::window_for(target.base_url, target.id))
                .map(|tokens| (tokens, FigureSource::Preset))
        })
        .map(|(tokens, from)| WindowFigure { tokens, from })
}

impl OutputCeiling {
    fn resolve(
        stated: Stated<Ceiling>,
        pinned: Option<Ceiling>,
        target: Target<'_>,
    ) -> Option<OutputCeiling> {
        let sent = |tokens, from| Some(OutputCeiling::Sent { tokens, from });
        if let Some((tokens, from)) = stated.first() {
            return sent(tokens, from);
        }
        match field_on(target.wire) {
            Field::Optional => Some(OutputCeiling::ProviderDefault),
            Field::Required => {
                match pinned.or_else(|| preset::ceiling_for(target.base_url, target.id)) {
                    Some(tokens) => sent(tokens, FigureSource::Preset),
                    None => sent(Ceiling::new(OUTPUT_CEILING_DEFAULT)?, FigureSource::Policy),
                }
            }
        }
    }

    #[must_use]
    pub const fn tokens(self) -> Option<Ceiling> {
        match self {
            OutputCeiling::Sent { tokens, .. } => Some(tokens),
            OutputCeiling::ProviderDefault => None,
        }
    }

    #[must_use]
    pub const fn source(self) -> Option<FigureSource> {
        match self {
            OutputCeiling::Sent { from, .. } => Some(from),
            OutputCeiling::ProviderDefault => None,
        }
    }

    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            OutputCeiling::Sent { from, .. } => from.as_str(),
            OutputCeiling::ProviderDefault => "provider",
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;

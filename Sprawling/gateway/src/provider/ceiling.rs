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
mod tests {
    use super::*;

    const ANTHROPIC: &str = "https://api.anthropic.com/v1";
    const RELAY: &str = "https://relay.example.test/v1";
    const LOCAL: &str = "http://127.0.0.1:8000/v1";
    const SONNET: &str = "claude-sonnet-4-5";

    fn messages(base_url: &'static str, id: &'static str) -> Target<'static> {
        Target {
            base_url,
            id,
            wire: DialectKind::Anthropic,
        }
    }

    fn sent(tokens: u64, from: FigureSource) -> OutputCeiling {
        OutputCeiling::Sent {
            tokens: Ceiling::new(tokens).unwrap(),
            from,
        }
    }

    fn tokens(count: u64) -> Option<Ceiling> {
        Ceiling::new(count)
    }

    fn stated(
        person: Option<Ceiling>,
        upstream: Option<Ceiling>,
        registered: Option<Ceiling>,
    ) -> Stated<Ceiling> {
        Stated {
            person,
            upstream,
            registered,
        }
    }

    const SILENT: Stated<Ceiling> = Stated {
        person: None,
        upstream: None,
        registered: None,
    };

    fn registration(id: &str, window: u64, ceiling: u64, from: FigureSource) -> ModelEntry {
        ModelEntry {
            id: id.to_owned(),
            context_tokens: window,
            window_from: Some(from),
            input: crate::InputKinds::Text,
            max_output_tokens: Ceiling::new(ceiling),
            ceiling_from: Some(from),
            input_price: kernel::UsdMicros::new(0),
            output_price: kernel::UsdMicros::new(0),
            cache_read_price: kernel::UsdMicros::new(0),
            cache_write_price: kernel::UsdMicros::new(0),
        }
    }

    fn listed(id: &str, window: u64, ceiling: u64) -> ModelFacts {
        ModelFacts {
            id: id.to_owned(),
            context_tokens: Some(window),
            max_output_tokens: Ceiling::new(ceiling),
            ..ModelFacts::default()
        }
    }

    #[test]
    fn the_higher_rung_wins_and_says_that_it_did() {
        let table = [
            (
                "a person outranks everything below",
                stated(tokens(1_000), tokens(2_000), tokens(2_500)),
                tokens(3_000),
                ANTHROPIC,
                1_000,
                FigureSource::Person,
            ),
            (
                "the provider's own list outranks an earlier registration",
                stated(None, tokens(2_000), tokens(2_500)),
                tokens(3_000),
                ANTHROPIC,
                2_000,
                FigureSource::Upstream,
            ),
            (
                "an earlier registration outranks anything this city pinned",
                stated(None, None, tokens(2_500)),
                tokens(3_000),
                ANTHROPIC,
                2_500,
                FigureSource::Registered,
            ),
            (
                "a pinned catalogue row answers when nobody stated one",
                SILENT,
                tokens(3_000),
                RELAY,
                3_000,
                FigureSource::Preset,
            ),
            (
                "the preset table answers for a documented host",
                SILENT,
                None,
                ANTHROPIC,
                64_000,
                FigureSource::Preset,
            ),
            (
                "policy answers last, for a server on this machine",
                SILENT,
                None,
                LOCAL,
                OUTPUT_CEILING_DEFAULT,
                FigureSource::Policy,
            ),
        ];
        for (why, stated, pinned, at, expected, from) in table {
            let got = OutputCeiling::resolve(stated, pinned, messages(at, SONNET));
            assert_eq!(got, Some(sent(expected, from)), "{why}");
        }
    }

    #[test]
    fn on_a_face_that_takes_no_figure_the_provider_picks_when_nobody_stated_one() {
        for wire in [DialectKind::OpenAi, DialectKind::OpenAiResponses] {
            let target = Target {
                base_url: ANTHROPIC,
                id: SONNET,
                wire,
            };
            let got = OutputCeiling::resolve(SILENT, tokens(3_000), target);
            assert_eq!(got, Some(OutputCeiling::ProviderDefault), "{wire:?}");
            let got = OutputCeiling::resolve(stated(None, tokens(2_000), None), None, target);
            assert_eq!(got, Some(sent(2_000, FigureSource::Upstream)), "{wire:?}");
        }
    }

    #[test]
    fn a_relay_forwarding_a_vendors_id_is_called_at_the_vendors_ceiling() {
        let got = OutputCeiling::resolve(SILENT, None, messages(RELAY, SONNET));
        assert_eq!(got, Some(sent(64_000, FigureSource::Preset)));
    }

    #[test]
    fn an_unknown_model_at_an_unknown_host_is_still_callable() {
        let got = OutputCeiling::resolve(SILENT, None, messages(RELAY, "some-relay-model"));
        assert_eq!(
            got,
            Some(sent(OUTPUT_CEILING_DEFAULT, FigureSource::Policy))
        );
        assert_eq!(got.map(OutputCeiling::word), Some("policy"));
    }

    #[test]
    fn each_rung_travels_under_one_word_and_reads_back_from_it() {
        let sources = [
            FigureSource::Person,
            FigureSource::Upstream,
            FigureSource::Registered,
            FigureSource::Preset,
            FigureSource::Policy,
        ];
        let words = sources.map(FigureSource::as_str);
        assert_eq!(
            words,
            ["person", "upstream", "registered", "preset", "policy"]
        );
        assert_eq!(words.map(FigureSource::read), sources);
        assert_eq!(
            FigureSource::read("stated-by-a-later-build"),
            FigureSource::Registered
        );
        assert_eq!(OutputCeiling::ProviderDefault.word(), "provider");
    }

    #[test]
    fn the_window_climbs_the_same_ladder_without_a_policy_rung() {
        let row = listed("relay-1", 131_072, 16_384);
        let by_person = registration("relay-1", 50_000, 5_000, FigureSource::Person);
        let by_table = registration(SONNET, 50_000, 5_000, FigureSource::Preset);
        let relay = Target {
            base_url: RELAY,
            id: "relay-1",
            wire: DialectKind::OpenAi,
        };
        let table = [
            (
                "the provider's list answers a pick that states nothing",
                Statements {
                    listed: Some(&row),
                    registered: Some(&by_person),
                    ..Statements::default()
                },
                relay,
                Some((131_072, FigureSource::Upstream)),
            ),
            (
                "what a person stated before answers when the list is silent",
                Statements {
                    registered: Some(&by_person),
                    ..Statements::default()
                },
                relay,
                Some((50_000, FigureSource::Registered)),
            ),
            (
                "a window the preset table stated before is the table's again",
                Statements {
                    registered: Some(&by_table),
                    ..Statements::default()
                },
                Target {
                    base_url: RELAY,
                    id: SONNET,
                    wire: DialectKind::OpenAi,
                },
                Some((200_000, FigureSource::Preset)),
            ),
            (
                "no rung knows the window of an unknown id on this machine",
                Statements::default(),
                Target {
                    base_url: LOCAL,
                    id: SONNET,
                    wire: DialectKind::OpenAi,
                },
                None,
            ),
        ];
        for (why, said, target, expected) in table {
            let got = Figures::resolve(said, None, target)
                .window
                .map(|figure| (figure.tokens.get(), figure.from));
            assert_eq!(got, expected, "{why}");
        }
    }

    use proptest::prelude::*;

    fn rung() -> impl proptest::strategy::Strategy<Value = Option<Ceiling>> {
        proptest::option::of((1u64..200_000).prop_map(|count| Ceiling::new(count).unwrap()))
    }

    fn source() -> impl proptest::strategy::Strategy<Value = FigureSource> {
        proptest::sample::select(vec![
            FigureSource::Person,
            FigureSource::Upstream,
            FigureSource::Registered,
            FigureSource::Preset,
            FigureSource::Policy,
        ])
    }

    proptest::proptest! {

        #[test]
        fn the_ceiling_ladder_keeps_the_lean_properties(
            person in rung(),
            upstream in rung(),
            earlier in proptest::option::of((rung(), source())),
            pinned in rung(),
            base_url in proptest::sample::select(vec![
                ANTHROPIC,
                RELAY,
                LOCAL,
                "https://api.openai.com/v1",
            ]),
            id in proptest::sample::select(vec![SONNET, "gpt-5", "some-unreleased-model"]),
            wire in proptest::sample::select(vec![
                DialectKind::Anthropic,
                DialectKind::OpenAi,
                DialectKind::OpenAiResponses,
            ]),
        ) {
            let listed = ModelFacts {
                id: id.to_owned(),
                max_output_tokens: upstream,
                ..ModelFacts::default()
            };
            let registered = earlier.map(|(figure, from)| ModelEntry {
                max_output_tokens: figure,
                ceiling_from: figure.map(|_| from),
                ..registration(id, 0, 1, from)
            });
            let pinned_row = pinned.map(|figure| ModelEntry {
                max_output_tokens: Some(figure),
                ..registration(id, 0, 1, FigureSource::Preset)
            });
            let target = Target { base_url, id, wire };
            let figures = |registered: Option<&ModelEntry>| {
                Figures::resolve(
                    Statements {
                        ceiling: person,
                        listed: Some(&listed),
                        registered,
                        ..Statements::default()
                    },
                    pinned_row.as_ref(),
                    target,
                )
                .ceiling
            };
            let answer = figures(registered.as_ref());
            let needs_figure = matches!(field_on(wire), Field::Required);

            let sent = matches!(answer, Some(OutputCeiling::Sent { .. }));
            proptest::prop_assert!(!needs_figure || sent);

            if let Some(OutputCeiling::Sent {
                from: FigureSource::Preset | FigureSource::Policy,
                ..
            }) = answer
            {
                proptest::prop_assert!(needs_figure);
            }

            if let Some((_, from)) = earlier
                && !from.carried()
            {
                proptest::prop_assert_eq!(answer, figures(None));
            }
        }
    }
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which statement about one model's context window and output ceiling
//! wins, and who made it (shape 1 decision; `crates/gateway/spec/Provider/Ceiling.lean`,
//! interface in `crates/gateway/spec/Provider.lean` §8-17, gateway D45).
//!
//! **A model with no registered ceiling could not be called at all.**
//! The Anthropic wire requires `max_tokens` in every request and refuses
//! to write one without it, so any id the pinned catalogue did not know
//! reached a dead end; the OpenAI wire omitted the field instead, so the
//! same city truncated answers differently on two providers for one
//! missing figure.
//!
//! The ladder below always answers, and it records which rung answered.
//!
//! **Where the face does not need a figure, the city states none.** The
//! chat and responses faces take the ceiling as an optional field, and
//! each vendor's default for it follows the model: a thinking model is
//! left tens of thousands of tokens, and a vendor that refuses a request
//! whose input and ceiling together pass the window never refuses its
//! own default. A number of this city's on those faces was either lower
//! than the vendor's or a refusal once the conversation grew, so there
//! the pinned and policy rungs stay silent and the field is left out.
//! A truncated run is then read off the account rather than guessed at:
//! `model_selected` carries `ceiling_from`, which is the reply to the
//! objection this city wrote against itself in `anthropic.rs` — that a
//! ceiling invented at the call site truncates runs for a reason that
//! appears nowhere in the account.
//!
//! **The window climbs the same ladder**, without the policy rung and
//! on every face alike: it never reaches the wire, and a window no rung
//! states stays unknown, so the context reminder stays silent rather
//! than counting against a guess.

use kernel::consts_policy::OUTPUT_CEILING_DEFAULT;
use kernel::event::record::ModelFacts;
use kernel::{Ceiling, DialectKind, Window};

use super::preset;
use crate::market::ModelEntry;

/// Who stated a figure about one model: its context window or its
/// output ceiling.
///
/// Exhaustive and ordered: the first variant outranks the second, and
/// so on down. The spellings are the ones that reach the ledger, so a
/// person reading a run sees the same words the code decides by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigureSource {
    /// A figure the person entered against this model. A decision, not
    /// an inference, which is why nothing outranks it.
    Person,
    /// A figure the provider's own model list stated.
    Upstream,
    /// A figure this tag was registered with when it last pointed at
    /// this same model, kept because an empty box is not an erased one:
    /// one a person stated then, or one an older line carries without
    /// naming who stated it.
    Registered,
    /// A figure this city pinned from the vendor's documentation —
    /// the pinned catalogue by exact id, or `provider::preset` by host
    /// and id prefix.
    Preset,
    /// No statement existed and the city used
    /// [`OUTPUT_CEILING_DEFAULT`]. Never the source of a window.
    Policy,
}

impl FigureSource {
    /// The word this source travels under in the ledger and on the
    /// wire. One spelling, written here, read everywhere.
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

    /// The source a ledger line names for a figure it carries.
    ///
    /// A word this build cannot name — one a later build wrote — reads
    /// as [`FigureSource::Registered`], as a figure whose line names no
    /// source at all does: the figure was this tag's registration, and
    /// who stated it is the one thing the line cannot say (gateway D45).
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

    /// Whether an earlier registration's figure from this source still
    /// answers at the earlier-registration rung (gateway D45).
    ///
    /// Only what a person stated is carried: the provider's list and
    /// this city's tables are asked again at their own rungs, so a
    /// figure they stated once is never kept above what they state now.
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

/// The ceiling one call is made with.
///
/// A figure and the rung it came from travel together, because a caller
/// holding only the number cannot say why a run stopped where it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCeiling {
    /// A figure every request states, and who stated it.
    Sent { tokens: Ceiling, from: FigureSource },
    /// Nobody stated one and the face takes none: the request leaves the
    /// field out and the provider applies its own default for the model.
    ProviderDefault,
}

/// A context window and the rung that stated it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowFigure {
    pub tokens: Window,
    pub from: FigureSource,
}

/// What one model is registered with: its window and its ceiling, each
/// with the rung that answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Figures {
    /// `None` when no rung states a window: the window is unknown.
    pub window: Option<WindowFigure>,
    /// `None` only when the policy default itself is zero, a state the
    /// policy module's own test forbids.
    pub ceiling: Option<OutputCeiling>,
}

/// What was said about one model before this city's own tables are
/// read.
#[derive(Debug, Clone, Copy, Default)]
pub struct Statements<'a> {
    /// The window the person entered now, if any.
    pub window: Option<Window>,
    /// The ceiling the person entered now, if any.
    pub ceiling: Option<Ceiling>,
    /// The row the provider's own model list wrote for this id, as the
    /// endpoint book holds it.
    pub listed: Option<&'a ModelFacts>,
    /// What this tag was registered with when it last pointed at this
    /// same model behind this same endpoint.
    pub registered: Option<&'a ModelEntry>,
}

/// Whether every request on a face must state a ceiling.
enum Field {
    Required,
    Optional,
}

/// The messages face refuses a request without `max_tokens`; the chat
/// and responses faces take their ceiling field as optional.
const fn field_on(wire: DialectKind) -> Field {
    match wire {
        DialectKind::Anthropic => Field::Required,
        DialectKind::OpenAi | DialectKind::OpenAiResponses => Field::Optional,
    }
}

/// The model a call is made to, as the ladder asks about it: where it
/// is served, its id, and the face its requests are written on.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub base_url: &'a str,
    pub id: &'a str,
    pub wire: DialectKind,
}

/// The three rungs above this city's own tables, for one figure.
///
/// They arrive together because they are read at the same moment, and
/// passing them apart gave chances to apply them in the wrong order.
#[derive(Debug, Clone, Copy)]
struct Stated<T> {
    person: Option<T>,
    upstream: Option<T>,
    registered: Option<T>,
}

impl<T: Copy> Stated<T> {
    /// The highest of the three rungs that states a figure.
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
    /// The window and the ceiling one model is registered with: the one
    /// ladder the pick point and every query about a model share.
    ///
    /// `pinned` is the pinned catalogue's row for this exact id. The
    /// caller builds the catalogue, so a query asking about a whole
    /// model list builds it once rather than once per id; the preset
    /// table is asked with the target's base URL and id, and the two are
    /// the same rung reached through two indexes.
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

/// An earlier registration's figure, when its source says it answers at
/// the earlier-registration rung (`FigureSource::carried`).
fn carried<T>(figure: Option<T>, from: Option<FigureSource>) -> Option<T> {
    figure.filter(|_| from.is_some_and(FigureSource::carried))
}

/// The window ladder: the three stated rungs, then the pinned
/// catalogue, then the preset row, and otherwise unknown.
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
    /// The ceiling one call is made with, and who supplied it.
    ///
    /// `None` says every rung stayed silent on a face that needs a
    /// figure, which requires [`OUTPUT_CEILING_DEFAULT`] itself to be
    /// zero — a state the policy module's own test forbids. Returning it
    /// rather than substituting a number keeps this decision free of a
    /// figure it invented.
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

    /// The figure a request states, or `None` when it states none.
    #[must_use]
    pub const fn tokens(self) -> Option<Ceiling> {
        match self {
            OutputCeiling::Sent { tokens, .. } => Some(tokens),
            OutputCeiling::ProviderDefault => None,
        }
    }

    /// The rung that stated the figure, or `None` when no figure is
    /// sent.
    #[must_use]
    pub const fn source(self) -> Option<FigureSource> {
        match self {
            OutputCeiling::Sent { from, .. } => Some(from),
            OutputCeiling::ProviderDefault => None,
        }
    }

    /// The word `model_selected.ceiling_from` records: the rung that
    /// stated the figure, or `provider` when no figure is sent.
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

    /// A registration of `id` whose two figures came from `from`.
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

    /// The row a provider's list writes for `id`, stating both figures.
    fn listed(id: &str, window: u64, ceiling: u64) -> ModelFacts {
        ModelFacts {
            id: id.to_owned(),
            context_tokens: Some(window),
            max_output_tokens: Ceiling::new(ceiling),
            ..ModelFacts::default()
        }
    }

    /// The priority table on the messages face, which needs a figure in
    /// every request, one row per pair of rungs that can both answer.
    /// Read it downward: whatever is stated higher wins, and the rung
    /// that won is named in the record.
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

    /// The chat and responses faces take the ceiling as optional, and
    /// a vendor's own default for it follows the model: nobody's
    /// statement means no figure on the wire, whatever this city pinned.
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

    /// A relay forwarding a vendor's id gets the vendor's documented
    /// ceiling on the messages face rather than the policy figure, which
    /// has nothing to do with either of them.
    #[test]
    fn a_relay_forwarding_a_vendors_id_is_called_at_the_vendors_ceiling() {
        let got = OutputCeiling::resolve(SILENT, None, messages(RELAY, SONNET));
        assert_eq!(got, Some(sent(64_000, FigureSource::Preset)));
    }

    /// The whole point of the ladder: an id no table knows, on the wire
    /// that requires the field, now has a figure to send.
    #[test]
    fn an_unknown_model_at_an_unknown_host_is_still_callable() {
        let got = OutputCeiling::resolve(SILENT, None, messages(RELAY, "some-relay-model"));
        assert_eq!(
            got,
            Some(sent(OUTPUT_CEILING_DEFAULT, FigureSource::Policy))
        );
        assert_eq!(got.map(OutputCeiling::word), Some("policy"));
    }

    /// Every source reads back from the word it is written under, and a
    /// word this build cannot name reads as the earlier registration.
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

    /// The window ladder at the pick point: the provider's list answers
    /// for a model the person left empty, an earlier registration a
    /// person stated answers when the list says nothing, and a window a
    /// table stated at an earlier pick is asked of the table again.
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
        /// The three properties of `crates/gateway/spec/Provider/Ceiling.lean`
        /// over the model's whole input space: each of the person, the
        /// provider's list, an earlier registration from each of the five
        /// sources and the pinned catalogue silent or stating a figure,
        /// the preset table answering or not (a vendor host, a relay,
        /// this machine, and an id nobody documents), on all three
        /// faces. Driven through `Figures::resolve`, the door the pick
        /// point uses.
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
            // a_face_that_needs_a_figure_always_gets_one
            let sent = matches!(answer, Some(OutputCeiling::Sent { .. }));
            proptest::prop_assert!(!needs_figure || sent);
            // the_city_states_a_figure_only_where_the_face_needs_one
            if let Some(OutputCeiling::Sent {
                from: FigureSource::Preset | FigureSource::Policy,
                ..
            }) = answer
            {
                proptest::prop_assert!(needs_figure);
            }
            // an_earlier_figure_nobody_stated_never_answers
            if let Some((_, from)) = earlier
                && !from.carried()
            {
                proptest::prop_assert_eq!(answer, figures(None));
            }
        }
    }
}

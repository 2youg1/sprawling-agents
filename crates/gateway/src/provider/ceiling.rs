// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which statement about one model's output ceiling wins, and who made
//! it (shape 1 decision; `crates/gateway/spec/Provider/Ceiling.lean`, interface
//! in `crates/gateway/spec/Provider.lean` §8-17).
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

use kernel::consts_policy::OUTPUT_CEILING_DEFAULT;
use kernel::{Ceiling, DialectKind};

use super::preset;

/// Who stated the ceiling a call was made with.
///
/// Exhaustive and ordered: the first variant outranks the second, and
/// so on down. The spellings are the ones that reach the ledger, so a
/// person reading a run sees the same four words the code decides by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CeilingSource {
    /// A figure the person entered against this model. A decision, not
    /// an inference, which is why nothing outranks it.
    Person,
    /// A figure the provider's own model list stated.
    Upstream,
    /// A figure this city pinned from the vendor's documentation —
    /// the pinned catalogue by exact id, or `provider::preset` by host
    /// and id prefix.
    Preset,
    /// No statement existed and the city used
    /// [`OUTPUT_CEILING_DEFAULT`].
    Policy,
}

impl CeilingSource {
    /// The word this source travels under in the ledger and on the
    /// wire. One spelling, written here, read everywhere.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            CeilingSource::Person => "person",
            CeilingSource::Upstream => "upstream",
            CeilingSource::Preset => "preset",
            CeilingSource::Policy => "policy",
        }
    }
}

/// The ceiling one call is made with.
///
/// A figure and the rung it came from travel together, because a caller
/// holding only the number cannot say why a run stopped where it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputCeiling {
    /// A figure every request states, and who stated it.
    Sent {
        tokens: Ceiling,
        from: CeilingSource,
    },
    /// Nobody stated one and the face takes none: the request leaves the
    /// field out and the provider applies its own default for the model.
    ProviderDefault,
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

impl OutputCeiling {
    /// The ceiling one call is made with, and who supplied it.
    ///
    /// `pinned` is the pinned catalogue's row for this exact id, and the
    /// preset table is asked with the target's base URL and id; both are
    /// the same rung reached through two indexes, and a test keeps the
    /// two indexes from answering for one id.
    ///
    /// `None` says every rung stayed silent on a face that needs a
    /// figure, which requires [`OUTPUT_CEILING_DEFAULT`] itself to be
    /// zero — a state the policy module's own test forbids. Returning it
    /// rather than substituting a number keeps this decision free of a
    /// figure it invented.
    #[must_use]
    pub fn resolve(
        stated: Stated,
        pinned: Option<Ceiling>,
        target: Target<'_>,
    ) -> Option<OutputCeiling> {
        let sent = |tokens, from| Some(OutputCeiling::Sent { tokens, from });
        if let Some(tokens) = stated.person {
            return sent(tokens, CeilingSource::Person);
        }
        if let Some(tokens) = stated.upstream {
            return sent(tokens, CeilingSource::Upstream);
        }
        match field_on(target.wire) {
            Field::Optional => Some(OutputCeiling::ProviderDefault),
            Field::Required => {
                match pinned.or_else(|| preset::ceiling_for(target.base_url, target.id)) {
                    Some(tokens) => sent(tokens, CeilingSource::Preset),
                    None => sent(Ceiling::new(OUTPUT_CEILING_DEFAULT)?, CeilingSource::Policy),
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

/// What somebody said about this model's ceiling.
///
/// The two arrive together because they are read at the same moment —
/// the person's form and the endpoint's model list — and passing them
/// apart gave two chances to apply them in the wrong order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stated {
    /// What the person entered against this model, if anything.
    pub person: Option<Ceiling>,
    /// What the provider's model list stated, if anything.
    pub upstream: Option<Ceiling>,
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

    fn sent(tokens: u64, from: CeilingSource) -> OutputCeiling {
        OutputCeiling::Sent {
            tokens: Ceiling::new(tokens).unwrap(),
            from,
        }
    }

    fn tokens(count: u64) -> Option<Ceiling> {
        Ceiling::new(count)
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
                Stated {
                    person: tokens(1_000),
                    upstream: tokens(2_000),
                },
                tokens(3_000),
                ANTHROPIC,
                1_000,
                CeilingSource::Person,
            ),
            (
                "the provider's own list outranks anything this city pinned",
                Stated {
                    person: None,
                    upstream: tokens(2_000),
                },
                tokens(3_000),
                ANTHROPIC,
                2_000,
                CeilingSource::Upstream,
            ),
            (
                "a pinned catalogue row answers when nobody stated one",
                Stated::default(),
                tokens(3_000),
                RELAY,
                3_000,
                CeilingSource::Preset,
            ),
            (
                "the preset table answers for a documented host",
                Stated::default(),
                None,
                ANTHROPIC,
                64_000,
                CeilingSource::Preset,
            ),
            (
                "policy answers last, for a server on this machine",
                Stated::default(),
                None,
                LOCAL,
                OUTPUT_CEILING_DEFAULT,
                CeilingSource::Policy,
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
            let got = OutputCeiling::resolve(Stated::default(), tokens(3_000), target);
            assert_eq!(got, Some(OutputCeiling::ProviderDefault), "{wire:?}");
            let stated = Stated {
                person: None,
                upstream: tokens(2_000),
            };
            let got = OutputCeiling::resolve(stated, None, target);
            assert_eq!(got, Some(sent(2_000, CeilingSource::Upstream)), "{wire:?}");
        }
    }

    /// A relay forwarding a vendor's id gets the vendor's documented
    /// ceiling on the messages face rather than the policy figure, which
    /// has nothing to do with either of them.
    #[test]
    fn a_relay_forwarding_a_vendors_id_is_called_at_the_vendors_ceiling() {
        let got = OutputCeiling::resolve(Stated::default(), None, messages(RELAY, SONNET));
        assert_eq!(got, Some(sent(64_000, CeilingSource::Preset)));
    }

    /// The whole point of the ladder: an id no table knows, on the wire
    /// that requires the field, now has a figure to send.
    #[test]
    fn an_unknown_model_at_an_unknown_host_is_still_callable() {
        let got =
            OutputCeiling::resolve(Stated::default(), None, messages(RELAY, "some-relay-model"));
        assert_eq!(
            got,
            Some(sent(OUTPUT_CEILING_DEFAULT, CeilingSource::Policy))
        );
        assert_eq!(got.map(OutputCeiling::word), Some("policy"));
    }

    #[test]
    fn each_rung_travels_under_one_word() {
        let words = [
            CeilingSource::Person,
            CeilingSource::Upstream,
            CeilingSource::Preset,
            CeilingSource::Policy,
        ]
        .map(CeilingSource::as_str);
        assert_eq!(words, ["person", "upstream", "preset", "policy"]);
        assert_eq!(OutputCeiling::ProviderDefault.word(), "provider");
    }
}

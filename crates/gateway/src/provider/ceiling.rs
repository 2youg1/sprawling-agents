// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which statement about one model's output ceiling wins, and who made
//! it (shape 1 decision).
//!
//! **A model with no registered ceiling could not be called at all.**
//! The Anthropic wire requires `max_tokens` in every request and refuses
//! to write one without it, so any id the pinned catalogue did not know
//! reached a dead end; the OpenAI wire omitted the field instead, so the
//! same city truncated answers differently on two providers for one
//! missing figure.
//!
//! The ladder below always answers, and it records which rung answered.
//! A truncated run is then read off the account rather than guessed at:
//! `model_selected` carries `ceiling_from`, which is the reply to the
//! objection this city wrote against itself in `anthropic.rs` — that a
//! ceiling invented at the call site truncates runs for a reason that
//! appears nowhere in the account.

use kernel::Ceiling;
use kernel::consts_policy::OUTPUT_CEILING_DEFAULT;

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

/// A ceiling and the rung it came from, which travel together because
/// a caller holding only the number cannot say why a run stopped where
/// it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputCeiling {
    tokens: Ceiling,
    from: CeilingSource,
}

impl OutputCeiling {
    /// The ceiling one call is made with, and who supplied it.
    ///
    /// `pinned` is the pinned catalogue's row for this exact id, and
    /// `at` with `id` is what the preset table is asked; both are the
    /// same rung reached through two indexes, and a test keeps the two
    /// indexes from answering for one id.
    ///
    /// `None` says every rung stayed silent, which requires
    /// [`OUTPUT_CEILING_DEFAULT`] itself to be zero — a state the policy
    /// module's own test forbids. Returning it rather than substituting
    /// a number keeps this decision free of a figure it invented.
    #[must_use]
    pub fn resolve(
        stated: Stated,
        pinned: Option<Ceiling>,
        at: &str,
        id: &str,
    ) -> Option<OutputCeiling> {
        let sourced = |tokens, from| Some(OutputCeiling { tokens, from });
        if let Some(tokens) = stated.person {
            return sourced(tokens, CeilingSource::Person);
        }
        if let Some(tokens) = stated.upstream {
            return sourced(tokens, CeilingSource::Upstream);
        }
        if let Some(tokens) = pinned.or_else(|| preset::ceiling_for(at, id)) {
            return sourced(tokens, CeilingSource::Preset);
        }
        sourced(Ceiling::new(OUTPUT_CEILING_DEFAULT)?, CeilingSource::Policy)
    }

    #[must_use]
    pub const fn tokens(self) -> Ceiling {
        self.tokens
    }

    /// Which rung answered. Named `source` rather than `from` because
    /// `from` is the conversion vocabulary of the standard library.
    #[must_use]
    pub const fn source(self) -> CeilingSource {
        self.from
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
    const SONNET: &str = "claude-sonnet-4-5";

    fn tokens(count: u64) -> Option<Ceiling> {
        Ceiling::new(count)
    }

    /// The priority table, one row per pair of rungs that can both
    /// answer. Read it downward: whatever is stated higher wins, and
    /// the rung that won is named in the record.
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
                "policy answers last, for a host nobody documented",
                Stated::default(),
                None,
                RELAY,
                OUTPUT_CEILING_DEFAULT,
                CeilingSource::Policy,
            ),
        ];
        for (why, stated, pinned, at, expected, from) in table {
            let got = OutputCeiling::resolve(stated, pinned, at, SONNET)
                .expect("the ladder always answers");
            assert_eq!(got.tokens().get(), expected, "{why}");
            assert_eq!(got.source(), from, "{why}");
        }
    }

    /// The whole point of the ladder: an id no table knows, on the wire
    /// that requires the field, now has a figure to send.
    #[test]
    fn an_unknown_model_at_an_unknown_host_is_still_callable() {
        let got = OutputCeiling::resolve(Stated::default(), None, RELAY, "some-relay-model")
            .expect("the ladder always answers");
        assert_eq!(got.tokens().get(), OUTPUT_CEILING_DEFAULT);
        assert_eq!(got.source().as_str(), "policy");
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
    }
}

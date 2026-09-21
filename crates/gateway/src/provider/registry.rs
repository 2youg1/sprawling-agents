// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! How one registered endpoint is connected, decided once (shape 1
//! decision).
//!
//! **Today three modules each hold a piece of this answer and none of
//! them can state it.** The dialect says which of two request writers
//! runs, the credential says whether a key or a subscription login
//! pays, and the ceiling ladder says what the call may write; a person
//! asking "how is this endpoint connected" gets three partial answers
//! that agree only until one of them is changed. The registration path
//! makes it worse by throwing a fact away: an endpoint the person
//! entered as a responses URL is stored as `DialectKind::OpenAi`,
//! because the stored enum has no third variant, and every later reader
//! has to guess what the person pasted.
//!
//! [`ConnectionKind`] is that one answer. It is resolved once, at
//! attach, after normalisation has folded the pasted URL, the person's
//! choice and the host table into one shape; it is written into
//! `endpoint_attached` and read back by `Query::Config`. **No call path
//! re-derives it**, which is the whole point: a fact derived twice is a
//! fact that can differ twice.
//!
//! What this module does not hold: the request bytes (`dialect`), the
//! header a key travels in (`endpoint::auth`), and the login flow of a
//! harness family (`credential::oauth`). It names the connection; the
//! modules that already own those facts keep them.

use kernel::{AxCode, AxError, DialectKind};

use crate::router::DialectHint;

/// A first-party client whose subscription this city signs in to
/// directly.
///
/// A family is not a URL and not a dialect: it is a vendor's own
/// harness, whose login this city implements itself and whose wire this
/// city writes itself. The city installs none of these clients, spawns
/// none of them, and carries none of their source — it follows their
/// published facts, which is what `docs/third-party.md` §1 records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// OpenAI's own client; signs in by device flow, answers on the
    /// responses face.
    Codex,
    /// Anthropic's own client; the subscription OAuth this city already
    /// implements in `credential::oauth`.
    ClaudeCode,
    /// xAI's own client; OpenAI-compatible chat under `api.x.ai`.
    GrokBuild,
    /// Moonshot's own client; OpenAI-compatible chat under the Kimi
    /// hosts.
    KimiCli,
}

impl Family {
    /// The request shape this family's subscription answers in.
    ///
    /// Stated here rather than beside each login flow, so that a
    /// registration made from a subscription and one made from a pasted
    /// URL are checked against the same statement.
    #[must_use]
    pub const fn shape(self) -> DialectHint {
        match self {
            Family::Codex => DialectHint::Responses,
            Family::ClaudeCode => DialectHint::Messages,
            Family::GrokBuild | Family::KimiCli => DialectHint::Chat,
        }
    }

    /// The word this family travels under in the ledger, on the wire,
    /// and in a person's configuration file. One spelling, written
    /// here.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Family::Codex => "codex",
            Family::ClaudeCode => "claude_code",
            Family::GrokBuild => "grok_build",
            Family::KimiCli => "kimi_cli",
        }
    }

    /// Every family, in the order this module declares them. The data
    /// face a credential form is generated from, so that adding a
    /// family adds a form rather than a form and a list.
    pub const ALL: [Family; 4] = [
        Family::Codex,
        Family::ClaudeCode,
        Family::GrokBuild,
        Family::KimiCli,
    ];
}

/// How this city talks to one attached endpoint.
///
/// Exhaustive and closed: a fifth way to connect is a compile error at
/// every reader, which is how a new one is kept from being approximated
/// with the nearest of the four already written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    /// The OpenAI-compatible chat face — what most relays, most local
    /// servers and most other laboratories serve.
    OpenAiCompat,
    /// OpenAI's responses face. Registered apart from the chat face
    /// because the person pasted a responses URL and that statement is
    /// theirs, not this build's, to keep.
    Responses,
    /// Anthropic's messages face.
    AnthropicNative,
    /// A first-party harness this city signs in to as that vendor's own
    /// client.
    Harness(Family),
}

impl ConnectionKind {
    /// Which request writer serves this connection today.
    ///
    /// `Responses` answers `OpenAi` because this build writes one
    /// OpenAI-shaped body and the responses body is not written yet
    /// (roadmap 4.5). **The registration still records `Responses`**,
    /// so the day that writer exists one arm of this match changes and
    /// nobody has to re-attach an endpoint to be called correctly.
    #[must_use]
    pub const fn wire(self) -> DialectKind {
        match self {
            ConnectionKind::AnthropicNative => DialectKind::Anthropic,
            ConnectionKind::OpenAiCompat | ConnectionKind::Responses => DialectKind::OpenAi,
            ConnectionKind::Harness(family) => match family {
                Family::ClaudeCode => DialectKind::Anthropic,
                Family::Codex | Family::GrokBuild | Family::KimiCli => DialectKind::OpenAi,
            },
        }
    }

    /// The word this connection travels under, everywhere it travels.
    ///
    /// Flat rather than compound: a harness is spelled by its family
    /// alone, so a reader splits nothing and a payload key holds one
    /// token. [`ConnectionKind::parse`] is the inverse.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ConnectionKind::OpenAiCompat => "openai_compat",
            ConnectionKind::Responses => "responses",
            ConnectionKind::AnthropicNative => "anthropic_native",
            ConnectionKind::Harness(family) => family.as_str(),
        }
    }

    /// The connection one recorded word names.
    ///
    /// # Errors
    /// A word no build of this city ever wrote, which a ledger written
    /// by a newer binary is the only way to reach.
    pub fn parse(word: &str) -> Result<ConnectionKind, AxError> {
        for family in Family::ALL {
            if family.as_str() == word {
                return Ok(ConnectionKind::Harness(family));
            }
        }
        match word {
            "openai_compat" => Ok(ConnectionKind::OpenAiCompat),
            "responses" => Ok(ConnectionKind::Responses),
            "anthropic_native" => Ok(ConnectionKind::AnthropicNative),
            other => Err(AxError::failure(
                AxCode::ConfigInvalid,
                "read how an endpoint is connected",
                format!("{other} is not a connection this build knows"),
            )
            .with_recovery(
                "attach this endpoint again with this build: the word was written by a \
                 newer one, and calling it under a guessed connection would send the \
                 wrong request shape",
            )),
        }
    }
}

/// Resolve how an endpoint is connected, once, at attach.
///
/// `shape` is what normalisation settled — the pasted URL first, then
/// the person's choice, then the host table. `subscription` is the
/// family whose login this registration pays with, and `None` is a key
/// the person entered.
///
/// A subscription decides the connection, because the harness wire
/// belongs to the vendor rather than to the form. A contradiction is
/// refused rather than resolved silently: somebody who signed in to one
/// vendor and pasted another vendor's URL has made a mistake that costs
/// a 404 at the first call and is free to fix here.
///
/// # Errors
/// When nothing said what shape the endpoint answers in, and when a
/// subscription and a pasted URL name different shapes.
pub fn resolve(
    shape: DialectHint,
    subscription: Option<Family>,
) -> Result<ConnectionKind, AxError> {
    match subscription {
        None => by_shape(shape),
        Some(family) => {
            if shape == DialectHint::Unset || shape == family.shape() {
                Ok(ConnectionKind::Harness(family))
            } else {
                Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "resolve how an endpoint is connected",
                    format!(
                        "signed in to {}, which answers on {}, but this URL names another face",
                        family.as_str(),
                        shape_word(family.shape())
                    ),
                )
                .with_recovery(
                    "attach the vendor's own base URL under this subscription, or attach \
                     the pasted URL with an API key instead",
                ))
            }
        }
    }
}

/// The connection a pasted URL names on its own.
fn by_shape(shape: DialectHint) -> Result<ConnectionKind, AxError> {
    match shape {
        DialectHint::Chat => Ok(ConnectionKind::OpenAiCompat),
        DialectHint::Responses => Ok(ConnectionKind::Responses),
        DialectHint::Messages => Ok(ConnectionKind::AnthropicNative),
        DialectHint::Unset => Err(AxError::failure(
            AxCode::ConfigInvalid,
            "resolve how an endpoint is connected",
            "neither the URL nor the person said which shape this endpoint answers in",
        )
        .with_recovery(
            "choose the request shape on the endpoint form, or paste the full URL the \
             provider's documentation prints: a path ending in chat/completions, \
             responses or messages says it by itself",
        )),
    }
}

/// One shape as a person reads it in a refusal.
fn shape_word(shape: DialectHint) -> &'static str {
    match shape {
        DialectHint::Unset => "no face",
        DialectHint::Chat => "chat completions",
        DialectHint::Responses => "responses",
        DialectHint::Messages => "messages",
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

    #[test]
    fn a_pasted_face_decides_the_connection() {
        assert_eq!(
            resolve(DialectHint::Chat, None).unwrap(),
            ConnectionKind::OpenAiCompat
        );
        assert_eq!(
            resolve(DialectHint::Messages, None).unwrap(),
            ConnectionKind::AnthropicNative
        );
    }

    /// The fact the registration path throws away today: a responses
    /// URL stored as `DialectKind::OpenAi` reaches every later reader
    /// as a chat endpoint.
    #[test]
    fn a_responses_url_stays_a_responses_registration() {
        let kind = resolve(DialectHint::Responses, None).unwrap();
        assert_eq!(kind, ConnectionKind::Responses);
        assert_ne!(kind, ConnectionKind::OpenAiCompat);
        assert_eq!(kind.wire(), DialectKind::OpenAi);
    }

    #[test]
    fn an_endpoint_nobody_named_a_shape_for_is_refused_with_a_way_out() {
        let refusal = resolve(DialectHint::Unset, None).unwrap_err();
        assert_eq!(*refusal.code(), AxCode::ConfigInvalid);
        assert!(refusal.recovery().contains("chat/completions"));
    }

    #[test]
    fn a_subscription_names_the_connection_and_its_wire() {
        for family in Family::ALL {
            let kind = resolve(family.shape(), Some(family)).unwrap();
            assert_eq!(kind, ConnectionKind::Harness(family));
            assert_eq!(resolve(DialectHint::Unset, Some(family)).unwrap(), kind);
        }
        assert_eq!(
            ConnectionKind::Harness(Family::ClaudeCode).wire(),
            DialectKind::Anthropic
        );
        assert_eq!(
            ConnectionKind::Harness(Family::GrokBuild).wire(),
            DialectKind::OpenAi
        );
    }

    #[test]
    fn a_subscription_and_a_url_that_disagree_are_refused_at_attach() {
        let refusal = resolve(DialectHint::Messages, Some(Family::GrokBuild)).unwrap_err();
        assert_eq!(*refusal.code(), AxCode::ConfigInvalid);
        assert!(refusal.subject().contains("grok_build"));
    }

    #[test]
    fn every_connection_writes_one_word_and_reads_back_from_it() {
        let mut words = Vec::new();
        let kinds = [
            ConnectionKind::OpenAiCompat,
            ConnectionKind::Responses,
            ConnectionKind::AnthropicNative,
            ConnectionKind::Harness(Family::Codex),
            ConnectionKind::Harness(Family::ClaudeCode),
            ConnectionKind::Harness(Family::GrokBuild),
            ConnectionKind::Harness(Family::KimiCli),
        ];
        for kind in kinds {
            assert_eq!(ConnectionKind::parse(kind.as_str()).unwrap(), kind);
            words.push(kind.as_str());
        }
        words.sort_unstable();
        let written = words.len();
        words.dedup();
        assert_eq!(words.len(), written, "two connections under one word");
    }

    #[test]
    fn a_word_this_build_never_wrote_is_refused_rather_than_guessed() {
        let refusal = ConnectionKind::parse("gemini_native").unwrap_err();
        assert_eq!(*refusal.code(), AxCode::ConfigInvalid);
        assert!(refusal.recovery().contains("attach this endpoint again"));
    }
}

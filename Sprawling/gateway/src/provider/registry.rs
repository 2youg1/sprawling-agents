// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, DialectKind};

use crate::router::DialectHint;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConnectionKind {
    OpenAiCompat,

    Responses,

    AnthropicNative,
}

impl ConnectionKind {
    #[must_use]
    pub const fn wire(self) -> DialectKind {
        match self {
            ConnectionKind::AnthropicNative => DialectKind::Anthropic,
            ConnectionKind::OpenAiCompat => DialectKind::OpenAi,
            ConnectionKind::Responses => DialectKind::OpenAiResponses,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ConnectionKind::OpenAiCompat => "openai_compat",
            ConnectionKind::Responses => "responses",
            ConnectionKind::AnthropicNative => "anthropic_native",
        }
    }

    pub fn parse(word: &str) -> Result<ConnectionKind, AxError> {
        match word {
            "openai_compat" | "grok_build" | "kimi_cli" => Ok(ConnectionKind::OpenAiCompat),
            "responses" | "codex" => Ok(ConnectionKind::Responses),
            "anthropic_native" | "claude_code" => Ok(ConnectionKind::AnthropicNative),
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

pub fn resolve(shape: DialectHint) -> Result<ConnectionKind, AxError> {
    match shape {
        DialectHint::Chat => Ok(ConnectionKind::OpenAiCompat),
        DialectHint::Responses => Ok(ConnectionKind::Responses),
        DialectHint::Messages => Ok(ConnectionKind::AnthropicNative),
        DialectHint::Unset => Err(AxError::failure(
            AxCode::ConfigInvalid,
            "resolve how an endpoint is connected",
            "neither the URL nor the User said which shape this endpoint answers in",
        )
        .with_recovery(
            "choose the request shape on the endpoint form, or paste the full URL the \
             provider's documentation prints: a path ending in chat/completions, \
             responses or messages says it by itself",
        )),
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
            resolve(DialectHint::Chat).unwrap(),
            ConnectionKind::OpenAiCompat
        );
        assert_eq!(
            resolve(DialectHint::Messages).unwrap(),
            ConnectionKind::AnthropicNative
        );
    }

    #[test]
    fn a_responses_url_is_registered_and_called_on_the_responses_face() {
        let kind = resolve(DialectHint::Responses).unwrap();
        assert_eq!(kind, ConnectionKind::Responses);
        assert_ne!(kind, ConnectionKind::OpenAiCompat);
        assert_eq!(kind.wire(), DialectKind::OpenAiResponses);
    }

    #[test]
    fn an_endpoint_nobody_named_a_shape_for_is_refused_with_a_way_out() {
        let refusal = resolve(DialectHint::Unset).unwrap_err();
        assert_eq!(*refusal.code(), AxCode::ConfigInvalid);
        assert!(refusal.recovery().contains("chat/completions"));
    }

    #[test]
    fn every_connection_writes_one_word_and_reads_back_from_it() {
        let mut words = Vec::new();
        let kinds = [
            ConnectionKind::OpenAiCompat,
            ConnectionKind::Responses,
            ConnectionKind::AnthropicNative,
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
    fn a_harness_word_an_older_build_wrote_reads_as_the_face_it_answered_on() {
        for (word, face) in [
            ("codex", ConnectionKind::Responses),
            ("claude_code", ConnectionKind::AnthropicNative),
            ("grok_build", ConnectionKind::OpenAiCompat),
            ("kimi_cli", ConnectionKind::OpenAiCompat),
        ] {
            assert_eq!(ConnectionKind::parse(word).unwrap(), face, "{word}");
        }
    }

    #[test]
    fn a_word_this_build_never_wrote_is_refused_rather_than_guessed() {
        let refusal = ConnectionKind::parse("gemini_native").unwrap_err();
        assert_eq!(*refusal.code(), AxCode::ConfigInvalid);
        assert!(refusal.recovery().contains("attach this endpoint again"));
    }
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The book of attached endpoints and the models chosen from them
//! (shape 7): a value rebuilt from the ledger, never a store. Deleting
//! it and replaying the city produces the same book.
//!
//! Two questions live here and nowhere else. *What did the person
//! attach* — a base URL, a dialect, a credential reference, and the
//! model ids the endpoint reported. *Which model answers for a tag* —
//! the choice a request needs, with the facts no probe returns.
//!
//! Duty pools and multi-axis grading are deliberately absent: with one
//! agent per run there is no consumer for a pool, and a pool without a
//! consumer is an authority nobody reads.

//! Attached endpoints: registration records.

use kernel::DialectKind;

use crate::endpoint::AuthSpec;
use crate::native::is_loopback;

use super::payload::auth_reference;
/// One endpoint the person attached, as the book holds it.
#[derive(Debug, Clone)]
pub struct AttachedEndpoint {
    pub name: String,
    /// The base URL the person entered, without a path of its own; the
    /// dialect knows which paths hang off it.
    pub base_url: String,
    pub dialect: DialectKind,
    pub auth: AuthSpec,
    /// What the endpoint said it serves. Ids only: no provider returns
    /// prices or limits from its model list, and a number we invented
    /// would outrank the one the provider actually bills.
    pub models: Vec<String>,
}

impl AttachedEndpoint {
    /// Whether calls to this endpoint stay on this machine. The answer
    /// comes from the same test the local adapter applies, so "local"
    /// means one thing city-wide.
    #[must_use]
    pub fn is_local(&self) -> bool {
        is_loopback(&self.base_url)
    }

    /// Whether a credential was enrolled for it. The only question about
    /// a credential this side of the vault can answer, and the only one
    /// a settings page needs.
    #[must_use]
    pub fn has_credential(&self) -> bool {
        auth_reference(&self.auth).is_some()
    }

    /// Where a chat request goes for this dialect. The person enters a
    /// base URL because that is what a provider's documentation prints;
    /// the path belongs to the dialect, which is the only party that
    /// knows it.
    #[must_use]
    pub fn chat_url(&self) -> String {
        join(&self.base_url, chat_path(self.dialect))
    }

    /// Where the model list lives for this dialect.
    #[must_use]
    pub fn models_url(&self) -> String {
        join(&self.base_url, "models")
    }
}

/// Both dialects list models at the same path; they differ in the chat
/// path and in the shape of what comes back.
fn chat_path(dialect: DialectKind) -> &'static str {
    match dialect {
        DialectKind::Anthropic => "messages",
        // Anything not spelled here is served by the OpenAI-compatible
        // shape, which is what an unknown local server almost always is.
        _ => "chat/completions",
    }
}

fn join(base: &str, path: &str) -> String {
    format!("{}/{}", base.trim_end_matches('/'), path)
}

#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::super::book::EndpointBook;
    use super::super::payload::attached_payload;
    use super::*;
    use crate::endpoint::AuthSpec;
    use kernel::{DialectKind, EventKind, EventRecord, Payload, SecretRef};
    use kernel::{EventDraft, GENESIS_PREV, RunId, Seq, TimeMs};
    fn record(kind: EventKind, data: Payload) -> EventRecord {
        EventRecord::from_draft(
            EventDraft {
                run: RunId::CITY,
                t: TimeMs::new(1),
                who: "owner".to_owned(),
                addr: None,
                kind,
                data,
                ig: false,
            },
            Seq::FIRST,
            GENESIS_PREV,
        )
    }
    fn attached(name: &str, base_url: &str) -> AttachedEndpoint {
        AttachedEndpoint {
            name: name.to_owned(),
            base_url: base_url.to_owned(),
            dialect: DialectKind::OpenAi,
            auth: AuthSpec::Bearer(SecretRef::parse("secret:provider/key").unwrap()),
            models: vec!["m-small".to_owned(), "m-large".to_owned()],
        }
    }

    #[test]
    fn an_attachment_survives_the_payload_round_trip() {
        let endpoint = attached("house", "https://api.example.test/v1");
        let mut book = EndpointBook::new();
        book.apply(&record(
            EventKind::EndpointAttached,
            attached_payload(&endpoint).unwrap(),
        ))
        .unwrap();
        let held = book.endpoints().next().unwrap();
        assert_eq!(held.name, endpoint.name);
        assert_eq!(held.base_url, endpoint.base_url);
        assert_eq!(held.models, endpoint.models);
    }

    #[test]
    fn the_payload_carries_a_reference_and_never_a_credential() {
        let endpoint = attached("house", "https://api.example.test/v1");
        let payload = attached_payload(&endpoint).unwrap();
        let text = serde_json::to_string(&payload).unwrap();
        assert!(text.contains("secret:provider/key"));
        assert!(
            !text.contains("Bearer"),
            "the ledger records where a credential lives, not what it is"
        );
    }
}

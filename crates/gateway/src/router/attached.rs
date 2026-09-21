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

use crate::endpoint::{AuthSpec, ModelFacts};
use crate::provider::registry::ConnectionKind;

use super::payload::auth_reference;
use super::tuning::EndpointTuning;
/// One endpoint the person attached, as the book holds it.
#[derive(Debug, Clone)]
pub struct AttachedEndpoint {
    pub name: String,
    /// The base URL the person entered, without a path of its own; the
    /// dialect knows which paths hang off it.
    pub base_url: String,
    pub dialect: DialectKind,
    /// How this endpoint is connected, resolved once when it was
    /// attached. `dialect` beside it answers the narrower question of
    /// which request writer runs, and two connections can share a
    /// writer while being different registrations; a page given only
    /// the writer cannot say which one a person set up.
    pub connection_kind: ConnectionKind,
    pub auth: AuthSpec,
    /// What the endpoint said it serves, with everything it said about
    /// each row. Kept whole rather than reduced to ids: the probe
    /// already reads the window, the output ceiling, the modalities and
    /// the provider's own prices out of `GET .../models`, and throwing
    /// them away here made the settings page ask a person for figures
    /// their provider had already stated.
    pub models: Vec<ModelFacts>,
    /// Where that list came from: `true` when the endpoint answered
    /// `GET .../models`, `false` when it did not and the person named
    /// the ids instead. It says how much the city knows about this
    /// list, never whether the endpoint is healthy.
    pub probed: bool,
    /// How the person set this endpoint up: its label, its deadlines,
    /// and what every request to it carries. Kept beside the
    /// registration so a call made a week later is made the way they
    /// set it up.
    pub tuning: EndpointTuning,
}

impl AttachedEndpoint {
    /// Whether calls to this endpoint stay on this machine. The answer
    /// comes from the same test the local adapter applies and the proxy
    /// decision takes, so "local" means one thing city-wide.
    #[must_use]
    pub fn is_local(&self) -> bool {
        crate::is_local(&self.base_url)
    }

    /// Whether a credential was enrolled for it. The only question about
    /// a credential this side of the vault can answer, and the only one
    /// a settings page needs.
    #[must_use]
    pub fn has_credential(&self) -> bool {
        auth_reference(&self.auth).is_some()
    }

    /// What to call this endpoint on screen: the label the person gave
    /// it, or the name it was filed under.
    #[must_use]
    pub fn label(&self) -> &str {
        self.tuning.label.as_deref().unwrap_or(&self.name)
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

/// All three compatible formats list models at the same path; they
/// differ in the chat path and in the shape of what comes back.
fn chat_path(dialect: DialectKind) -> &'static str {
    match dialect {
        DialectKind::Anthropic => "messages",
        DialectKind::OpenAi => "chat/completions",
        DialectKind::OpenAiResponses => "responses",
    }
}

/// A base URL a person entered plus the path a compatible format owns.
/// One algorithm city-wide: the chat face, the model list and the audio
/// face all hang paths off a base the same way.
pub(crate) fn join(base: &str, path: &str) -> String {
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
    /// One catalogue row as a probe that answered nothing but the id
    /// would have left it.
    fn facts(id: &str) -> ModelFacts {
        ModelFacts {
            id: id.to_owned(),
            context_tokens: None,
            max_output_tokens: None,
            input_modalities: Vec::new(),
            input_price: None,
            output_price: None,
        }
    }

    fn attached(name: &str, base_url: &str) -> AttachedEndpoint {
        AttachedEndpoint {
            name: name.to_owned(),
            base_url: base_url.to_owned(),
            dialect: DialectKind::OpenAi,
            connection_kind: ConnectionKind::OpenAiCompat,
            auth: AuthSpec::Bearer(SecretRef::parse("secret:provider/key").unwrap()),
            models: vec![facts("m-small"), facts("m-large")],
            probed: true,
            tuning: EndpointTuning::default(),
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
    fn a_record_written_before_probed_existed_reads_as_probed() {
        let endpoint = attached("house", "https://api.example.test/v1");
        let mut map = attached_payload(&endpoint).unwrap().as_map().clone();
        map.remove("probed");
        let read = super::super::payload::read_attached(&Payload::new(map).unwrap()).unwrap();
        assert!(
            read.probed,
            "a ledger written before this key existed recorded only probed attachments"
        );
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

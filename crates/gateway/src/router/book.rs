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

//! The endpoint book: choices the city made.

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, BuildingPolicy, EventKind, EventRecord, ModelTag, Payload};

use crate::fallback::Fallback;
use crate::market::ModelEntry;

use super::attached::AttachedEndpoint;
use super::payload::{read_attached, read_choice, text};
/// The model that answers for one tag, the endpoint it lives behind,
/// and what happens when that endpoint will not answer. The third is
/// here rather than at the call site because a caller holding only the
/// first two has no honest move left when the call fails.
#[derive(Debug, Clone, Copy)]
pub struct Chosen<'b> {
    pub endpoint: &'b AttachedEndpoint,
    pub entry: &'b ModelEntry,
    pub fallback: &'b Fallback,
}

#[derive(Debug, Clone)]
pub(crate) struct Choice {
    pub(crate) endpoint: String,
    pub(crate) entry: ModelEntry,
    pub(crate) fallback: Fallback,
}

/// Every endpoint and every choice, rebuilt from the event stream.
#[derive(Debug, Clone, Default)]
pub struct EndpointBook {
    endpoints: BTreeMap<String, AttachedEndpoint>,
    chosen: BTreeMap<ModelTag, Choice>,
}

impl EndpointBook {
    #[must_use]
    pub fn new() -> EndpointBook {
        EndpointBook::default()
    }

    /// Folds one record. Records of other kinds pass through untouched,
    /// so a caller may hand it the whole stream.
    ///
    /// # Errors
    /// A payload of a kind this book owns that it cannot read is an
    /// error rather than a skipped record: a book that quietly drops a
    /// registration would send calls to an endpoint the person retired.
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.apply_payload(record.kind(), record.data())
    }

    /// The same fold, for the writer that has the payload in hand and
    /// not yet a record. Both entry points read the payload with one
    /// reader, so what the writer believes and what a rebuild produces
    /// cannot drift.
    ///
    /// # Errors
    /// As [`EndpointBook::apply`].
    pub fn apply_payload(&mut self, kind: EventKind, data: &Payload) -> Result<(), AxError> {
        match kind {
            EventKind::EndpointAttached => {
                let attached = read_attached(data)?;
                self.endpoints.insert(attached.name.clone(), attached);
                Ok(())
            }
            EventKind::EndpointLost => {
                let name = text(data, "name")?;
                self.endpoints.remove(&name);
                self.chosen.retain(|_, choice| choice.endpoint != name);
                Ok(())
            }
            EventKind::ModelSelected => {
                let (tag, choice) = read_choice(data)?;
                self.chosen.insert(tag, choice);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// The model for one tag under one building's policy.
    ///
    /// # Errors
    /// Refuses when nothing is chosen for the tag, when the endpoint
    /// behind the choice is gone, and when a confidential building's
    /// choice would leave this machine.
    pub fn select(&self, tag: ModelTag, policy: &BuildingPolicy) -> Result<Chosen<'_>, AxError> {
        let choice = self.chosen.get(&tag).ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                format!("choose the {tag} model"),
                "no model is chosen for this tag",
            )
            .with_recovery("attach a provider on the settings page and pick a model for this tag")
        })?;
        let endpoint = self.endpoints.get(&choice.endpoint).ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                format!("choose the {tag} model"),
                format!("the endpoint {} is no longer attached", choice.endpoint),
            )
            .with_recovery("pick a model from an attached endpoint")
        })?;
        if policy.confidential && !endpoint.is_local() {
            return Err(AxError::failure(
                AxCode::GateDenied,
                format!("choose the {tag} model"),
                format!(
                    "{} is confidential and {} is not on this machine",
                    "this building", endpoint.base_url
                ),
            )
            .with_recovery("attach a loopback inference server and pick a model from it"));
        }
        Ok(Chosen {
            endpoint,
            entry: &choice.entry,
            fallback: &choice.fallback,
        })
    }

    pub fn endpoints(&self) -> impl Iterator<Item = &AttachedEndpoint> {
        self.endpoints.values()
    }

    /// What is chosen, tag by tag, and what each choice retreats to.
    pub fn choices(&self) -> impl Iterator<Item = (ModelTag, &str, &ModelEntry, &Fallback)> {
        self.chosen.iter().map(|(tag, choice)| {
            (
                *tag,
                choice.endpoint.as_str(),
                &choice.entry,
                &choice.fallback,
            )
        })
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.endpoints.is_empty() && self.chosen.is_empty()
    }
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
    use super::super::payload::{attached_payload, selected_payload};
    use super::*;
    use crate::endpoint::AuthSpec;
    use crate::fallback::Fallback;
    use crate::market::ModelEntry;
    use kernel::{
        AxCode, DialectKind, EventKind, EventRecord, ModelTag, Payload, SecretRef, UsdMicros,
    };
    use kernel::{EventDraft, GENESIS_PREV, RunId, Seq, TimeMs};
    use serde_json::Map;
    use serde_json::Value;
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
            probed: true,
        }
    }
    fn entry(id: &str) -> ModelEntry {
        ModelEntry {
            id: id.to_owned(),
            context_tokens: 128_000,
            max_output_tokens: kernel::Ceiling::new(8_192),
            input: crate::market::InputKinds::Text,
            input_price: UsdMicros::new(1_000_000),
            output_price: UsdMicros::new(2_000_000),
            cache_read_price: UsdMicros::new(0),
            cache_write_price: UsdMicros::new(0),
        }
    }
    fn book_with(base_url: &str) -> EndpointBook {
        book_falling_back_to(base_url, &Fallback::None)
    }

    fn book_falling_back_to(base_url: &str, fallback: &Fallback) -> EndpointBook {
        let mut book = EndpointBook::new();
        let endpoint = attached("house", base_url);
        book.apply(&record(
            EventKind::EndpointAttached,
            attached_payload(&endpoint).unwrap(),
        ))
        .unwrap();
        book.apply(&record(
            EventKind::ModelSelected,
            selected_payload(ModelTag::Main, "house", &entry("m-large"), fallback).unwrap(),
        ))
        .unwrap();
        book
    }

    #[test]
    fn a_choice_carries_its_fallback_through_the_ledger_and_back() {
        let spare = Fallback::then("spare", "m-small").unwrap();
        let book = book_falling_back_to("https://api.example.test/v1", &spare);
        let chosen = book
            .select(ModelTag::Main, &BuildingPolicy::default())
            .unwrap();
        assert_eq!(*chosen.fallback, spare);
    }

    #[test]
    fn a_record_written_before_a_fallback_could_be_set_reads_as_none() {
        let book = book_with("https://api.example.test/v1");
        let chosen = book
            .select(ModelTag::Main, &BuildingPolicy::default())
            .unwrap();
        assert_eq!(
            *chosen.fallback,
            Fallback::None,
            "an old ledger must not gain a retreat nobody ever asked for"
        );
    }

    #[test]
    fn a_tag_with_no_choice_says_what_to_do_about_it() {
        let err = EndpointBook::new()
            .select(ModelTag::Digest, &BuildingPolicy::default())
            .unwrap_err();
        assert_eq!(*err.code(), AxCode::ConfigInvalid);
        assert!(err.recovery().contains("settings page"));
    }

    #[test]
    fn a_confidential_building_cannot_choose_a_model_off_this_machine() {
        let remote = book_with("https://api.example.test/v1");
        assert!(
            remote
                .select(ModelTag::Main, &BuildingPolicy::default())
                .is_ok()
        );
        let err = remote
            .select(ModelTag::Main, &BuildingPolicy::new(true))
            .unwrap_err();
        assert_eq!(*err.code(), AxCode::GateDenied);

        let local = book_with("http://127.0.0.1:11434/v1");
        assert!(
            local
                .select(ModelTag::Main, &BuildingPolicy::new(true))
                .is_ok(),
            "a loopback endpoint is what a confidential building is allowed to reach"
        );
    }

    #[test]
    fn losing_an_endpoint_takes_its_choices_with_it() {
        let mut book = book_with("https://api.example.test/v1");
        let mut gone = Map::new();
        gone.insert("name".to_owned(), Value::String("house".to_owned()));
        book.apply(&record(
            EventKind::EndpointLost,
            Payload::new(gone).unwrap(),
        ))
        .unwrap();
        assert!(book.is_empty());
        assert!(
            book.select(ModelTag::Main, &BuildingPolicy::default())
                .is_err()
        );
    }

    #[test]
    fn the_dialect_owns_the_path_the_person_did_not_type() {
        let openai = attached("house", "https://api.example.test/v1/");
        assert_eq!(
            openai.chat_url(),
            "https://api.example.test/v1/chat/completions"
        );
        assert_eq!(openai.models_url(), "https://api.example.test/v1/models");
        let mut anthropic = attached("house", "https://api.example.test/v1");
        anthropic.dialect = DialectKind::Anthropic;
        assert_eq!(anthropic.chat_url(), "https://api.example.test/v1/messages");
    }

    #[test]
    fn a_record_this_book_does_not_own_passes_through() {
        let mut book = EndpointBook::new();
        book.apply(&record(EventKind::RunStarted, Payload::empty()))
            .unwrap();
        assert!(book.is_empty());
    }

    #[test]
    fn an_unreadable_registration_is_an_error_not_a_skip() {
        let mut book = EndpointBook::new();
        let mut half = Map::new();
        half.insert("name".to_owned(), Value::String("house".to_owned()));
        let err = book
            .apply(&record(
                EventKind::EndpointAttached,
                Payload::new(half).unwrap(),
            ))
            .unwrap_err();
        assert_eq!(*err.code(), AxCode::WireMismatch);
    }
}

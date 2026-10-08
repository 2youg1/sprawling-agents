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

use kernel::event::record::EndpointLost;
use kernel::{AxCode, AxError, BuildingPolicy, EventKind, EventRecord, ModelTag, Payload};

use crate::concurrency::vendor_in_flight;
use crate::endpoint::{Transport, WarmUp};
use crate::market::ModelEntry;

use super::attached::AttachedEndpoint;
use super::payload::{read_attached, read_choice};
/// The model that answers for one tag and the endpoint it lives
/// behind.
#[derive(Debug, Clone, Copy)]
pub struct Chosen<'b> {
    pub endpoint: &'b AttachedEndpoint,
    pub entry: &'b ModelEntry,
    /// The client every call to this endpoint shares.
    pub(crate) transport: &'b Transport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Choice {
    pub(crate) endpoint: String,
    pub(crate) entry: ModelEntry,
}

/// Every endpoint and every choice, rebuilt from the event stream.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EndpointBook {
    endpoints: BTreeMap<String, Held>,
    chosen: BTreeMap<ModelTag, Choice>,
    #[serde(default)]
    affinity: BTreeMap<kernel::Address, BTreeMap<String, kernel::ServerLabel>>,
    #[serde(default)]
    attempts: BTreeMap<kernel::RunId, kernel::event::record::ProviderAccountBinding>,
    #[serde(default)]
    session_rooms: BTreeMap<kernel::RunId, kernel::Address>,
}

/// One attached endpoint and the client its calls share. A second
/// registration under the same name replaces both, because the tuning
/// the client was built from may have changed.
/// A snapshot keeps the endpoint alone: the client is built again on
/// first use, as it is after an attachment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(from = "Kept", into = "Kept")]
struct Held {
    endpoint: AttachedEndpoint,
    transport: Transport,
}

/// What a snapshot keeps of a [`Held`]: the endpoint alone.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Kept {
    endpoint: AttachedEndpoint,
}

impl From<Kept> for Held {
    /// The endpoint with a transport whose gate admits its tuning's
    /// ceiling, or the default for its kind of connection, so an
    /// attachment and a snapshot restore build the same gate.
    fn from(Kept { endpoint }: Kept) -> Held {
        let ceiling = endpoint
            .tuning
            .max_in_flight
            .unwrap_or_else(|| vendor_in_flight(&endpoint.base_url));
        Held {
            endpoint,
            transport: Transport::admitting(ceiling),
        }
    }
}

impl From<Held> for Kept {
    fn from(Held { endpoint, .. }: Held) -> Kept {
        Kept { endpoint }
    }
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
        self.absorb(record.kind(), record.run(), record.addr(), record.data())
    }

    /// The same fold, for the writer that has the payload in hand and
    /// not yet a record. Both entry points read the payload with one
    /// reader, so what the writer believes and what a rebuild produces
    /// cannot drift.
    ///
    /// Besides registrations it folds successful Session account
    /// affinity: `RunStarted` admits each Run to its room's current
    /// Session, `ModelCalled` holds a member's last attempted account,
    /// `ModelReturned` commits it, and `SessionOpened` revokes the room's
    /// old Runs, whose later calls and answers then cannot bind.
    ///
    /// # Errors
    /// Propagates unreadable owned records before making a decision from them.
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "three kinds move registrations and five move Session affinity; the rest of the vocabulary passes through"
    )]
    pub fn absorb(
        &mut self,
        kind: EventKind,
        run: kernel::RunId,
        addr: Option<&kernel::Address>,
        data: &Payload,
    ) -> Result<(), AxError> {
        match kind {
            EventKind::EndpointAttached => {
                let mut attached = read_attached(data)?;
                attached.tuning.accounts =
                    self.accounts_for_attachment(&attached.name, attached.tuning.accounts);
                self.endpoints.insert(
                    attached.name.clone(),
                    Held::from(Kept { endpoint: attached }),
                );
            }
            EventKind::EndpointLost => {
                let EndpointLost { name } = data.read()?;
                self.endpoints.remove(&name);
                self.affinity.retain(|_, bindings| {
                    bindings.remove(&name);
                    !bindings.is_empty()
                });
                self.chosen.retain(|_, choice| choice.endpoint != name);
            }
            EventKind::ModelSelected => {
                let (tag, choice) = read_choice(data)?;
                self.chosen.insert(tag, choice);
            }
            EventKind::RunStarted => {
                if let Some(addr) = addr {
                    self.session_rooms.insert(run, addr.clone());
                }
            }
            EventKind::ModelCalled => {
                let called: kernel::event::record::ModelCalled = data.read()?;
                if self.session_rooms.contains_key(&run) {
                    match called.provider_account {
                        Some(binding) => self.attempts.insert(run, binding),
                        None => self.attempts.remove(&run),
                    };
                }
            }
            EventKind::ModelReturned => {
                if let Some(addr) = self.session_rooms.get(&run)
                    && let Some(binding) = self.attempts.remove(&run)
                {
                    self.affinity
                        .entry(addr.clone())
                        .or_default()
                        .insert(binding.provider, binding.account);
                }
            }
            EventKind::SessionOpened => {
                if let Some(addr) = addr {
                    self.affinity.remove(addr);
                    self.session_rooms.retain(|run, room| {
                        if room == addr {
                            self.attempts.remove(run);
                            false
                        } else {
                            true
                        }
                    });
                }
            }
            EventKind::RunFrozen => {
                self.attempts.remove(&run);
                self.session_rooms.remove(&run);
            }
            // The rest of the vocabulary says nothing about which
            // endpoint answers, or with which account, which is what
            // this book holds.
            _ => {}
        }
        Ok(())
    }

    /// The account which last answered successfully in this Session.
    #[must_use]
    pub fn session_account(
        &self,
        addr: &kernel::Address,
        provider: &str,
    ) -> Option<&kernel::ServerLabel> {
        self.affinity
            .get(addr)
            .and_then(|bindings| bindings.get(provider))
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
                AxCode::ModelUnchosen,
                format!("choose the {tag} model"),
                "no model is chosen for this tag",
            )
            .with_recovery("attach a provider on the settings page and pick a model for this tag")
        })?;
        let held = self.endpoints.get(&choice.endpoint).ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                format!("choose the {tag} model"),
                format!("the endpoint {} is no longer attached", choice.endpoint),
            )
            .with_recovery("pick a model from an attached endpoint")
        })?;
        let endpoint = &held.endpoint;
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
            transport: &held.transport,
        })
    }

    /// One warm-up for every attached endpoint, each sharing that
    /// endpoint's client slot, so the connection it opens is the one the
    /// endpoint's first call takes (`crates/gateway/spec/Endpoint/Transport.lean` D26).
    pub fn warm_ups(&self) -> impl Iterator<Item = WarmUp> + '_ {
        self.endpoints.values().map(|held| {
            WarmUp::new(
                held.transport.clone(),
                held.endpoint.client_shape(),
                held.endpoint.models_url(),
            )
        })
    }

    /// Resolves one attachment's account list before effects or replay folding.
    /// An omitted list preserves the existing declaration; an explicit list
    /// replaces it atomically. See `crates/gateway/spec/Router.lean` Accounts.
    #[must_use]
    pub fn accounts_for_attachment(
        &self,
        name: &str,
        incoming: Option<Vec<kernel::event::record::ProviderAccount>>,
    ) -> Option<Vec<kernel::event::record::ProviderAccount>> {
        incoming.or_else(|| {
            self.endpoints
                .get(name)
                .and_then(|held| held.endpoint.tuning.accounts.clone())
        })
    }

    pub fn endpoints(&self) -> impl Iterator<Item = &AttachedEndpoint> {
        self.endpoints.values().map(|held| &held.endpoint)
    }

    /// Puts back what an attach read about each model, from the bytes
    /// of the CAS blob `blob` (written by
    /// [`AttachedEndpoint::facts_bytes`]). Only an endpoint whose latest
    /// line names this blob learns it, and only for the ids that line
    /// admitted, so a later attach is never overwritten by an earlier
    /// reading.
    ///
    /// # Errors
    /// `E_WIRE_MISMATCH` when the bytes are not a list of model facts.
    pub fn learn(&mut self, blob: &kernel::B3Hash, bytes: &[u8]) -> Result<(), AxError> {
        let read: Vec<kernel::event::record::ModelFacts> = serde_json::from_slice(bytes)
            .map_err(|err| crate::mismatch::mismatch("model facts", &err.to_string()))?;
        for held in self.endpoints.values_mut() {
            if held.endpoint.facts_blob.as_ref() != Some(blob) {
                continue;
            }
            for row in &mut held.endpoint.models {
                if let Some(stated) = read.iter().find(|stated| stated.id == row.id) {
                    row.clone_from(stated);
                }
            }
        }
        Ok(())
    }

    /// What is chosen, tag by tag.
    pub fn choices(&self) -> impl Iterator<Item = (ModelTag, &str, &ModelEntry)> {
        self.chosen
            .iter()
            .map(|(tag, choice)| (*tag, choice.endpoint.as_str(), &choice.entry))
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
    use super::super::tuning::EndpointTuning;
    use super::*;
    use crate::endpoint::AuthSpec;
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
    /// One catalogue row as a probe that answered nothing but the id
    /// would have left it.
    use crate::endpoint::ModelFacts;
    use crate::provider::registry::ConnectionKind;

    fn facts(id: &str) -> ModelFacts {
        ModelFacts {
            id: id.to_owned(),
            ..ModelFacts::default()
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
            facts_blob: None,
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
        let mut book = EndpointBook::new();
        let endpoint = attached("house", base_url);
        book.apply(&record(
            EventKind::EndpointAttached,
            attached_payload(&endpoint).unwrap(),
        ))
        .unwrap();
        book.apply(&record(
            EventKind::ModelSelected,
            selected_payload(ModelTag::Main, "house", &entry("m-large"), None).unwrap(),
        ))
        .unwrap();
        book
    }

    /// An endpoint's max_in_flight rides its endpoint_attached line: the
    /// gate a replay builds admits that ceiling, and so does the gate a
    /// snapshot restore builds; a line without the key, as every line
    /// written before it existed, gives the default for its kind of
    /// connection.
    #[test]
    fn max_in_flight_survives_a_replay_and_a_snapshot() {
        let mut capped = attached("capped", "https://api.example.test/v1");
        capped.tuning.max_in_flight = Some(crate::concurrency::MaxInFlight::try_from(3).unwrap());
        let mut book = EndpointBook::new();
        for endpoint in [
            capped,
            attached("remote", "https://api.example.test/v1"),
            attached("local", "http://127.0.0.1:11434/v1"),
        ] {
            let mut line = attached_payload(&endpoint).unwrap().as_map().clone();
            if endpoint.name != "capped" {
                assert!(!line.contains_key("tuning"), "{line:?}");
                line.remove("tuning");
            }
            book.apply(&record(
                EventKind::EndpointAttached,
                Payload::new(line).unwrap(),
            ))
            .unwrap();
        }
        let restored: EndpointBook =
            serde_json::from_str(&serde_json::to_string(&book).unwrap()).unwrap();
        let limits = |book: &EndpointBook| {
            ["capped", "remote", "local"].map(|name| book.endpoints[name].transport.limit())
        };
        assert_eq!((limits(&book), limits(&restored)), ([3, 16, 4], [3, 16, 4]));
    }

    /// The per-account retry figure rides the endpoint_attached line as
    /// the word the person chose; a line without it, as every line
    /// written before it existed, and a line holding a word this build
    /// cannot read both take the one default `DEFAULTS` states, and
    /// neither refuses the line.
    #[test]
    fn account_retries_survive_a_replay_and_an_unset_line_takes_the_default() {
        use kernel::account_recovery::AccountRetries;
        let mut chosen = attached("chosen", "https://api.example.test/v1");
        chosen.tuning.account_retries = Some(AccountRetries::One);
        let written = attached_payload(&chosen).unwrap();
        assert_eq!(
            written
                .as_map()
                .get("tuning")
                .and_then(|tuning| tuning.get("account_retries")),
            Some(&Value::String("one".to_owned()))
        );
        let mut unreadable = attached_payload(&attached("odd", "https://api.example.test/v1"))
            .unwrap()
            .as_map()
            .clone();
        unreadable.insert(
            "tuning".to_owned(),
            serde_json::json!({"account_retries": "three"}),
        );
        let mut book = EndpointBook::new();
        for line in [
            written,
            attached_payload(&attached("unset", "https://api.example.test/v1")).unwrap(),
            Payload::new(unreadable).unwrap(),
        ] {
            book.apply(&record(EventKind::EndpointAttached, line))
                .unwrap();
        }
        let read = ["chosen", "unset", "odd"].map(|name| {
            let tuning = &book.endpoints[name].endpoint.tuning;
            (tuning.account_retries, tuning.account_retries())
        });
        assert_eq!(
            read,
            [
                (Some(AccountRetries::One), AccountRetries::One),
                (None, EndpointTuning::DEFAULTS.account_retries),
                (None, EndpointTuning::DEFAULTS.account_retries),
            ]
        );
        assert_eq!(
            EndpointTuning::DEFAULTS.account_retries,
            AccountRetries::Two
        );
    }

    /// A record written while this build carried a retreat arm names
    /// two keys nothing ever wrote a value into. Reading it must still
    /// produce the choice it states.
    #[test]
    fn a_record_naming_a_retreat_still_reads_as_the_choice_it_carries() {
        let mut book = EndpointBook::new();
        book.apply(&record(
            EventKind::EndpointAttached,
            attached_payload(&attached("house", "https://api.example.test/v1")).unwrap(),
        ))
        .unwrap();
        let mut map = selected_payload(ModelTag::Main, "house", &entry("m-large"), None)
            .unwrap()
            .as_map()
            .clone();
        map.insert(
            "fallback_endpoint".to_owned(),
            Value::String("spare".to_owned()),
        );
        map.insert(
            "fallback_model".to_owned(),
            Value::String("m-small".to_owned()),
        );
        book.apply(&record(
            EventKind::ModelSelected,
            Payload::new(map).unwrap(),
        ))
        .unwrap();
        let chosen = book
            .select(ModelTag::Main, &BuildingPolicy::default())
            .unwrap();
        assert_eq!(chosen.entry.id, "m-large");
    }

    #[test]
    fn a_tag_with_no_choice_says_what_to_do_about_it() {
        let err = EndpointBook::new()
            .select(ModelTag::Digest, &BuildingPolicy::default())
            .unwrap_err();
        assert_eq!(*err.code(), AxCode::ModelUnchosen);
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
    #[test]
    fn malformed_provider_accounts_are_refused_during_replay() {
        let mut payload = attached_payload(&attached("house", "https://api.example.test/v1"))
            .unwrap()
            .as_map()
            .clone();
        payload.insert(
            "tuning".to_owned(),
            serde_json::json!({
                "accounts": [{"id": "first", "reference": "plain"}]
            }),
        );
        let mut book = EndpointBook::new();
        assert!(
            book.absorb(
                EventKind::EndpointAttached,
                RunId::CITY,
                None,
                &Payload::new(payload).unwrap()
            )
            .is_err(),
            "an invalid explicit account must not silently fall back to legacy auth"
        );
        assert!(book.is_empty());
    }

    fn accounts() -> Vec<kernel::event::record::ProviderAccount> {
        ["first", "second"]
            .map(|id| kernel::event::record::ProviderAccount {
                id: kernel::ServerLabel::parse(id).unwrap(),
                reference: Some(kernel::SecretRef::new("providers", id).unwrap()),
                header: None,
            })
            .to_vec()
    }

    #[test]
    fn provider_accounts_preserve_explicit_reorder_and_reject_malformed_replay() {
        let mut endpoint = attached("house", "http://127.0.0.1:11434/v1");
        endpoint.tuning.accounts = Some(accounts());
        let mut book = EndpointBook::new();
        book.absorb(
            EventKind::EndpointAttached,
            RunId::CITY,
            None,
            &attached_payload(&endpoint).unwrap(),
        )
        .unwrap();
        endpoint.tuning.accounts.as_mut().unwrap().reverse();
        book.absorb(
            EventKind::EndpointAttached,
            RunId::CITY,
            None,
            &attached_payload(&endpoint).unwrap(),
        )
        .unwrap();
        endpoint.tuning.accounts = None;
        book.absorb(
            EventKind::EndpointAttached,
            RunId::CITY,
            None,
            &attached_payload(&endpoint).unwrap(),
        )
        .unwrap();
        let kept = book.endpoints().next().unwrap();
        assert_eq!(
            kept.tuning
                .accounts
                .as_ref()
                .unwrap()
                .iter()
                .map(|account| account.id.as_str())
                .collect::<Vec<_>>(),
            ["second", "first"]
        );
        assert_eq!(
            kept.first_auth().unwrap(),
            AuthSpec::Bearer(SecretRef::new("providers", "second").unwrap())
        );
        let encoded = serde_json::to_vec(&book).unwrap();
        let restored: EndpointBook = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(&book).unwrap()
        );
        let mut payload = attached_payload(kept).unwrap().as_map().clone();
        payload.get_mut("tuning").unwrap()["accounts"][0]["reference"] = serde_json::json!("plain");
        assert!(
            book.absorb(
                EventKind::EndpointAttached,
                RunId::CITY,
                None,
                &Payload::new(payload).unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn failed_account_attempts_never_replace_successful_session_affinity() {
        let mut book = EndpointBook::new();
        let addr = kernel::Address::parse("lab/session").unwrap();
        let run = RunId::from_bytes([9; 16]);
        book.absorb(EventKind::RunStarted, run, Some(&addr), &Payload::empty())
            .unwrap();
        let called = |account: &str| {
            Payload::of(&kernel::event::record::ModelCalled {
                model: "model".to_owned(),
                segments: Vec::new(),
                provider_account: Some(kernel::event::record::ProviderAccountBinding {
                    provider: "house".to_owned(),
                    account: kernel::ServerLabel::parse(account).unwrap(),
                }),
            })
            .unwrap()
        };
        book.absorb(EventKind::ModelCalled, run, Some(&addr), &called("first"))
            .unwrap();
        assert!(book.session_account(&addr, "house").is_none());
        book.absorb(EventKind::ModelReturned, run, None, &Payload::empty())
            .unwrap();
        book.absorb(EventKind::ModelCalled, run, Some(&addr), &called("second"))
            .unwrap();
        book.absorb(EventKind::RunFrozen, run, Some(&addr), &Payload::empty())
            .unwrap();
        assert_eq!(
            book.session_account(&addr, "house").unwrap().as_str(),
            "first"
        );
        let run = RunId::from_bytes([10; 16]);
        book.absorb(EventKind::RunStarted, run, Some(&addr), &Payload::empty())
            .unwrap();
        book.absorb(EventKind::ModelCalled, run, Some(&addr), &called("second"))
            .unwrap();
        book.absorb(EventKind::ModelReturned, run, None, &Payload::empty())
            .unwrap();
        assert_eq!(
            book.session_account(&addr, "house").unwrap().as_str(),
            "second"
        );
        book.absorb(
            EventKind::SessionOpened,
            RunId::CITY,
            Some(&addr),
            &Payload::empty(),
        )
        .unwrap();
        assert!(book.session_account(&addr, "house").is_none());
    }
    #[test]
    fn old_run_answer_must_not_bind_a_new_session() {
        let mut book = EndpointBook::new();
        let room = kernel::Address::parse("lab/room1").unwrap();
        let old_run = RunId::from_bytes([2; 16]);
        book.absorb(
            EventKind::RunStarted,
            old_run,
            Some(&room),
            &Payload::empty(),
        )
        .unwrap();
        let called = Payload::of(&kernel::event::record::ModelCalled {
            model: "fixture".to_owned(),
            segments: Vec::new(),
            provider_account: Some(kernel::event::record::ProviderAccountBinding {
                provider: "house".to_owned(),
                account: kernel::ServerLabel::parse("old").unwrap(),
            }),
        })
        .unwrap();
        book.absorb(EventKind::ModelCalled, old_run, None, &called)
            .unwrap();
        book.absorb(
            EventKind::SessionOpened,
            RunId::CITY,
            Some(&room),
            &Payload::empty(),
        )
        .unwrap();
        book.absorb(EventKind::ModelReturned, old_run, None, &Payload::empty())
            .unwrap();
        assert_eq!(book.session_account(&room, "house"), None);
    }
    proptest::proptest! {
        #[test]
        fn failed_attempt_traces_preserve_the_lean_binding(
            attempted in proptest::collection::vec(0u8..8, 0..32),
        ) {
            let mut book = EndpointBook::new();
            let addr = kernel::Address::parse("lab/session").unwrap();
            let run = RunId::from_bytes([9; 16]);
            book.absorb(EventKind::RunStarted, run, Some(&addr), &Payload::empty()).unwrap();
            let called = |id: &str| Payload::of(&kernel::event::record::ModelCalled {
                model: "fixture".to_owned(), segments: Vec::new(),
                provider_account: Some(kernel::event::record::ProviderAccountBinding {
                    provider: "house".to_owned(), account: kernel::ServerLabel::parse(id).unwrap(),
                }),
            }).unwrap();
            book.absorb(EventKind::ModelCalled, run, Some(&addr), &called("bound")).unwrap();
            book.absorb(EventKind::ModelReturned, run, None, &Payload::empty()).unwrap();
            for id in attempted {
                book.absorb(EventKind::ModelCalled, run, Some(&addr), &called(&format!("account{id}"))).unwrap();
                proptest::prop_assert_eq!(book.session_account(&addr, "house").unwrap().as_str(), "bound");
            }
            book.absorb(EventKind::RunFrozen, run, Some(&addr), &Payload::empty()).unwrap();
            proptest::prop_assert_eq!(book.session_account(&addr, "house").unwrap().as_str(), "bound");
        }
    }
}

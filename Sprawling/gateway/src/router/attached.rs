// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::DialectKind;

use crate::endpoint::{AuthSpec, ModelFacts};
use crate::provider::registry::ConnectionKind;

use super::payload::auth_reference;
use super::tuning::EndpointTuning;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttachedEndpoint {
    pub name: String,

    pub base_url: String,
    pub dialect: DialectKind,

    pub connection_kind: ConnectionKind,
    pub auth: AuthSpec,

    pub models: Vec<ModelFacts>,

    pub probed: bool,

    pub tuning: EndpointTuning,

    pub facts_blob: Option<kernel::B3Hash>,
}

impl AttachedEndpoint {
    pub fn facts_bytes(&self) -> Result<Vec<u8>, kernel::AxError> {
        serde_json::to_vec(&self.models)
            .map_err(|err| crate::mismatch::mismatch("model facts", &err.to_string()))
    }

    pub fn validate_legacy_fields(
        name: &str,
        accounts: Option<&[kernel::event::record::ProviderAccount]>,
        secret: Option<&str>,
        auth_header: Option<&str>,
    ) -> Result<(), kernel::AxError> {
        if accounts.is_some() && (secret.is_some() || auth_header.is_some()) {
            return Err(kernel::AxError::failure(
                kernel::AxCode::ConfigInvalid,
                "configure provider accounts",
                format!("{name} uses explicit accounts; a legacy credential has no account target"),
            )
            .with_recovery(
                "edit the named account's reference/header in accounts; omit legacy secret and auth_header fields",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn is_local(&self) -> bool {
        crate::is_local(&self.base_url)
    }

    #[must_use]
    pub fn has_credential(&self) -> bool {
        !self.references().is_empty()
    }

    #[must_use]
    pub fn references(&self) -> Vec<&kernel::SecretRef> {
        match &self.tuning.accounts {
            Some(accounts) => accounts
                .iter()
                .filter_map(|account| account.reference.as_ref())
                .collect(),
            None => auth_reference(&self.auth).into_iter().collect(),
        }
    }

    pub fn first_auth(&self) -> Result<AuthSpec, kernel::AxError> {
        Ok(self
            .account_auths()?
            .into_iter()
            .next()
            .map_or_else(|| self.auth.clone(), |(_, auth)| auth))
    }

    pub(crate) fn account_auths(
        &self,
    ) -> Result<Vec<(kernel::ServerLabel, AuthSpec)>, kernel::AxError> {
        super::tuning::validate_accounts(self.tuning.accounts.as_deref())?;
        Ok(self
            .tuning
            .accounts
            .iter()
            .flatten()
            .map(|account| {
                let auth = match &account.reference {
                    Some(reference) => AuthSpec::for_dialect(
                        self.dialect,
                        reference.clone(),
                        account.header.clone(),
                    ),
                    None => AuthSpec::None,
                };
                (account.id.clone(), auth)
            })
            .collect())
    }

    #[must_use]
    pub fn label(&self) -> &str {
        self.tuning.label.as_deref().unwrap_or(&self.name)
    }

    #[must_use]
    pub fn chat_url(&self) -> String {
        join(&self.base_url, chat_path(self.dialect))
    }

    pub(crate) fn client_shape(&self) -> crate::endpoint::ClientShape {
        crate::endpoint::ClientShape {
            url: self.chat_url(),
            proxying: self.tuning.proxying,

            timeout_ms: self.tuning.call_timeout_ms(),
        }
    }

    #[must_use]
    pub fn models_url(&self) -> String {
        join(&self.base_url, "models")
    }
}

fn chat_path(dialect: DialectKind) -> &'static str {
    match dialect {
        DialectKind::Anthropic => "messages",
        DialectKind::OpenAi => "chat/completions",
        DialectKind::OpenAiResponses => "responses",
    }
}

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
    fn generation_defaults_and_capability_corrections_survive_the_ledger_round_trip() {
        let mut endpoint = attached("house", "https://api.example.test/v1");
        endpoint.tuning.generation = vec![kernel::ModelGeneration {
            model: "m-small".to_owned(),
            settings: kernel::GenerationSettings {
                temperature: Some("0.2".to_owned()),
                ..kernel::GenerationSettings::default()
            },
            output_tokens: kernel::Ceiling::new(1024),
            support: kernel::GenerationSupport {
                top_k: Some(kernel::ParameterSupport::Unsupported),
                ..kernel::GenerationSupport::default()
            },
        }];
        let mut book = EndpointBook::new();
        book.apply(&record(
            EventKind::EndpointAttached,
            attached_payload(&endpoint).unwrap(),
        ))
        .unwrap();
        assert_eq!(
            book.endpoints().next().unwrap().tuning.generation,
            endpoint.tuning.generation
        );
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
    fn an_unreadable_probed_is_refused_rather_than_read_as_probed() {
        let endpoint = attached("house", "https://api.example.test/v1");
        let mut map = attached_payload(&endpoint).unwrap().as_map().clone();
        map.insert("probed".to_owned(), serde_json::Value::from("no"));
        assert!(super::super::payload::read_attached(&Payload::new(map).unwrap()).is_err());
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

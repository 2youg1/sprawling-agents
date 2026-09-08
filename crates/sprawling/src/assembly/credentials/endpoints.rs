// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this city may call: an endpoint probed before it is attached,
//! a model chosen for a tag, and the local server the environment names
//! when nothing else is registered.

use kernel::Payload;
use kernel::{AxCode, AxError, EventKind};

use super::super::RunWorker;
use super::{
    Ceilings, Chosen, ENVIRONMENT_ENDPOINT, Entered, PROBE_TIMEOUT_MS, dialect_headers,
    local_model_facts,
};

impl RunWorker {
    /// Asks a base URL what it serves, and attaches nothing.
    ///
    /// The list is recorded rather than returned: a query would have to
    /// make this blocking call on the socket's own task, and the answer
    /// is a fact about what this city can reach - which is the kind of
    /// thing the ledger holds.
    pub(in crate::assembly) fn probe_endpoint(&mut self, entered: Entered) -> Result<(), AxError> {
        let endpoint = self.endpoint_of(entered)?;
        let models = self.probe(&endpoint)?;
        let mut map = serde_json::Map::new();
        map.insert(
            "name".to_owned(),
            serde_json::Value::String(endpoint.name.clone()),
        );
        map.insert(
            "base_url".to_owned(),
            serde_json::Value::String(endpoint.base_url.clone()),
        );
        map.insert(
            "models".to_owned(),
            serde_json::Value::Array(
                models
                    .iter()
                    .map(|id| serde_json::Value::String(id.clone()))
                    .collect(),
            ),
        );
        self.record(EventKind::EndpointProbed, Payload::new(map)?)
    }

    /// The endpoint a form describes, before anybody has asked it
    /// anything. One reading of the four fields, so a probe and the
    /// attachment that follows it cannot disagree about what they are
    /// talking to.
    fn endpoint_of(&self, entered: Entered) -> Result<gateway::AttachedEndpoint, AxError> {
        let Entered {
            name,
            base_url,
            dialect,
            secret,
            auth_header,
        } = entered;
        let auth = match secret {
            None => gateway::AuthSpec::None,
            Some(raw) => {
                let reference = kernel::SecretRef::parse(&raw)?;
                match auth_header {
                    Some(header) => gateway::AuthSpec::Header {
                        name: header,
                        value: reference,
                    },
                    None => gateway::AuthSpec::Bearer(reference),
                }
            }
        };
        Ok(gateway::AttachedEndpoint {
            name,
            base_url,
            dialect,
            auth,
            models: Vec::new(),
        })
    }

    /// Registers what the person entered, after asking the endpoint what
    /// it serves. The probe happens before the record: an endpoint that
    /// cannot be reached is not attached, so the book never advertises a
    /// model nobody can call.
    ///
    /// `admit` narrows what is registered to the models the person
    /// ticked. An empty list admits everything the endpoint serves,
    /// which is what somebody who never asked for the list meant; a name
    /// on the list that the endpoint does not serve is left out rather
    /// than promised, the same answer a reading room gives a skill that
    /// is not on the shelves.
    pub(in crate::assembly) fn attach_endpoint(
        &mut self,
        entered: Entered,
        admit: &[String],
    ) -> Result<(), AxError> {
        let mut endpoint = self.endpoint_of(entered)?;
        let served = self.probe(&endpoint)?;
        endpoint.models = if admit.is_empty() {
            served
        } else {
            served
                .into_iter()
                .filter(|id| admit.iter().any(|wanted| wanted == id))
                .collect()
        };
        self.note(
            runtime::diagnostics::Level::Effect,
            "gateway::router",
            &format!(
                "{} at {} serves {} model(s)",
                endpoint.name,
                endpoint.base_url,
                endpoint.models.len()
            ),
        );
        let payload = gateway::attached_payload(&endpoint)?;
        self.record(EventKind::EndpointAttached, payload)
    }

    fn probe(&self, endpoint: &gateway::AttachedEndpoint) -> Result<Vec<String>, AxError> {
        let probe = gateway::Endpoint::new(
            gateway::EndpointConfig {
                base_url: endpoint.chat_url(),
                dialect: endpoint.dialect,
                model: String::new(),
                auth: endpoint.auth.clone(),
                extra_headers: dialect_headers(endpoint.dialect),
                overrides: Vec::new(),
                timeout_ms: PROBE_TIMEOUT_MS,
                pricing: None,
            },
            self.resolver(),
        )?;
        probe.list_models(&endpoint.models_url())
    }

    /// Points one tag at one model. The two token counts come from the
    /// person because no provider's model list carries them, and a
    /// number invented here would outrank the one that bills.
    pub(in crate::assembly) fn select_model(
        &mut self,
        chosen: Chosen,
        ceilings: Ceilings,
    ) -> Result<(), AxError> {
        let Chosen {
            endpoint,
            model,
            tag,
        } = chosen;
        let Ceilings {
            context_tokens,
            max_output_tokens,
        } = ceilings;
        let known = self
            .book
            .endpoints()
            .find(|candidate| candidate.name == endpoint)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "choose a model",
                    format!("{endpoint} is not attached"),
                )
                .with_recovery("attach the endpoint first, then choose one of the models it lists")
            })?;
        if !known.models.contains(&model) {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "choose a model",
                format!("{endpoint} does not serve {model}"),
            )
            .with_recovery("choose one of the models the endpoint listed")
            .with_nearby(known.models.clone()));
        }
        let priced = gateway::MarketSnapshot::builtin().lookup(&model).cloned();
        // Zero means "take the catalogue's figure". A person choosing a
        // model in the settings page has no business typing a context
        // window: the ceiling is a fact about the model, and a number
        // invented on a form would end runs for a reason that appears
        // nowhere in the account.
        let context_tokens = match context_tokens {
            0 => priced.as_ref().map_or(0, |row| row.context_tokens),
            stated => stated,
        };
        let max_output_tokens = match max_output_tokens {
            0 => priced.as_ref().map_or(0, |row| row.max_output_tokens),
            stated => stated,
        };
        let entry = gateway::ModelEntry {
            id: model,
            context_tokens,
            max_output_tokens,
            // Prices come from the pinned catalog when it knows the
            // model and are zero when it does not: an unpriced call is
            // reported as unpriced rather than as free-looking guesswork.
            input_price: priced
                .as_ref()
                .map(|row| row.input_price)
                .unwrap_or_default(),
            output_price: priced
                .as_ref()
                .map(|row| row.output_price)
                .unwrap_or_default(),
            cache_read_price: priced
                .as_ref()
                .map(|row| row.cache_read_price)
                .unwrap_or_default(),
            cache_write_price: priced.map(|row| row.cache_write_price).unwrap_or_default(),
        };
        let payload = gateway::selected_payload(tag, &endpoint, &entry)?;
        self.record(EventKind::ModelSelected, payload)
    }

    fn seed_from_environment(&mut self, base_url: &str, model: &str) -> Result<(), AxError> {
        let facts = local_model_facts(model)?;
        self.attach_endpoint(
            Entered {
                name: ENVIRONMENT_ENDPOINT.to_owned(),
                base_url: base_url.to_owned(),
                dialect: kernel::DialectKind::OpenAi,
                secret: None,
                auth_header: None,
            },
            &[],
        )?;
        self.select_model(
            Chosen {
                endpoint: ENVIRONMENT_ENDPOINT.to_owned(),
                model: model.to_owned(),
                tag: kernel::ModelTag::Main,
            },
            Ceilings {
                context_tokens: facts.context_tokens,
                max_output_tokens: facts.max_output_tokens,
            },
        )
    }

    /// Records what the vault turned out to be, and registers the local
    /// server named in the environment when nothing is registered yet.
    ///
    /// The environment path is a convenience, not a second authority:
    /// it writes the same two records the settings page writes, so the
    /// book stays the only statement of what this city can call. A
    /// failure here is reported and not fatal — a city stays readable
    /// without a provider.
    pub(crate) fn open_for_service(&mut self, vault_notice: Option<Payload>) {
        if let Some(notice) = vault_notice
            && let Err(err) = self.record(EventKind::ProviderDegraded, notice)
        {
            self.note(
                runtime::diagnostics::Level::Refuse,
                "bin::assembly",
                &format!("{err}; {}", err.recovery()),
            );
        }
        if !self.book.is_empty() {
            return;
        }
        let (Ok(base_url), Ok(model)) = (
            std::env::var("SPRAWLING_MODEL_URL"),
            std::env::var("SPRAWLING_MODEL"),
        ) else {
            return;
        };
        if let Err(err) = self.seed_from_environment(&base_url, &model) {
            self.note(
                runtime::diagnostics::Level::Refuse,
                "bin::assembly",
                &format!(
                    "the model named in the environment is not attached: {err}; {}",
                    err.recovery()
                ),
            );
        }
    }
}

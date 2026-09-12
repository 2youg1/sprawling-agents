// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this city may call: an endpoint probed before it is attached,
//! a model chosen for a tag, and the local server the environment names
//! when nothing else is registered.

use kernel::{AxCode, AxError, EventKind, Payload};

use super::super::RunWorker;
use super::probing::{Probing, probed_payload, reach_of};
use super::{
    Ceilings, Chosen, Credential, ENVIRONMENT_ENDPOINT, Entered, PROBE_TIMEOUT_MS, dialect_headers,
    local_model_facts,
};

impl RunWorker {
    /// Asks a base URL what it serves, and attaches nothing.
    ///
    /// The reading is recorded rather than returned: a query would have
    /// to make this blocking call on the socket's own task, and what a
    /// probe learns is a fact about what this city can reach - which is
    /// the kind of thing the ledger holds.
    ///
    /// **A probe that could not read a model list still answers.** Where
    /// the call stopped is the finding a person acts on: a name that
    /// does not resolve, a socket nothing answers and a 401 are three
    /// different next steps, and a refusal returned instead of a record
    /// would leave the form with one sentence from a transport library.
    /// The record carries the stage, and the refusal's own code and
    /// subject beside it.
    ///
    /// # Errors
    /// A payload the ledger will not take, and a base URL whose
    /// credential reference cannot be parsed - neither of which a probe
    /// can report on, because neither got as far as a request.
    pub(in crate::assembly) fn probe_endpoint(&mut self, entered: Entered) -> Result<(), AxError> {
        let endpoint = self.endpoint_of(entered)?;
        let found = Probing {
            reach: reach_of(&endpoint.base_url)?,
            served: self.probe(&endpoint),
        };
        let payload = probed_payload(&endpoint.name, &endpoint.base_url, &found)?;
        self.record(EventKind::EndpointProbed, payload)
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
            credential,
            tuning,
        } = entered;
        let auth = match credential {
            Credential::Absent => gateway::AuthSpec::None,
            // Which header a key travels in is the compatible format's
            // own answer, so this page does not give a second one.
            Credential::Key { reference, header } => gateway::AuthSpec::for_dialect(
                dialect,
                kernel::SecretRef::parse(&reference)?,
                header,
            ),
            Credential::Subscription { reference } => {
                gateway::AuthSpec::Bearer(kernel::SecretRef::parse(&reference)?)
            }
        };
        Ok(gateway::AttachedEndpoint {
            name,
            base_url,
            dialect,
            auth,
            models: Vec::new(),
            probed: false,
            tuning,
        })
    }

    /// Registers what the person entered, asking the endpoint what it
    /// serves first.
    ///
    /// `admit` narrows what is registered to the models the person
    /// ticked. An empty list admits everything the endpoint serves,
    /// which is what somebody who never asked for the list meant; a name
    /// on the list that the endpoint does not serve is left out rather
    /// than promised, the same answer a reading room gives a skill that
    /// is not on the shelves.
    ///
    /// A probe that fails does not stop the attachment when the person
    /// named the ids: most compatible endpoints serve no model list at
    /// all, so refusing here would keep a working provider out of the
    /// city over an interface it never promised. What the failure costs
    /// is `probed`, which records that this list is the person's word
    /// rather than the endpoint's.
    ///
    /// # Errors
    /// A probe that fails with nothing declared: the city would have no
    /// model id to call.
    pub(in crate::assembly) fn attach_endpoint(
        &mut self,
        entered: Entered,
        admit: &[String],
    ) -> Result<(), AxError> {
        let mut endpoint = self.endpoint_of(entered)?;
        let unprobed = match self.probe(&endpoint) {
            Ok(served) => {
                endpoint.probed = true;
                endpoint.models = served
                    .into_iter()
                    .map(|row| row.id)
                    .filter(|id| admit.is_empty() || admit.iter().any(|wanted| wanted == id))
                    .collect();
                None
            }
            Err(err) if admit.is_empty() => {
                return Err(err.with_recovery("name the model ids to admit, then attach again"));
            }
            Err(err) => {
                endpoint.models = admit.to_vec();
                Some(err.subject().to_owned())
            }
        };
        // One line either way, and it says which of the two happened.
        // The unprobed attachment is degraded rather than refused - the
        // registration went through - so it stays at `effect`, where a
        // `refuse` line would tell the person their endpoint was turned
        // away.
        self.note(
            runtime::diagnostics::Level::Effect,
            "gateway::router",
            &match unprobed {
                None => format!(
                    "{} at {} serves {} model(s)",
                    endpoint.name,
                    endpoint.base_url,
                    endpoint.models.len()
                ),
                Some(why) => format!(
                    "{} at {} lists no models ({why}); attached on the {} the person named",
                    endpoint.name,
                    endpoint.base_url,
                    endpoint.models.len()
                ),
            },
        );
        let payload = gateway::attached_payload(&endpoint)?;
        self.record(EventKind::EndpointAttached, payload)
    }

    /// What the endpoint says it serves, read for every fact each row
    /// states.
    ///
    /// The request is made the way a call to this endpoint would be -
    /// same headers, same deadline - because a probe that reached a
    /// gateway without the header that gateway requires answers 401 for
    /// a key that is in fact good. A body override is left off: it
    /// belongs to a chat request, and a model list takes no body.
    ///
    /// `request_max_retries` is honoured here, and here only: a model
    /// call's retries are the watchdog's decision (gateway-SPEC.md
    /// 8-NN), while a person watching a settings page is waiting on this
    /// one request and a network that dropped it once is worth asking
    /// again.
    fn probe(
        &self,
        endpoint: &gateway::AttachedEndpoint,
    ) -> Result<Vec<gateway::ModelFacts>, AxError> {
        let tuning = &endpoint.tuning;
        let mut extra_headers = dialect_headers(endpoint.dialect);
        extra_headers.extend(tuning.extra_headers.iter().cloned());
        let probe = gateway::Endpoint::new(
            gateway::EndpointConfig {
                base_url: endpoint.chat_url(),
                dialect: endpoint.dialect,
                model: String::new(),
                auth: endpoint.auth.clone(),
                extra_headers,
                overrides: Vec::new(),
                timeout_ms: tuning.timeout_ms.unwrap_or(PROBE_TIMEOUT_MS),
                stream_deadline_ms: None,
                pricing: None,
            },
            // A probe asks which models an endpoint serves. It carries
            // no conversation, so it carries no picture either.
            gateway::Redemption::without_images(self.resolver()),
        )?;
        let url = endpoint.models_url();
        let mut attempts_left = tuning.request_max_retries.unwrap_or(0);
        loop {
            match probe.list_models(&url) {
                Ok(served) => return Ok(served),
                Err(err) if attempts_left > 0 && err.is_retriable() => {
                    attempts_left = attempts_left.saturating_sub(1);
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// What an adapter redeems at the wire: the credential this city
    /// holds, and the pictures its content store holds.
    ///
    /// The store is opened once here rather than once per picture: a
    /// conversation carrying four pictures used to open four handles on
    /// one immutable directory (sprawling-SPEC.md 8-50). The handle is
    /// this adapter's own rather than the worker's, because the worker's
    /// is needed elsewhere while a call is out.
    ///
    /// The mutex is for the type rather than for contention: the picture
    /// face must be `Send + Sync`, and `memory::Cas` is only `Send`.
    ///
    /// # Errors
    /// Refuses a city whose content store will not open. That refusal
    /// arrives while the adapter is being built rather than half way
    /// through a conversation.
    pub(in crate::assembly) fn redemption(&self) -> Result<gateway::Redemption, AxError> {
        let cas_dir = self.city_root.join(".sprawling").join("cas");
        let store = std::sync::Arc::new(std::sync::Mutex::new(
            memory::Cas::open(&cas_dir).map_err(|err| {
                AxError::failure(AxCode::InvalidArgs, "read a picture", err.to_string())
                    .with_recovery("make the city's content store readable, then dispatch again")
            })?,
        ));
        Ok(gateway::Redemption::new(
            self.resolver(),
            std::sync::Arc::new(move |at: &kernel::Locator| {
                let kernel::Locator::Cas { hash, .. } = at else {
                    return Err(AxError::failure(
                        AxCode::InvalidArgs,
                        "read a picture",
                        at.to_string(),
                    )
                    .with_recovery("a picture is referred to by a `cas:` locator"));
                };
                let held = store.lock().map_err(|_| {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "read a picture",
                        "the content store handle was poisoned",
                    )
                    .with_recovery("restart the server; nothing in the store was changed")
                })?;
                held.get(hash).map_err(memory::MemoryError::into_ax)
            }),
        ))
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
        // What the person stated outranks the catalogue, and what
        // neither states stays unstated. **The old reading of an unknown
        // model was zero**, which the OpenAI wire wrote out as
        // `max_tokens: 0` and a provider answered with no content at
        // all; the run then froze as work that finished. A ceiling this
        // city cannot name is now carried as one it cannot name.
        let context_tokens = match context_tokens {
            0 => priced.as_ref().map_or(0, |row| row.context_tokens),
            stated => stated,
        };
        let max_output_tokens =
            max_output_tokens.or_else(|| priced.as_ref().and_then(|row| row.max_output_tokens));
        let entry = gateway::ModelEntry {
            id: model,
            context_tokens,
            max_output_tokens,
            // What a model accepts is the catalogue's fact, not a
            // person's: a form cannot make a text-only model see.
            input: priced.as_ref().map(|row| row.input).unwrap_or_default(),
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
        // `Fallback::None` until a person can say otherwise: the wire
        // has no field for a retreat yet, and a default that switched
        // someone's model would be the one decision a default may not
        // make.
        let payload = gateway::selected_payload(tag, &endpoint, &entry, &gateway::Fallback::None)?;
        self.record(EventKind::ModelSelected, payload)
    }

    fn seed_from_environment(&mut self, base_url: &str, model: &str) -> Result<(), AxError> {
        let facts = local_model_facts(model)?;
        self.attach_endpoint(
            Entered {
                name: ENVIRONMENT_ENDPOINT.to_owned(),
                base_url: base_url.to_owned(),
                dialect: kernel::DialectKind::OpenAi,
                credential: Credential::Absent,
                tuning: gateway::EndpointTuning::default(),
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

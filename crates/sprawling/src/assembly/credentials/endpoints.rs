// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this city may call: an endpoint probed before it is attached,
//! and a model chosen for a tag.

use kernel::{AxCode, AxError, EventKind};

use super::super::RunWorker;
use super::probing::{Probing, probed_payload, reach_of};
use super::{Credential, Entered, PROBE_TIMEOUT_MS, dialect_headers, hint_of};

mod choosing;

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
        let entered = entered.resolved()?;
        let endpoint = self.endpoint_of(entered)?;
        let found = Probing {
            reach: reach_of(&endpoint.base_url, endpoint.tuning.proxying)?,
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
        // Resolved here and nowhere later: what a person set up is
        // settled at the moment they set it up, and a reader that
        // worked it out again from the writer would be answering a
        // narrower question than the one it was asked.
        let connection_kind = gateway::resolve_connection(hint_of(dialect), None)?;
        Ok(gateway::AttachedEndpoint {
            name,
            base_url,
            dialect,
            connection_kind,
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
        let entered = entered.resolved()?;
        let mut endpoint = self.endpoint_of(entered)?;
        let unprobed = match self.probe(&endpoint) {
            Ok(served) => {
                endpoint.probed = true;
                endpoint.models = served
                    .into_iter()
                    .filter(|row| admit.is_empty() || admit.contains(&row.id))
                    .collect();
                None
            }
            Err(err) if admit.is_empty() => {
                return Err(err.rewrite_recovery("name the model ids to admit, then attach again"));
            }
            Err(err) => {
                // The probe never answered, so the only fact this city
                // has about these rows is that a person named them.
                // Everything else stays unstated rather than invented.
                endpoint.models = admit
                    .iter()
                    .map(|id| gateway::ModelFacts {
                        id: id.clone(),
                        context_tokens: None,
                        max_output_tokens: None,
                        input_modalities: Vec::new(),
                        input_price: None,
                        output_price: None,
                    })
                    .collect();
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
                stream_idle_timeout_ms: None,
                pricing: None,
                proxying: tuning.proxying,
            },
            // A probe asks which models an endpoint serves. It carries
            // no conversation, so it carries no picture either.
            gateway::Redemption::without_images(self.resolver()),
        )?;
        let url = endpoint.models_url();
        let mut attempts_left = tuning.request_max_retries.without_a_brake();
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
    use crate::assembly::fixture::{fake_openai, worker_with_provider};
    use crate::assembly::init_city;

    /// The two rungs this layer owns: what the person sends now, and
    /// what the book already holds for the same model. The rungs above
    /// and below them - the provider's own model list and the city's
    /// policy default - are decided in `gateway::provider::ceiling`,
    /// where all four are tested together.
    #[test]
    fn an_empty_ceiling_keeps_the_one_this_model_was_registered_with() {
        let dir = tempfile::tempdir().unwrap();
        init_city(dir.path()).unwrap();
        let (base_url, _provider) = fake_openai(&["m-1"], Vec::new());
        // Attaches and picks `m-1` at 32_768 / 4_096, which is the row
        // a person fills in on the settings page.
        let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();
        // The same pick again with both boxes empty, which is what the
        // model dropdown sends when nobody edited the two figures.
        worker
            .handle(channels::Command::SelectModel {
                endpoint: channels::ProviderName::parse("house").unwrap(),
                model: "m-1".to_owned(),
                tag: kernel::ModelTag::Main,
                context_tokens: None,
                max_output_tokens: None,
                idem: kernel::IdemKey::derive(
                    &kernel::RunId::CITY,
                    kernel::Seq::FIRST,
                    b"re-picked",
                ),
            })
            .unwrap();
        let (_, _, entry) = worker.book.choices().next().unwrap();
        assert_eq!(
            entry.max_output_tokens,
            kernel::Ceiling::new(4_096),
            "re-picking a model erased the ceiling the person entered"
        );
        assert_eq!(entry.context_tokens, 32_768);
    }
}

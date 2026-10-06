// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this city may call: an endpoint probed before it is attached,
//! and a model chosen for a tag.

use kernel::{AxCode, AxError, EventKind};

use super::super::RunWorker;
use super::probing::{Probing, probed_payload, reach_of};
use super::{Credential, Entered, PROBE_TIMEOUT_MS, dialect_headers};

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
    pub(in crate::worker) fn probe_endpoint(&mut self, entered: Entered) -> Result<(), AxError> {
        let entered = entered.resolved()?;
        let endpoint = self.endpoint_of(entered)?;
        let found = Probing {
            reach: reach_of(&endpoint.base_url, endpoint.tuning.proxying, self.monotonic)?,
            served: self.probe(&endpoint),
        };
        let payload = probed_payload(&endpoint.name, &endpoint.base_url, found)?;
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
            mut tuning,
        } = entered;
        tuning.accounts = self
            .credentials
            .book
            .accounts_for_attachment(&name, tuning.accounts);
        let (secret, header) = match &credential {
            Credential::Absent { header } => (None, header.as_deref()),
            Credential::Key { reference, header } => (Some(reference.as_str()), header.as_deref()),
        };
        gateway::AttachedEndpoint::validate_legacy_fields(
            &name,
            tuning.accounts.as_deref(),
            secret,
            header,
        )?;
        let auth = match credential {
            Credential::Absent { header } => self.kept_credential(&name, dialect, header),
            Credential::Key { reference, header } => gateway::AuthSpec::for_dialect(
                dialect,
                kernel::SecretRef::parse(&reference)?,
                header,
            ),
        };
        // Resolved here and nowhere later: what a person set up is
        // settled at the moment they set it up, and a reader that
        // worked it out again from the writer would be answering a
        // narrower question than the one it was asked.
        let connection_kind = gateway::resolve_connection(gateway::DialectHint::of(dialect))?;
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

    /// The credential this city already keeps for an endpoint of this
    /// name, as it should travel now.
    ///
    /// **An empty key box means "leave the key alone", never "remove
    /// it"** (`crates/sprawling/Spec.lean` §8-81). The form answers the vault once
    /// and holds the reference only while it is mounted, so every
    /// later visit says nothing about the credential; reading that as
    /// `AuthSpec::None` probed without a key and wrote the endpoint
    /// back without one. Removing a credential needs a command of its
    /// own, which the wire does not carry yet.
    ///
    /// The reference is kept and the header is worked out again by
    /// `AuthSpec::for_dialect` (`gateway::endpoint::auth`), because the same key moves between faces: one archived as
    /// `Authorization: Bearer` under the chat face travels as
    /// `x-api-key` under the messages face, and carrying the old
    /// spelling over answers 401 for a key that is good.
    fn kept_credential(
        &self,
        name: &str,
        dialect: kernel::DialectKind,
        header: Option<String>,
    ) -> gateway::AuthSpec {
        let archived = self
            .credentials
            .book
            .endpoints()
            .find(|endpoint| endpoint.name == name)
            .map(|endpoint| &endpoint.auth);
        let reference = match archived {
            Some(gateway::AuthSpec::Bearer(reference)) => reference.clone(),
            Some(gateway::AuthSpec::Header { value, .. }) => value.clone(),
            Some(gateway::AuthSpec::None) | None => return gateway::AuthSpec::None,
        };
        gateway::AuthSpec::for_dialect(dialect, reference, header)
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
    pub(in crate::worker) fn attach_endpoint(
        &mut self,
        entered: Entered,
        admit: &[String],
    ) -> Result<(), AxError> {
        let entered = entered.resolved()?;
        let explicit_accounts = entered.tuning.accounts.is_some();
        let mut endpoint = self.endpoint_of(entered)?;
        let kept = self
            .credentials
            .book
            .endpoints()
            .find(|kept| kept.name == endpoint.name)
            .cloned();
        if explicit_accounts && let Some(kept) = kept {
            let mut comparing = endpoint.tuning.clone();
            comparing.accounts = kept.tuning.accounts.clone();
            if endpoint.base_url == kept.base_url
                && endpoint.dialect == kept.dialect
                && comparing == kept.tuning
                && admit
                    == kept
                        .models
                        .iter()
                        .map(|model| model.id.clone())
                        .collect::<Vec<_>>()
            {
                endpoint.models = kept.models.clone();
                endpoint.probed = kept.probed;
                return self.record(
                    EventKind::EndpointAttached,
                    gateway::attached_payload(&endpoint)?,
                );
            }
        }
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
                    "{} at {} lists no models ({why}); attached on the {} the User named",
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
    /// call's retries are the watchdog's decision (`crates/gateway/Spec.lean`
    /// §8-16), while a person watching a settings page is waiting on this
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
                auth: endpoint.first_auth()?,
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
                // Listing models is read-only, so an unknown effect is
                // asked again as readily as a known one.
                Err(err) if attempts_left > 0 && err.retry() != kernel::Retry::No => {
                    attempts_left = attempts_left.saturating_sub(1);
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// What an adapter redeems at the wire: the credential this city
    /// holds, and the pictures its content store holds.
    ///
    /// The store is opened once here rather than once per picture, so a
    /// conversation carrying four pictures holds one handle on one
    /// immutable directory rather than four (`crates/sprawling/Spec.lean` §8-171). The handle is
    /// this adapter's own rather than the worker's, because the worker's
    /// is needed elsewhere while a call is out.
    ///
    /// The mutex is for the type rather than for contention: the picture
    /// face must be `Send + Sync`, and `storage::Cas` is only `Send`.
    ///
    /// # Errors
    /// Refuses a city whose content store will not open. That refusal
    /// arrives while the adapter is being built rather than half way
    /// through a conversation.
    pub(in crate::worker) fn redemption(&self) -> Result<gateway::Redemption, AxError> {
        let cas_dir = self.city_root.join(".sprawling").join("cas");
        let store = std::sync::Arc::new(std::sync::Mutex::new(
            storage::Cas::open(&cas_dir).map_err(|err| {
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
                held.get(hash).map_err(storage::StorageError::into_ax)
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
    use crate::worker::fixture::{fake_openai, worker_with_provider};

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(32))]
        #[test]
        fn account_submission_traces_preserve_explicit_authority(
            updates in proptest::collection::vec(
                (proptest::option::of(1usize..5), proptest::bool::ANY, proptest::bool::ANY),
                0..12,
            ),
        ) {
            use kernel::{IdemKey, RunId, Seq, ServerLabel};
            let dir = tempfile::tempdir().unwrap();
            let report = crate::worker::fixture::init_city(dir.path()).unwrap();
            let (base_url, provider) = fake_openai(&["m-1"], vec![]);
            let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();
            let accounts = |count: usize| (0..count).map(|index| {
                kernel::event::record::ProviderAccount {
                    id: ServerLabel::parse(&format!("account{index}")).unwrap(),
                    reference: None,
                    header: None,
                }
            }).collect::<Vec<_>>();
            let mut held = Some(accounts(1));
            let command = |incoming, secret, header, marker: &[u8]| wire::Command::AttachEndpoint {
                name: wire::ProviderName::parse("house").unwrap(),
                base_url: base_url.clone(),
                dialect: kernel::DialectKind::OpenAi,
                secret,
                auth_header: header,
                admit: vec!["m-1".to_owned()],
                tuning: wire::EndpointTuning { accounts: incoming, ..Default::default() },
                idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, marker),
            };
            worker.handle(command(held.clone(), None, None, b"migrate")).unwrap();
            for (index, (incoming, secret, header)) in updates.into_iter().enumerate() {
                let incoming = incoming.map(accounts);
                let before = serde_json::to_value(&worker.credentials.book).unwrap();
                let calls = provider.exchanges().len();
                let ledger = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
                let result = worker.handle(command(
                    incoming.clone(),
                    secret.then(|| "secret:fixture/c".to_owned()),
                    header.then(|| "X-Legacy".to_owned()),
                    format!("update-{index}").as_bytes(),
                ));
                if secret || header {
                    proptest::prop_assert_eq!(*result.unwrap_err().code(), kernel::AxCode::ConfigInvalid);
                    proptest::prop_assert_eq!(serde_json::to_value(&worker.credentials.book).unwrap(), before);
                    proptest::prop_assert_eq!(provider.exchanges().len(), calls);
                    proptest::prop_assert_eq!(
                        runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap().raw_lines().to_vec(),
                        ledger.raw_lines(),
                    );
                } else {
                    result.unwrap();
                    held = incoming.or(held);
                }
                let actual = worker.credentials.book.endpoints().find(|endpoint| endpoint.name == "house")
                    .unwrap().tuning.accounts.clone();
                proptest::prop_assert_eq!(&actual, &held);
                proptest::prop_assert!(actual.is_some());
            }
        }
    }

    #[test]
    fn explicit_accounts_refuse_legacy_probe_fields_before_effects() {
        let dir = tempfile::tempdir().unwrap();
        let report = crate::worker::fixture::init_city(dir.path()).unwrap();
        let (base_url, provider) = fake_openai(&["m-1"], vec![]);
        let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();
        for id in ["a", "b"] {
            worker
                .handle(wire::Command::PutSecret {
                    realm: "fixture".to_owned(),
                    name: id.to_owned(),
                    value: kernel::Sealed::new(Box::new(format!("fixture-{id}"))),
                })
                .unwrap();
        }
        worker
            .handle(wire::Command::AttachEndpoint {
                name: wire::ProviderName::parse("house").unwrap(),
                base_url: base_url.clone(),
                dialect: kernel::DialectKind::OpenAi,
                secret: None,
                auth_header: None,
                admit: vec!["m-1".to_owned()],
                tuning: wire::EndpointTuning {
                    accounts: Some(
                        ["a", "b"]
                            .map(|id| kernel::event::record::ProviderAccount {
                                id: kernel::ServerLabel::parse(id).unwrap(),
                                reference: Some(kernel::SecretRef::new("fixture", id).unwrap()),
                                header: Some("X-Account".to_owned()),
                            })
                            .to_vec(),
                    ),
                    ..Default::default()
                },
                idem: kernel::IdemKey::derive(&kernel::RunId::CITY, kernel::Seq::FIRST, b"migrate"),
            })
            .unwrap();
        worker
            .handle(wire::Command::ProbeEndpoint {
                name: wire::ProviderName::parse("house").unwrap(),
                base_url: base_url.clone(),
                dialect: kernel::DialectKind::OpenAi,
                secret: None,
                auth_header: None,
                tuning: wire::EndpointTuning::default(),
                idem: kernel::IdemKey::derive(
                    &kernel::RunId::CITY,
                    kernel::Seq::FIRST,
                    b"probe-migrated",
                ),
            })
            .unwrap();
        assert!(
            provider
                .exchanges()
                .last()
                .unwrap()
                .to_ascii_lowercase()
                .contains("x-account: fixture-a")
        );
        let before = serde_json::to_value(&worker.credentials.book).unwrap();
        let calls = provider.exchanges();
        let ledger = runtime::replay::verify_ledger_dir(&report.ledger_dir)
            .unwrap()
            .raw_lines()
            .to_vec();
        for (index, (secret, header)) in [
            (Some("secret:fixture/old"), None),
            (None, Some("X-Legacy")),
            (Some(""), None),
            (None, Some("")),
        ]
        .into_iter()
        .enumerate()
        {
            let result = worker.handle(wire::Command::ProbeEndpoint {
                name: wire::ProviderName::parse("house").unwrap(),
                base_url: base_url.clone(),
                dialect: kernel::DialectKind::OpenAi,
                secret: secret.map(str::to_owned),
                auth_header: header.map(str::to_owned),
                tuning: wire::EndpointTuning::default(),
                idem: kernel::IdemKey::derive(
                    &kernel::RunId::CITY,
                    kernel::Seq::FIRST,
                    format!("legacy-probe-{index}").as_bytes(),
                ),
            });
            assert_eq!(
                *result
                    .expect_err("legacy fields have no account target")
                    .code(),
                kernel::AxCode::ConfigInvalid
            );
            assert_eq!(
                serde_json::to_value(&worker.credentials.book).unwrap(),
                before
            );
            assert_eq!(provider.exchanges(), calls);
            assert_eq!(
                runtime::replay::verify_ledger_dir(&report.ledger_dir)
                    .unwrap()
                    .raw_lines(),
                ledger.as_slice()
            );
        }
        drop(worker);
        let resumed = super::RunWorker::new(
            dir.path(),
            runtime::diagnostics::Diagnostics::off(),
            crate::worker::fixture::hands(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&resumed.credentials.book).unwrap(),
            before
        );
    }

    /// The two rungs this layer owns: what the person sends now, and
    /// what the book already holds for the same model. The rungs above
    /// and below them - the provider's own model list and the city's
    /// policy default - are decided in `gateway::provider::ceiling`,
    /// where all four are tested together.
    #[test]
    fn an_empty_ceiling_keeps_the_one_this_model_was_registered_with() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let (base_url, _provider) = fake_openai(&["m-1"], Vec::new());
        // Attaches and picks `m-1` at 32_768 / 4_096, which is the row
        // a person fills in on the settings page.
        let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();
        // The same pick again with both boxes empty, which is what the
        // model dropdown sends when nobody edited the two figures.
        worker
            .handle(wire::Command::SelectModel {
                endpoint: wire::ProviderName::parse("house").unwrap(),
                model: "m-1".to_owned(),
                tag: kernel::ModelTag::Main,
                context_tokens: None,
                max_output_tokens: None,
                input: None,
                idem: kernel::IdemKey::derive(
                    &kernel::RunId::CITY,
                    kernel::Seq::FIRST,
                    b"re-picked",
                ),
            })
            .unwrap();
        let (_, _, entry) = worker.credentials.book.choices().next().unwrap();
        assert_eq!(
            entry.max_output_tokens,
            kernel::Ceiling::new(4_096),
            "re-picking a model erased the ceiling the person entered"
        );
        assert_eq!(entry.context_tokens, 32_768);
    }

    /// On the chat face a model nobody stated a ceiling for is called
    /// with none, so the provider's own default for that model applies
    /// (`crates/gateway/Spec.lean` §8-17), and the account says who picked it.
    #[test]
    fn a_chat_face_model_nobody_stated_a_ceiling_for_is_left_to_the_provider() {
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let (base_url, _provider) = fake_openai(&["m-1", "m-2"], Vec::new());
        let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();
        worker
            .handle(wire::Command::SelectModel {
                endpoint: wire::ProviderName::parse("house").unwrap(),
                model: "m-2".to_owned(),
                tag: kernel::ModelTag::Main,
                context_tokens: None,
                max_output_tokens: None,
                input: None,
                idem: kernel::IdemKey::derive(
                    &kernel::RunId::CITY,
                    kernel::Seq::FIRST,
                    b"picked with nothing stated",
                ),
            })
            .unwrap();
        let (_, _, entry) = worker.credentials.book.choices().next().unwrap();
        assert_eq!(entry.id, "m-2");
        assert_eq!(entry.max_output_tokens, None);
    }
    #[test]
    fn provider_accounts_keep_the_successful_session_after_reorder_and_restart() {
        use kernel::event::record::ProviderAccount;
        use kernel::{Address, IdemKey, RunId, Seq, ServerLabel};
        let dir = tempfile::tempdir().unwrap();
        crate::worker::fixture::init_city(dir.path()).unwrap();
        let (base_url, provider) = fake_openai(
            &["m-1"],
            vec![
                crate::worker::fixture::completion("answered", None),
                crate::worker::fixture::completion("answered", None),
                crate::worker::fixture::completion("answered", None),
                crate::worker::fixture::completion("new session", None),
            ],
        );
        let mut worker = worker_with_provider(dir.path(), &base_url, "m-1").unwrap();
        let accounts = ["one", "two"].map(|name| ProviderAccount {
            id: ServerLabel::parse(name).unwrap(),
            reference: Some(kernel::SecretRef::new("fixture", name).unwrap()),
            header: None,
        });
        let enrol = |worker: &mut crate::worker::RunWorker| {
            for name in ["one", "two"] {
                worker
                    .handle(wire::Command::PutSecret {
                        realm: "fixture".to_owned(),
                        name: name.to_owned(),
                        value: kernel::Sealed::new(Box::new(format!("fixture-value-{name}"))),
                    })
                    .unwrap();
            }
        };
        enrol(&mut worker);
        let attach =
            |worker: &mut crate::worker::RunWorker, order: Vec<ProviderAccount>, marker: &[u8]| {
                worker
                    .handle(wire::Command::AttachEndpoint {
                        name: wire::ProviderName::parse("house").unwrap(),
                        base_url: base_url.clone(),
                        dialect: kernel::DialectKind::OpenAi,
                        secret: None,
                        auth_header: None,
                        admit: vec!["m-1".to_owned()],
                        tuning: wire::EndpointTuning {
                            accounts: Some(order),
                            ..wire::EndpointTuning::default()
                        },
                        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, marker),
                    })
                    .unwrap();
            };
        let dispatch = |worker: &mut crate::worker::RunWorker, marker: &[u8]| {
            worker
                .handle(wire::Command::Dispatch {
                    addr: Address::parse("lab/room1").unwrap(),
                    task: "answer once".to_owned(),
                    goal: "one answer".to_owned(),
                    policy: kernel::RunPolicy::of(kernel::Mode::Work),
                    idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, marker),
                    session: None,
                    effort: None,
                    model: None,
                })
                .unwrap();
        };
        attach(&mut worker, accounts.to_vec(), b"account-order-one");
        dispatch(&mut worker, b"account-run-one");
        let recorded = runtime::replay::verify_ledger_dir(
            &kernel::layout::CityLayout::new(dir.path()).ledger(),
        )
        .unwrap();
        let called = recorded
            .lines()
            .iter()
            .filter_map(|line| match line {
                runtime::replay::VerifiedLine::Known { record, .. }
                    if record.kind() == kernel::EventKind::ModelCalled =>
                {
                    Some(
                        record
                            .data()
                            .read::<kernel::event::record::ModelCalled>()
                            .unwrap()
                            .provider_account,
                    )
                }
                runtime::replay::VerifiedLine::Known { .. }
                | runtime::replay::VerifiedLine::IgnoredUnknown { .. } => None,
            })
            .collect::<Vec<_>>();
        assert!(
            called.iter().any(|binding| binding
                .as_ref()
                .is_some_and(|binding| binding.account.as_str() == "one")),
            "production calls must record account identity: {called:?}"
        );
        assert_eq!(
            worker
                .credentials
                .book
                .session_account(&Address::parse("lab/room1").unwrap(), "house")
                .map(ServerLabel::as_str),
            Some("one"),
            "first successful call must enter the live binding projection"
        );
        attach(
            &mut worker,
            accounts.iter().rev().cloned().collect(),
            b"account-order-two",
        );
        dispatch(&mut worker, b"account-run-two");
        drop(worker);
        let mut worker = crate::worker::RunWorker::new(
            dir.path(),
            runtime::diagnostics::Diagnostics::off(),
            crate::worker::fixture::hands(),
        )
        .unwrap();
        worker.read_volume_with(crate::worker::fixture::roomy_volume);
        enrol(&mut worker);
        dispatch(&mut worker, b"account-run-three");
        worker
            .handle(wire::Command::OpenSession {
                addr: Address::parse("lab/room1").unwrap(),
                carry: wire::Carry::Nothing,
                from: None,
                idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"account-session-new"),
            })
            .unwrap();
        dispatch(&mut worker, b"account-run-four");
        let auth = provider
            .exchanges()
            .into_iter()
            .filter(|request| request.starts_with("POST "))
            .map(|request| {
                request
                    .lines()
                    .find(|line| line.to_ascii_lowercase().starts_with("authorization:"))
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            auth,
            [
                vec!["authorization: Bearer fixture-value-one".to_owned(); 3],
                vec!["authorization: Bearer fixture-value-two".to_owned()]
            ]
            .concat()
        );
    }
}

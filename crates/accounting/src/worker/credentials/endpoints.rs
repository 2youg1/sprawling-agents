// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What this city may call: an endpoint probed before it is attached,
//! and a model chosen for a tag.

use std::collections::BTreeSet;

use kernel::event::record::Listing;
use kernel::{AxCode, AxError, EventKind};

use super::super::RunWorker;
use super::probing::{Probing, probed_payload, reach_of};
use super::{Credential, Entered, PROBE_TIMEOUT_MS, dialect_headers};
use crate::worker::commanding::door::Finishing;
use crate::worker::errand::Answer;

mod admitting;
mod choosing;

pub(in crate::worker) use admitting::Admission;
use admitting::Remembered;

/// What a `ProbeEndpoint` read off the accounting thread, on its way
/// back to be recorded: the endpoint as the form named it, and the
/// staged reading with the model list, or the refusal that stopped the
/// reading before any request.
pub(crate) struct Probed {
    name: String,
    base_url: String,
    found: Result<Probing, AxError>,
    finishing: Finishing,
}

/// What an `AttachEndpoint` asked the endpoint off the accounting
/// thread, on its way back to be registered: the endpoint, the admission
/// the person gave, and the model list or why it could not be read.
pub(crate) struct Attaching {
    endpoint: gateway::AttachedEndpoint,
    admission: Admission,
    kept: Option<Box<gateway::AttachedEndpoint>>,
    served: Result<Vec<gateway::ModelFacts>, AxError>,
    finishing: Finishing,
}

/// One request for an endpoint's model list, built on the accounting
/// thread from what the city holds and sent from a thread of its own.
/// Named apart from the kernel's `Listing`, which says where a model
/// row came from.
struct ListRequest {
    probe: gateway::Endpoint,
    url: String,
    attempts_left: u32,
}

impl ListRequest {
    /// What the endpoint says it serves, asked again while the network
    /// dropped the request and a retry is left.
    ///
    /// # Errors
    /// The last refusal: a retry that is not allowed, or none left.
    fn read(self) -> Result<Vec<gateway::ModelFacts>, AxError> {
        let ListRequest {
            probe,
            url,
            mut attempts_left,
        } = self;
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
}

/// A re-attach whose probe read no list: why, and how many of the
/// admitted rows kept what an earlier attach read.
struct Unread {
    why: String,
    kept: usize,
}

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
    /// **The requests leave the accounting thread.** The reading and the
    /// model list are asked from a thread of their own, and the record
    /// is written by [`Self::record_probe`] when they answer, so a slow
    /// endpoint holds up nothing but this probe
    /// (`crates/sprawling/Spec.lean` §8-42-9).
    ///
    /// # Errors
    /// A base URL whose credential reference cannot be parsed, and a
    /// thread that will not start - neither of which a probe can report
    /// on, because neither got as far as a request. A payload the
    /// ledger will not take, and a transport this machine will not
    /// build, are handed back once the probe answers.
    pub(in crate::worker) fn probe_endpoint(&mut self, entered: Entered) -> Result<(), AxError> {
        let entered = entered.resolved()?;
        let endpoint = self.endpoint_of(entered)?;
        let listing = self.listing(&endpoint);
        let monotonic = self.monotonic;
        let proxying = endpoint.tuning.proxying;
        let gateway::AttachedEndpoint { name, base_url, .. } = endpoint;
        let finishing = self.finishing();
        self.flight.errands.send(move || {
            let found = reach_of(&base_url, proxying, monotonic).map(|reach| Probing {
                reach,
                served: listing.and_then(ListRequest::read),
            });
            Answer::Probed(Probed {
                name,
                base_url,
                found,
                finishing,
            })
        })
    }

    /// Writes the `endpoint_probed` line a probe answered with, under
    /// the key of the command that asked for it.
    pub(in crate::worker) fn record_probe(&mut self, probed: Probed) {
        let Probed {
            name,
            base_url,
            found,
            finishing,
        } = probed;
        self.finish(finishing, |worker| {
            let payload = probed_payload(&name, &base_url, found?)?;
            worker.record(EventKind::EndpointProbed, payload)
        });
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
            facts_blob: None,
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
    /// `admission` holds the rows the person ticked on the endpoint's
    /// list and the ids they typed (`crates/gateway/spec/Endpoint/Models.lean`
    /// D47). Asking for nothing admits everything the endpoint serves,
    /// which is what somebody who never asked for the list meant; a
    /// ticked id the endpoint does not serve is left out rather than
    /// promised, the same answer a reading room gives a skill that is not
    /// on the shelves; a typed id is registered whether or not the list
    /// carries it, because it is the person's word.
    ///
    /// A probe that fails does not stop the attachment when the person
    /// named the ids: most compatible endpoints serve no model list at
    /// all, so refusing here would keep a working provider out of the
    /// city over an interface it never promised. Nor does it change what
    /// the city already knew (gateway D46): an id already registered
    /// under this name keeps the row the book holds, and only an id
    /// named for the first time is registered on the person's word,
    /// which `probed` then records as false.
    ///
    /// The probe is asked from a thread of its own, and the endpoint is
    /// registered by [`Self::finish_attach`] when it answers
    /// (`crates/sprawling/Spec.lean` §8-42-9). An attachment that
    /// changes nothing the city keeps asks nothing and is registered at
    /// once.
    ///
    /// # Errors
    /// A thread that will not start. Once the probe answers: a probe
    /// that failed with nothing declared - the city would have no model
    /// id to call - a ticked list none of whose ids the endpoint serves
    /// with nothing typed (gateway D47), and rows that leave out a model
    /// a tag points at (gateway D48). Each refusal comes before anything
    /// is written.
    pub(in crate::worker) fn attach_endpoint(
        &mut self,
        entered: Entered,
        admission: &Admission,
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
        if explicit_accounts && let Some(kept) = &kept {
            let mut comparing = endpoint.tuning.clone();
            comparing.accounts = kept.tuning.accounts.clone();
            let ids = |ids: Vec<String>| ids.into_iter().collect::<BTreeSet<String>>();
            if endpoint.base_url == kept.base_url
                && endpoint.dialect == kept.dialect
                && comparing == kept.tuning
                && ids(admission.asked())
                    == ids(kept.models.iter().map(|model| model.id.clone()).collect())
            {
                endpoint.models = kept.models.clone();
                endpoint.probed = kept.probed;
                return self.record_attached(endpoint);
            }
        }
        let listing = self.listing(&endpoint);
        let (admission, finishing) = (admission.clone(), self.finishing());
        self.flight.errands.send(move || {
            let served = listing.and_then(ListRequest::read);
            Answer::Attached(Attaching {
                endpoint,
                admission,
                kept: kept.map(Box::new),
                served,
                finishing,
            })
        })
    }

    /// Registers the endpoint an `AttachEndpoint` probed, under the key
    /// of the command that asked for it.
    pub(in crate::worker) fn finish_attach(&mut self, attaching: Attaching) {
        let Attaching {
            endpoint,
            admission,
            kept,
            served,
            finishing,
        } = attaching;
        self.finish(finishing, |worker| {
            worker.attach_served(endpoint, &admission, kept.as_deref(), served)
        });
    }

    /// Registers `endpoint` with the models `served` lists that `admit`
    /// lets in, or with the ids `admit` names when the list could not be
    /// read.
    ///
    /// # Errors
    /// A list that could not be read with nothing declared.
    fn attach_served(
        &mut self,
        mut endpoint: gateway::AttachedEndpoint,
        admission: &Admission,
        kept: Option<&gateway::AttachedEndpoint>,
        served: Result<Vec<gateway::ModelFacts>, AxError>,
    ) -> Result<(), AxError> {
        let unprobed = match served {
            Ok(served) => {
                endpoint.models = admission.of_the_list(&endpoint.name, served)?;
                endpoint.probed = endpoint
                    .models
                    .iter()
                    .all(|row| row.listing == Listing::Listed);
                None
            }
            Err(err) if admission.asks_nothing() => {
                return Err(err.rewrite_recovery("name the model ids to admit, then attach again"));
            }
            Err(err) => {
                // The probe never answered this time. A row registered
                // before keeps what an earlier attach recorded; of a new
                // id the only fact this city has is that a person named
                // it, and everything else stays unstated rather than
                // invented.
                let earlier = kept.as_ref().map_or(&[][..], |kept| kept.models.as_slice());
                let Remembered { rows, kept: still } = admission.unread(earlier);
                endpoint.probed =
                    kept.as_ref().is_some_and(|kept| kept.probed) && still == rows.len();
                endpoint.models = rows;
                Some(Unread {
                    why: err.subject().to_owned(),
                    kept: still,
                })
            }
        };
        self.credentials.book.refuse_dropping_a_role(&endpoint)?;
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
                Some(Unread { why, kept }) => format!(
                    "{} at {}: the model list could not be read this time ({why}); attached on \
                     the {} model(s) the User named, {kept} of them keeping what an earlier \
                     attach read",
                    endpoint.name,
                    endpoint.base_url,
                    endpoint.models.len()
                ),
            },
        );
        self.record_attached(endpoint)
    }

    /// Writes what the attach read into the CAS, the line that names it,
    /// and hands the facts back to the worker's own book, which folded
    /// the line to ids only (`crates/gateway/spec/Endpoint/Models.lean` §8-10).
    fn record_attached(&mut self, mut endpoint: gateway::AttachedEndpoint) -> Result<(), AxError> {
        let (blob, bytes) = crate::listed_facts::keep(&mut self.cas, &mut endpoint)?;
        self.record(
            EventKind::EndpointAttached,
            gateway::attached_payload(&endpoint)?,
        )?;
        self.credentials.book.learn(&blob, &bytes)
    }

    /// The request that reads what the endpoint serves, for every fact
    /// each row states; [`Listing::read`] sends it.
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
    fn listing(&self, endpoint: &gateway::AttachedEndpoint) -> Result<ListRequest, AxError> {
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
        Ok(ListRequest {
            probe,
            url: endpoint.models_url(),
            attempts_left: tuning.request_max_retries.without_a_brake(),
        })
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
                named: Vec::new(),
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
                named: Vec::new(),
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

    /// The two facts this layer hands the ladder that matter here: what
    /// the person sends now, and what the book already holds for the
    /// same model. Every rung is decided in `gateway::provider::ceiling`
    /// and tested together there; this test is the production path that
    /// carries a figure the person stated into the next empty pick.
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
                        named: Vec::new(),
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

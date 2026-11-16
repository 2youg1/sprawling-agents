// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

pub(crate) struct Probed {
    name: String,
    base_url: String,
    found: Result<Probing, AxError>,
    finishing: Finishing,
}

pub(crate) struct Attaching {
    endpoint: gateway::AttachedEndpoint,
    admission: Admission,
    kept: Option<Box<gateway::AttachedEndpoint>>,
    served: Result<Vec<gateway::ModelFacts>, AxError>,
    finishing: Finishing,
}

struct ListRequest {
    probe: gateway::Endpoint,
    url: String,
    attempts_left: u32,
}

impl ListRequest {
    fn read(self) -> Result<Vec<gateway::ModelFacts>, AxError> {
        let ListRequest {
            probe,
            url,
            mut attempts_left,
        } = self;
        loop {
            match probe.list_models(&url) {
                Ok(served) => return Ok(served),

                Err(err) if attempts_left > 0 && err.retry() != kernel::Retry::No => {
                    attempts_left = attempts_left.saturating_sub(1);
                }
                Err(err) => return Err(err),
            }
        }
    }
}

struct Unread {
    why: String,
    kept: usize,
}

impl RunWorker {
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
                gateway::validate_generation(&endpoint)?;
                return self.record_attached(endpoint);
            }
        }
        let listing = self.listing(&endpoint);
        let (admission, finishing) = (admission.clone(), self.finishing());
        self.flight.errands.send(move || {
            let served = listing.and_then(ListRequest::read);
            Answer::Attached(Box::new(Attaching {
                endpoint,
                admission,
                kept: kept.map(Box::new),
                served,
                finishing,
            }))
        })
    }

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
        gateway::validate_generation(&endpoint)?;
        self.credentials.book.refuse_dropping_a_role(&endpoint)?;

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

    fn record_attached(&mut self, mut endpoint: gateway::AttachedEndpoint) -> Result<(), AxError> {
        let (blob, bytes) = crate::listed_facts::keep(&mut self.cas, &mut endpoint)?;
        self.record(
            EventKind::EndpointAttached,
            gateway::attached_payload(&endpoint)?,
        )?;
        self.credentials.book.learn(&blob, &bytes)
    }

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
            gateway::Redemption::without_images(self.resolver()),
        )?;
        Ok(ListRequest {
            probe,
            url: endpoint.models_url(),
            attempts_left: tuning.request_max_retries.without_a_brake(),
        })
    }

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
mod tests;

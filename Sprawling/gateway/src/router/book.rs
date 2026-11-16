// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::event::record::EndpointLost;
use kernel::{AxCode, AxError, BuildingPolicy, EventKind, EventRecord, ModelTag, Payload};

use crate::concurrency::vendor_in_flight;
use crate::endpoint::{Transport, WarmUp};
use crate::market::ModelEntry;

use super::attached::AttachedEndpoint;
use super::payload::{read_attached, read_choice};

#[derive(Debug, Clone, Copy)]
pub struct Chosen<'b> {
    pub endpoint: &'b AttachedEndpoint,
    pub entry: &'b ModelEntry,

    pub(crate) transport: &'b Transport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Choice {
    pub(crate) endpoint: String,
    pub(crate) entry: ModelEntry,
}

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(from = "Kept", into = "Kept")]
struct Held {
    endpoint: AttachedEndpoint,
    transport: Transport,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Kept {
    endpoint: AttachedEndpoint,
}

impl From<Kept> for Held {
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

    pub fn apply(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.absorb(record.kind(), record.run(), record.addr(), record.data())
    }

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

            _ => {}
        }
        Ok(())
    }

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

    pub fn select_ref(
        &self,
        reference: &kernel::ModelRef,
        policy: &BuildingPolicy,
    ) -> Result<Chosen<'_>, AxError> {
        let tag = self
            .choices()
            .find_map(|(tag, endpoint, entry)| {
                (endpoint == reference.endpoint && entry.id == reference.model).then_some(tag)
            })
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::ConfigInvalid,
                    "choose a workspace model",
                    format!("{reference} is not registered on an attached endpoint"),
                )
                .with_recovery(
                    "register this endpoint/model pair under a tag on the settings page, or \
                     correct the workspace policy",
                )
            })?;
        self.select(tag, policy)
    }

    pub fn warm_ups(&self) -> impl Iterator<Item = WarmUp> + '_ {
        self.endpoints.values().map(|held| {
            WarmUp::new(
                held.transport.clone(),
                held.endpoint.client_shape(),
                held.endpoint.models_url(),
            )
        })
    }

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

    pub fn choices(&self) -> impl Iterator<Item = (ModelTag, &str, &ModelEntry)> {
        self.chosen
            .iter()
            .map(|(tag, choice)| (*tag, choice.endpoint.as_str(), &choice.entry))
    }

    #[must_use]
    pub fn registered(&self, tag: ModelTag, endpoint: &str, model: &str) -> Option<&ModelEntry> {
        self.chosen
            .get(&tag)
            .filter(|choice| choice.endpoint == endpoint && choice.entry.id == model)
            .map(|choice| &choice.entry)
    }

    pub fn refuse_dropping_a_role(&self, attaching: &AttachedEndpoint) -> Result<(), AxError> {
        let dropped: Vec<String> = self
            .chosen
            .iter()
            .filter(|(_, choice)| {
                choice.endpoint == attaching.name
                    && !attaching.models.iter().any(|row| row.id == choice.entry.id)
            })
            .map(|(tag, choice)| {
                format!("{} is {tag}; choose another {tag} first", choice.entry.id)
            })
            .collect();
        if dropped.is_empty() {
            return Ok(());
        }
        Err(AxError::failure(
            AxCode::ConfigInvalid,
            "attach an endpoint",
            dropped.join("; "),
        )
        .with_nearby(attaching.models.iter().map(|row| row.id.clone()).collect())
        .with_recovery(
            "choose another model for that role, then attach again with the model left out",
        ))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.endpoints.is_empty() && self.chosen.is_empty()
    }
}
#[cfg(test)]
mod tests;

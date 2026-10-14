// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which model rows an attach registers (`crates/gateway/spec/Endpoint/Models.lean`
//! §8-10, D38 and D39).

use kernel::event::record::Listing;
use kernel::{AxCode, AxError};

/// What the person asked an attach to admit: the rows they ticked on
/// the endpoint's list, and the ids they typed.
#[derive(Clone)]
pub(in crate::worker) struct Admission {
    pub(in crate::worker) ticked: Vec<String>,
    pub(in crate::worker) named: Vec<String>,
}

/// The rows an attach registers when the endpoint's list could not be
/// read, and how many of them kept the row an earlier attach registered.
pub(super) struct Remembered {
    pub(super) rows: Vec<gateway::ModelFacts>,
    pub(super) kept: usize,
}

impl Admission {
    /// Nothing ticked and nothing typed: everything the endpoint serves.
    pub(in crate::worker) fn everything_served() -> Admission {
        Admission {
            ticked: Vec::new(),
            named: Vec::new(),
        }
    }

    /// Whether the person asked for nothing, which is what somebody who
    /// never looked at the list meant: everything it serves.
    pub(super) fn asks_nothing(&self) -> bool {
        self.ticked.is_empty() && self.named.is_empty()
    }

    /// Every id the person asked for, ticked first, each once.
    pub(super) fn asked(&self) -> Vec<String> {
        let mut asked: Vec<String> = Vec::new();
        for id in self.ticked.iter().chain(&self.named) {
            if !asked.contains(id) {
                asked.push(id.clone());
            }
        }
        asked
    }

    /// The rows an attach registers from the list the endpoint served.
    ///
    /// The listed rows the person asked for keep the list's order and
    /// facts; a typed id the list does not carry follows them on the
    /// person's word. A ticked id the list does not carry is left out:
    /// it was picked from a list, and the list no longer offers it.
    ///
    /// # Errors
    /// `E_CONFIG_INVALID` when ticked ids were sent, none of them is
    /// served, and nothing was typed: registering that would leave the
    /// endpoint with no models, and the next pick would fail for a
    /// reason that names something else.
    pub(super) fn of_the_list(
        &self,
        endpoint: &str,
        served: Vec<gateway::ModelFacts>,
    ) -> Result<Vec<gateway::ModelFacts>, AxError> {
        if self.asks_nothing() {
            return Ok(served);
        }
        let asked = self.asked();
        let (mut rows, offered): (Vec<gateway::ModelFacts>, Vec<gateway::ModelFacts>) =
            served.into_iter().partition(|row| asked.contains(&row.id));
        if rows.is_empty() && self.named.is_empty() {
            return Err(AxError::failure(
                AxCode::ConfigInvalid,
                "attach an endpoint",
                format!("{endpoint} does not serve {}", self.ticked.join(", ")),
            )
            .with_nearby(offered.into_iter().map(|row| row.id).collect())
            .with_recovery(
                "tick a model the endpoint lists, or type the id into the box for ids it does                  not list, then attach again",
            ));
        }
        let unlisted: Vec<gateway::ModelFacts> = asked
            .iter()
            .filter(|id| self.named.contains(id) && !rows.iter().any(|row| row.id == **id))
            .map(|id| named_row(id))
            .collect();
        rows.extend(unlisted);
        Ok(rows)
    }

    /// The rows an attach registers when no list could be read (D38):
    /// an id registered before under this endpoint keeps the row the
    /// book holds, and any other id is registered on the person's word.
    pub(super) fn unread(&self, earlier: &[gateway::ModelFacts]) -> Remembered {
        let registered = |id: &String| earlier.iter().find(|row| row.id == *id);
        let asked = self.asked();
        Remembered {
            kept: asked.iter().filter(|id| registered(id).is_some()).count(),
            rows: asked
                .iter()
                .map(|id| registered(id).cloned().unwrap_or_else(|| named_row(id)))
                .collect(),
        }
    }
}

/// A row the person named and no list stated: its id, and nothing else,
/// because nobody asked the provider about it.
fn named_row(id: &str) -> gateway::ModelFacts {
    gateway::ModelFacts {
        id: id.to_owned(),
        listing: Listing::Named,
        ..gateway::ModelFacts::default()
    }
}

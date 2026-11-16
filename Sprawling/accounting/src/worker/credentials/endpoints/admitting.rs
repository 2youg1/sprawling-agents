// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::Listing;
use kernel::{AxCode, AxError};

#[derive(Clone)]
pub(in crate::worker) struct Admission {
    pub(in crate::worker) ticked: Vec<String>,
    pub(in crate::worker) named: Vec<String>,
}

pub(super) struct Remembered {
    pub(super) rows: Vec<gateway::ModelFacts>,
    pub(super) kept: usize,
}

impl Admission {
    pub(in crate::worker) fn everything_served() -> Admission {
        Admission {
            ticked: Vec::new(),
            named: Vec::new(),
        }
    }

    pub(super) fn asks_nothing(&self) -> bool {
        self.ticked.is_empty() && self.named.is_empty()
    }

    pub(super) fn asked(&self) -> Vec<String> {
        let mut asked: Vec<String> = Vec::new();
        for id in self.ticked.iter().chain(&self.named) {
            if !asked.contains(id) {
                asked.push(id.clone());
            }
        }
        asked
    }

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

fn named_row(id: &str) -> gateway::ModelFacts {
    gateway::ModelFacts {
        id: id.to_owned(),
        listing: Listing::Named,
        ..gateway::ModelFacts::default()
    }
}

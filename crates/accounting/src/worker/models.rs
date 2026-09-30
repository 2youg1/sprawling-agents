// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The production `crate::ModelFactory`, and the one door that
//! swaps it for another (accounting-SPEC.md 8-1).

use kernel::AxError;

use super::RunWorker;
use super::credentials::dialect_headers;

/// Reaches a chosen model the way the endpoint book describes it: the
/// dialect's own headers, then `gateway::adapter_for`.
pub struct GatewayModels;

impl crate::ModelFactory for GatewayModels {
    fn build(
        &self,
        chosen: &gateway::Chosen<'_>,
        redemption: gateway::Redemption,
    ) -> Result<Box<dyn kernel::Model + Send>, AxError> {
        gateway::adapter_for(
            chosen,
            redemption,
            dialect_headers(chosen.endpoint.dialect)
                .into_iter()
                .map(|(name, value)| (name, value.spelled()))
                .collect(),
        )
    }
}

impl RunWorker {
    /// The same worker, reaching every model through `models` instead of
    /// the endpoint book's own adapters.
    ///
    /// The door citysim and the dispatch tests drive a worker through:
    /// what a run's model says is theirs to script, while choosing the
    /// model, renewing its credential and accounting for the run stay
    /// the worker's.
    #[must_use]
    pub fn with_models(self, models: Box<dyn crate::ModelFactory + Send>) -> RunWorker {
        RunWorker { models, ..self }
    }
}

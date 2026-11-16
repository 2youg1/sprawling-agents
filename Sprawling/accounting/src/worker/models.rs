// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;

use super::RunWorker;
use super::credentials::dialect_headers;

pub(crate) struct GatewayModels {
    pub(crate) monotonic: fn() -> std::time::Instant,
}

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
            self.monotonic,
        )
    }
}

impl RunWorker {
    #[must_use]
    pub fn with_models(self, models: Box<dyn crate::ModelFactory + Send>) -> RunWorker {
        RunWorker { models, ..self }
    }
}

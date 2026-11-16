// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;

use super::holding::Views;

impl Views {
    pub fn transcriber(
        &self,
        secrets: gateway::SecretResolver,
    ) -> Result<gateway::Transcriber, AxError> {
        let chosen = self.book.select(
            kernel::ModelTag::Transcribe,
            &kernel::BuildingPolicy::new(false),
        )?;
        gateway::transcriber_for(&chosen, secrets)
    }
}

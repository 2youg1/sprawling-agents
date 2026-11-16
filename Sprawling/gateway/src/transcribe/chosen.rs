// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;

use crate::endpoint::SecretResolver;
use crate::router::Chosen;

use super::transcriber::{Transcriber, TranscriberConfig};

const TRANSCRIBE_TIMEOUT_MS: u64 = 120_000;

pub fn transcriber_for(
    chosen: &Chosen<'_>,
    secrets: SecretResolver,
) -> Result<Transcriber, AxError> {
    Transcriber::attach(
        TranscriberConfig {

            base_url: chosen.endpoint.base_url.clone(),
            model: chosen.entry.id.clone(),
            auth: chosen.endpoint.first_auth()?,
            timeout_ms: TRANSCRIBE_TIMEOUT_MS,
            proxying: chosen.endpoint.tuning.proxying,
        },
        secrets,
    )
}

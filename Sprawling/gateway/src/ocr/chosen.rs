// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;

use kernel::{AxCode, AxError, Locator};

use crate::endpoint::{Redemption, SecretResolver};
use crate::router::Chosen;

use super::recogniser::{Recogniser, Seeing, Shown, held};

pub fn recogniser_for(
    chosen: &Chosen<'_>,
    secrets: SecretResolver,
    dialect_headers: Vec<(String, String)>,
    monotonic: fn() -> std::time::Instant,
) -> Result<Recogniser, AxError> {
    let shown: Shown = Arc::default();
    let showing = Arc::clone(&shown);
    let images = Arc::new(move |at: &Locator| {
        held(&showing)?
            .as_ref()
            .and_then(|picture| picture.bytes_at(at))
            .map(<[u8]>::to_vec)
            .ok_or_else(|| {
                AxError::failure(AxCode::InvalidArgs, "show a picture", at.to_string())
                    .with_recovery(
                        "this facility shows only the picture it was handed; report this \
                         against gateway::ocr",
                    )
            })
    });
    let model = crate::adapter_for(
        chosen,
        Redemption::new(secrets, images),
        dialect_headers,
        monotonic,
    )?;
    Ok(Recogniser::attached(Seeing {
        model,
        model_id: chosen.entry.id.clone(),
        max_tokens: chosen.entry.max_output_tokens,
        shown,
    }))
}

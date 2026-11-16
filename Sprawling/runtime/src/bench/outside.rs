// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, Effect, TaintSet, TaintSource};

pub(super) fn entered(effect: &Effect, taint: &TaintSet) -> Result<TaintSet, AxError> {
    let label = match effect {
        Effect::Connector { label } => format!("mcp:{}", label.as_str()),
        Effect::Egress => "web".to_owned(),
        Effect::AttachUserBrowser { .. } => "web:browser".to_owned(),
        Effect::Read | Effect::Write { .. } | Effect::Spawn | Effect::Govern | Effect::Spend => {
            return Ok(taint.clone());
        }
    };
    TaintSource::new(label)
        .map(|source| taint.union(&TaintSet::of(source)))
        .ok_or_else(|| {
            AxError::failure(
                AxCode::ConfigInvalid,
                "mark the taint of outside content",
                "the taint source label is empty",
            )
            .with_recovery("give the connector a non-empty server label")
        })
}

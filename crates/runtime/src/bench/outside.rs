// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one entrance outside content takes into a run (runtime-SPEC
//! 8-46): every answer a tool gave passes here before the model reads
//! it, and whatever an answer brought in from outside this city grows
//! the taint every later door of the run reads.

use kernel::{AxCode, AxError, Effect, TaintSet, TaintSource};

/// The taint a run carries once an answer produced under `effect` has
/// entered it.
///
/// Judged by the effect the tool declared at registration rather than
/// by anything the tool reports per call: the effect is already the one
/// statement of where a call reaches, and a second statement is one a
/// tool could forget to make.
///
/// # Errors
/// Refuses an answer whose source label is empty, so outside content
/// never enters under an empty set that every door would let through.
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

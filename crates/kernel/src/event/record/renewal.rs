// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one keep-warm renewal cost, so a person can read off the history
//! what keeping a prefix's cache alive spends.
//!
//! The request is absent for the reason `ModelCalled` carries no body: a
//! renewal resends a prefix whose bytes the run already recorded, and the
//! room address the line is written under says which one.

use serde::{Deserialize, Serialize};

use crate::budget::UsdMicros;
use crate::error::AxError;
use crate::model::ModelUsage;

/// `cache_renewed`: one renewal a worker sent, answered or refused.
///
/// A refusal is a line of its own rather than a note in the diagnostic
/// log, because a provider that refuses renewals is a fact about what
/// keep-warm costs, and the log is not the history.
///
/// `Refused` is tried first when reading: every field of `Answered` may
/// be absent, so it would read any line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum CacheRenewed {
    /// The provider refused the renewal; its refusal, unchanged.
    Refused {
        /// The failure the provider returned.
        refused: AxError,
    },
    /// The provider answered the renewal.
    Answered {
        /// Four token counts, as the provider reported them. Absent when
        /// it reported none, which is not the same fact as zero.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(
            feature = "schema",
            schemars(with = "Option<std::collections::BTreeMap<String, u64>>")
        )]
        usage: Option<ModelUsage>,
        /// What the provider itself billed, when it returns a figure.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        billed_usd_micros: Option<UsdMicros>,
    },
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

    #[test]
    fn a_refusal_reads_back_as_a_refusal_and_an_empty_answer_as_an_answer() {
        let refused = CacheRenewed::Refused {
            refused: AxError::failure(
                crate::error::AxCode::Provider,
                "renew a cached prefix",
                "rate limited",
            )
            .with_recovery("renewal is skipped until the next real request"),
        };
        let answered = CacheRenewed::Answered {
            usage: None,
            billed_usd_micros: None,
        };
        for line in [refused, answered] {
            assert_eq!(
                Payload::of(&line).unwrap().read::<CacheRenewed>().unwrap(),
                line
            );
        }
    }
}

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

use crate::budget::{Tokens, UsdMicros};
use crate::error::AxError;
use crate::model::{CacheCount, ModelUsage};

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

/// What one renewal shows about the cache it was sent to keep.
///
/// The one reading of a `cache_renewed` line that a cost view counts by
/// (`crates/kernel/spec/KeepWarm.lean` §8-74, storage D35).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenewalOutcome {
    /// The provider reported no cache write: the renewal reached the
    /// entry while it was alive.
    Read,
    /// The provider reported cache write tokens above zero: the entry
    /// had expired, and the renewal paid the write price for the prefix
    /// instead of the read price.
    Late,
    /// The provider answered without a cache write count.
    Unreported,
    /// The provider refused the renewal.
    Refused,
}

impl CacheRenewed {
    /// What this renewal shows about the cache it was sent to keep.
    pub fn outcome(&self) -> RenewalOutcome {
        match self {
            CacheRenewed::Refused { .. } => RenewalOutcome::Refused,
            CacheRenewed::Answered { usage, .. } => {
                match usage.map(|usage| usage.cache_write_tokens) {
                    None | Some(CacheCount::Unreported) => RenewalOutcome::Unreported,
                    Some(CacheCount::Reported(written)) if written == Tokens::new(0) => {
                        RenewalOutcome::Read
                    }
                    Some(CacheCount::Reported(_)) => RenewalOutcome::Late,
                }
            }
        }
    }
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

    #[test]
    fn a_renewal_that_wrote_the_cache_reads_as_late() {
        let answered = |written: CacheCount| CacheRenewed::Answered {
            usage: Some(ModelUsage {
                cache_write_tokens: written,
                ..ModelUsage::default()
            }),
            billed_usd_micros: None,
        };
        let lines = [
            answered(CacheCount::Reported(Tokens::new(0))),
            answered(CacheCount::Reported(Tokens::new(4096))),
            answered(CacheCount::Unreported),
            CacheRenewed::Answered {
                usage: None,
                billed_usd_micros: None,
            },
            CacheRenewed::Refused {
                refused: AxError::failure(
                    crate::error::AxCode::Provider,
                    "renew a cached prefix",
                    "rate limited",
                )
                .with_recovery("renewal is skipped until the next real request"),
            },
        ];
        assert_eq!(
            lines.iter().map(CacheRenewed::outcome).collect::<Vec<_>>(),
            vec![
                RenewalOutcome::Read,
                RenewalOutcome::Late,
                RenewalOutcome::Unreported,
                RenewalOutcome::Unreported,
                RenewalOutcome::Refused,
            ]
        );
    }
}

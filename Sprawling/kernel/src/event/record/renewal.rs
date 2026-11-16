// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::budget::{Tokens, UsdMicros};
use crate::error::AxError;
use crate::model::{CacheCount, ModelUsage};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum CacheRenewed {

    Refused {

        refused: AxError,
    },

    Answered {

        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(
            feature = "schema",
            schemars(with = "Option<std::collections::BTreeMap<String, u64>>")
        )]
        usage: Option<ModelUsage>,

        #[serde(default, skip_serializing_if = "Option::is_none")]
        billed_usd_micros: Option<UsdMicros>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenewalOutcome {

    Read,

    Late,

    Unreported,

    Refused,
}

impl CacheRenewed {

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

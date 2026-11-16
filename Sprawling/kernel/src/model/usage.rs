// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::budget::Tokens;
use crate::model::wire::DialectKind;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CacheCount {
    Reported(Tokens),
    #[default]
    Unreported,
}

impl CacheCount {

    #[must_use]
    pub const fn reported(self) -> Option<Tokens> {
        match self {
            CacheCount::Reported(tokens) => Some(tokens),
            CacheCount::Unreported => None,
        }
    }

    #[must_use]
    pub const fn or_zero(self) -> Tokens {
        match self {
            CacheCount::Reported(tokens) => tokens,
            CacheCount::Unreported => Tokens::new(0),
        }
    }
}

impl From<Option<Tokens>> for CacheCount {
    fn from(count: Option<Tokens>) -> CacheCount {
        match count {
            Some(tokens) => CacheCount::Reported(tokens),
            None => CacheCount::Unreported,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "UsageRow", try_from = "UsageRow")]
pub struct ModelUsage {
    pub input_tokens: Tokens,
    pub output_tokens: Tokens,
    pub cache_read_tokens: CacheCount,
    pub cache_write_tokens: CacheCount,
    pub dialect: Option<DialectKind>,
}

const USAGE_MEANING: u8 = 1;

#[derive(Serialize, Deserialize)]
struct UsageRow {
    input_tokens: Tokens,
    output_tokens: Tokens,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cache_read_tokens: Option<Tokens>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cache_write_tokens: Option<Tokens>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    v: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dialect: Option<DialectKind>,
}

impl From<ModelUsage> for UsageRow {
    fn from(usage: ModelUsage) -> UsageRow {
        UsageRow {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_read_tokens: usage.cache_read_tokens.reported(),
            cache_write_tokens: usage.cache_write_tokens.reported(),
            v: Some(USAGE_MEANING),
            dialect: usage.dialect,
        }
    }
}

impl TryFrom<UsageRow> for ModelUsage {
    type Error = String;

    fn try_from(row: UsageRow) -> Result<ModelUsage, String> {
        let input_tokens = match row.v {
            Some(USAGE_MEANING) => row.input_tokens,
            Some(other) => {
                return Err(format!(
                    "usage row version {other} is newer than this build reads \
                     (it reads {USAGE_MEANING}); read the ledger with a newer sprawling"
                ));
            }
            None => {
                let cached = CacheCount::from(row.cache_read_tokens)
                    .or_zero()
                    .checked_add(CacheCount::from(row.cache_write_tokens).or_zero())
                    .ok_or("usage row cache counts overflow a token count")?;
                if cached > row.input_tokens {
                    row.input_tokens
                        .checked_add(cached)
                        .ok_or("usage row counts overflow a token count")?
                } else {
                    row.input_tokens
                }
            }
        };
        Ok(ModelUsage {
            input_tokens,
            output_tokens: row.output_tokens,
            cache_read_tokens: CacheCount::from(row.cache_read_tokens),
            cache_write_tokens: CacheCount::from(row.cache_write_tokens),
            dialect: row.dialect,
        })
    }
}

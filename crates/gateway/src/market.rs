// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The model catalog snapshot: pinned entries with
//! integer prices. Value semantics on purpose — holding the previous
//! snapshot *is* the rollback; persistence is a projection concern, not
//! this module's.

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, Ceiling, UsdMicros};
use serde::{Deserialize, Serialize};

/// What a model accepts as input.
///
/// A closed judgement rather than a set of flags: every row answers it,
/// and the answer decides whether a picture may be sent at all. The
/// default is the narrow one, because guessing narrow costs a refusal a
/// person can act on and guessing wide costs a 400 from the provider.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKinds {
    #[default]
    Text,
    TextImage,
}

/// One catalog row. Prices are USD micros per one million tokens —
/// integers end to end (decision paths ban floats).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelEntry {
    pub id: String,
    pub context_tokens: u64,
    /// What this model may be sent. A picture reaching a `Text` row is
    /// refused at the endpoint rather than dropped on the wire.
    pub input: InputKinds,
    /// The most this model may emit in one response, thinking included.
    /// A property of the model rather than a caller's preference, and
    /// one no provider's model list answers, so it is registered with
    /// the model or it is not known at all. `None` says nobody has
    /// registered it: the OpenAI wire then omits the field and the
    /// Anthropic wire refuses, which is the whole difference between a
    /// call that answers and a call that returns nothing.
    pub max_output_tokens: Option<Ceiling>,
    pub input_price: UsdMicros,
    pub output_price: UsdMicros,
    pub cache_read_price: UsdMicros,
    pub cache_write_price: UsdMicros,
}

/// A versioned snapshot; lookups are by provider model id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketSnapshot {
    version: u32,
    entries: BTreeMap<String, ModelEntry>,
}

impl MarketSnapshot {
    /// The built-in pinned catalog (data plane; rows the city actually
    /// uses, prices rechecked at stage openings). Micros per Mtok:
    /// $3.00 = 3_000_000.
    pub fn builtin() -> MarketSnapshot {
        let rows = vec![
            ModelEntry {
                id: "claude-sonnet".to_owned(),
                context_tokens: 200_000,
                input: InputKinds::TextImage,
                max_output_tokens: Ceiling::new(64_000),
                input_price: UsdMicros::new(3_000_000),
                output_price: UsdMicros::new(15_000_000),
                cache_read_price: UsdMicros::new(300_000),
                cache_write_price: UsdMicros::new(3_750_000),
            },
            ModelEntry {
                id: "local".to_owned(),
                context_tokens: 32_768,
                // Local inference here is text-only until a row says
                // otherwise: what a local server can see is that
                // server's fact, and this table does not guess it.
                input: InputKinds::Text,
                max_output_tokens: Ceiling::new(4_096),
                input_price: UsdMicros::new(0),
                output_price: UsdMicros::new(0),
                cache_read_price: UsdMicros::new(0),
                cache_write_price: UsdMicros::new(0),
            },
        ];
        match MarketSnapshot::from_entries(1, rows) {
            Ok(snapshot) => snapshot,
            // The built-in table is duplicate-free by construction; a
            // regression here is a programming error surfaced in tests.
            Err(_) => MarketSnapshot {
                version: 1,
                entries: BTreeMap::new(),
            },
        }
    }

    /// Duplicate ids are refused: two prices for one model is two
    /// authorities for one rule.
    pub fn from_entries(version: u32, rows: Vec<ModelEntry>) -> Result<MarketSnapshot, AxError> {
        let mut entries = BTreeMap::new();
        for row in rows {
            let id = row.id.clone();
            if entries.insert(id.clone(), row).is_some() {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "build market snapshot",
                    format!("duplicate model id `{id}`"),
                ));
            }
        }
        Ok(MarketSnapshot { version, entries })
    }

    pub fn lookup(&self, id: &str) -> Option<&ModelEntry> {
        self.entries.get(id)
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rows_resolve_and_duplicates_are_refused() {
        let market = MarketSnapshot::builtin();
        assert!(market.lookup("claude-sonnet").is_some());
        assert!(market.lookup("nonexistent").is_none());
        assert!(!market.is_empty());
        let dup = vec![
            market.lookup("local").unwrap().clone(),
            market.lookup("local").unwrap().clone(),
        ];
        assert!(MarketSnapshot::from_entries(2, dup).is_err());
    }

    #[test]
    fn the_catalogue_says_which_rows_can_be_shown_a_picture() {
        let market = MarketSnapshot::builtin();
        assert_eq!(
            market.lookup("claude-sonnet").unwrap().input,
            InputKinds::TextImage
        );
        assert_eq!(market.lookup("local").unwrap().input, InputKinds::Text);
        assert_eq!(InputKinds::default(), InputKinds::Text);
        assert_eq!(
            serde_json::to_string(&InputKinds::TextImage).unwrap(),
            "\"text_image\""
        );
    }

    #[test]
    fn holding_the_previous_snapshot_is_the_rollback() {
        let old = MarketSnapshot::builtin();
        let mut rows: Vec<ModelEntry> = old.entries.values().cloned().collect();
        rows[0].input_price = UsdMicros::new(9_999_999);
        let newer = MarketSnapshot::from_entries(2, rows).unwrap();
        assert_ne!(old, newer);
        let rolled_back = old.clone();
        assert_eq!(
            rolled_back, old,
            "value semantics: keep it, you can return to it"
        );
    }
}

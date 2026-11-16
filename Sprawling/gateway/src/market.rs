// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use kernel::{AxCode, AxError, Ceiling, UsdMicros};

use crate::provider::ceiling::FigureSource;

pub use kernel::event::record::InputKinds;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ModelEntry {
    pub id: String,

    pub context_tokens: u64,
    #[serde(default)]
    pub window_from: Option<FigureSource>,

    pub input: InputKinds,

    pub max_output_tokens: Option<Ceiling>,
    #[serde(default)]
    pub ceiling_from: Option<FigureSource>,
    pub input_price: UsdMicros,
    pub output_price: UsdMicros,
    pub cache_read_price: UsdMicros,
    pub cache_write_price: UsdMicros,
}

impl ModelEntry {

    pub fn states_a_price(&self) -> bool {
        [
            self.input_price,
            self.output_price,
            self.cache_read_price,
            self.cache_write_price,
        ]
        .iter()
        .any(|price| price.get() > 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketSnapshot {
    version: u32,
    entries: BTreeMap<String, ModelEntry>,
}

impl MarketSnapshot {

    pub fn builtin() -> Result<MarketSnapshot, AxError> {
        let rows = vec![
            ModelEntry {
                id: "claude-sonnet".to_owned(),
                context_tokens: 200_000,
                window_from: Some(FigureSource::Preset),
                input: InputKinds::TextImage,
                max_output_tokens: Ceiling::new(64_000),
                ceiling_from: Some(FigureSource::Preset),
                input_price: UsdMicros::new(3_000_000),
                output_price: UsdMicros::new(15_000_000),
                cache_read_price: UsdMicros::new(300_000),
                cache_write_price: UsdMicros::new(3_750_000),
            },
            ModelEntry {
                id: "local".to_owned(),
                context_tokens: 32_768,
                window_from: Some(FigureSource::Preset),

                input: InputKinds::Text,
                max_output_tokens: Ceiling::new(4_096),
                ceiling_from: Some(FigureSource::Preset),
                input_price: UsdMicros::new(0),
                output_price: UsdMicros::new(0),
                cache_read_price: UsdMicros::new(0),
                cache_write_price: UsdMicros::new(0),
            },
        ];
        MarketSnapshot::from_entries(1, rows)
    }

    pub fn from_entries(version: u32, rows: Vec<ModelEntry>) -> Result<MarketSnapshot, AxError> {
        let mut entries = BTreeMap::new();
        for row in rows {
            let id = row.id.clone();
            if entries.insert(id.clone(), row).is_some() {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "build market snapshot",
                    format!("duplicate model id `{id}`"),
                )
                .with_recovery(format!(
                    "delete one of the two rows carrying the id `{id}`: a model's facts \
                     have one row, and two rows are two prices for one call"
                )));
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
        let market = MarketSnapshot::builtin().unwrap();
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
        let market = MarketSnapshot::builtin().unwrap();
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
        let old = MarketSnapshot::builtin().unwrap();
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

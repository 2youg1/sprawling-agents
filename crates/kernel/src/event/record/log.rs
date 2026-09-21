// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the ledger records about itself.

use serde::{Deserialize, Serialize};

/// `log_truncated`: how many bytes a crash left behind that the opening
/// procedure could not read as records.
///
/// The line is written by the city, because opening the ledger is the
/// city's own work rather than any run's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LogTruncated {
    pub dropped_bytes: u64,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

    /// The bytes `memory::jsonl::append_log_truncated` wrote by hand.
    #[test]
    fn a_truncation_line_writes_the_one_key_the_hand_written_map_wrote() {
        let mut hand = serde_json::Map::new();
        hand.insert("dropped_bytes".to_owned(), serde_json::Value::from(191u64));
        let hand = Payload::new(hand).unwrap();
        let typed = Payload::of(&LogTruncated { dropped_bytes: 191 }).unwrap();
        assert_eq!(
            serde_json::to_string(&typed).unwrap(),
            serde_json::to_string(&hand).unwrap()
        );
        assert_eq!(typed.read::<LogTruncated>().unwrap().dropped_bytes, 191);
    }
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Lines {
    Counted { added: u32, removed: u32 },
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum How {
    Added,
    Modified,
    Deleted,
    Renamed { from: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FileChange {
    pub path: String,
    pub how: How,
    pub lines: Lines,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_binary_file_is_not_a_file_that_moved_nothing() {
        let binary = serde_json::to_string(&Lines::Binary).unwrap();
        let nothing = serde_json::to_string(&Lines::Counted {
            added: 0,
            removed: 0,
        })
        .unwrap();
        assert_ne!(binary, nothing);
    }

    #[test]
    fn a_rename_carries_where_the_file_came_from() {
        let moved = FileChange {
            path: "lab/lex.rs".to_owned(),
            how: How::Renamed {
                from: "lab/lexer.rs".to_owned(),
            },
            lines: Lines::Counted {
                added: 0,
                removed: 0,
            },
        };
        let bytes = serde_json::to_vec(&moved).unwrap();
        let back: FileChange = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, moved);
    }
}

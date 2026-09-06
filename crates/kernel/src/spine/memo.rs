// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Memo outline: the six fields and the moments they are written.

use serde::{Deserialize, Serialize};

pub const MEMO_OUTLINE_FIELDS: [&str; 6] = [
    "Current goal",
    "Current stage",
    "Next action",
    "Blocked by",
    "Decision index",
    "Checkpoint index",
];

/// Deliberately exhaustive shape verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoShape {
    WellFormed,
    Malformed { missing: Vec<&'static str> },
}

/// A field is present when some line, stripped of leading markdown
/// furniture (#, -, *, spaces), starts with it. Case is not part of the
/// contract, for the reason given at `parse_status`.
#[must_use]
pub fn check_memo_shape(text: &str) -> MemoShape {
    let stripped: Vec<String> = text
        .lines()
        .map(|line| {
            line.trim_start_matches(['#', '-', '*', ' ', '\t'])
                .to_lowercase()
        })
        .collect();
    let missing: Vec<&'static str> = MEMO_OUTLINE_FIELDS
        .into_iter()
        .filter(|field| {
            let needle = field.to_lowercase();
            !stripped.iter().any(|line| line.starts_with(&needle))
        })
        .collect();
    if missing.is_empty() {
        MemoShape::WellFormed
    } else {
        MemoShape::Malformed { missing }
    }
}

/// Scope-change vocabulary: requirements move by KEEP/ADD/DROP, never by
/// piling replacements into a bigger project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeChange {
    Keep,
    Add,
    Drop,
}

/// The three moments the plan may be written.
///
/// Its consumer is the projection that holds the parsed tree: because
/// the set of moments is closed, a reader can hold the tree between them
/// instead of parsing every building's file for every question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteMoment {
    BeforeReport,
    AfterFeedback,
    OnPlanChange,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;
    #[test]
    fn memo_outline_names_what_is_missing() {
        let memo = "\
## Current goal
ship S2
## current stage
S2.08
## Next action
approval
## Blocked by
none
## Decision index
d-1
";
        let MemoShape::Malformed { missing } = check_memo_shape(memo) else {
            panic!("the checkpoint index is absent, shape must be malformed");
        };
        assert_eq!(missing, ["Checkpoint index"]);
        let full = format!("{memo}## Checkpoint index\nc-1\n");
        assert_eq!(check_memo_shape(&full), MemoShape::WellFormed);
    }
}

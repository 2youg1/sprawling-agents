// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The `[context]` table: where a layer moves the second rung of the
//! context reminder.
//!
//! The value's shape and its legal domain are `kernel`'s to answer
//! (`kernel::config::SecondThreshold`). What this module owns is how one
//! layer's `CONFIG.toml` spells that value, and the rule that an
//! out-of-domain percent is refused where the file is parsed rather than
//! read into a value nobody can diagnose.

use serde::Deserialize;

/// What one layer's `[context]` table states, read as written rather
/// than as a parsed type: the domain is `SecondThreshold`'s answer, and
/// a deserializer that enforced it here would be the second place that
/// rule lives (the `McpSection` rule, city-SPEC 8-4).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ContextSection {
    /// A whole percent of the window. Checked where it becomes a
    /// `SecondThreshold`, which is the one construction point.
    pub(super) second_threshold: u64,
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
    use crate::config_layers::ConfigLayer;

    /// The refusal is the only sentence a person editing this file
    /// gets, so it carries the legal domain (kernel-SPEC 8-22). No
    /// clamping: a file that states 25 means something its writer has
    /// to be told is not accepted.
    #[test]
    fn a_second_threshold_outside_30_through_90_is_refused_with_the_domain() {
        for refused in [29, 91] {
            let text = format!("[context]\nsecond_threshold = {refused}\n");
            let err = ConfigLayer::parse(&text)
                .expect_err("a percent outside the legal domain is refused");
            let recovery = err.recovery();
            assert!(
                recovery.contains("30") && recovery.contains("90"),
                "{refused}: {recovery}"
            );
        }
    }

    #[test]
    fn the_domain_ends_of_the_second_threshold_are_taken() {
        for taken in [30, 90] {
            let text = format!("[context]\nsecond_threshold = {taken}\n");
            assert!(
                ConfigLayer::parse(&text).is_ok(),
                "{taken} is in the domain"
            );
        }
    }
}

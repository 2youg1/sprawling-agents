// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ContextSection {
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

    #[test]
    fn a_second_threshold_outside_31_through_90_is_refused_with_the_domain() {
        for refused in [30, 91] {
            let text = format!("[context]\nsecond_threshold = {refused}\n");
            let err = ConfigLayer::parse(&text)
                .expect_err("a percent outside the legal domain is refused");
            let recovery = err.recovery();
            assert!(
                recovery.contains("31") && recovery.contains("90"),
                "{refused}: {recovery}"
            );
        }
    }

    #[test]
    fn the_domain_ends_of_the_second_threshold_are_taken() {
        for taken in [31, 90] {
            let text = format!("[context]\nsecond_threshold = {taken}\n");
            assert!(
                ConfigLayer::parse(&text).is_ok(),
                "{taken} is in the domain"
            );
        }
    }
}

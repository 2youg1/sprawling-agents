// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

pub(crate) fn outside_reserved(relative: &Path) -> bool {
    !relative.components().any(|component| {
        component.as_os_str().to_str().is_some_and(|segment| {
            kernel::PROTECTED_METADATA
                .iter()
                .any(|name| segment.eq_ignore_ascii_case(name))
        })
    })
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
    fn the_city_s_own_bookkeeping_is_never_somebody_s_work() {
        assert!(outside_reserved(Path::new("City.md")));
        assert!(outside_reserved(Path::new("lab/Roadmap.md")));
        assert!(!outside_reserved(Path::new(".sprawling/ledger/a.jsonl")));
        assert!(!outside_reserved(Path::new("lab/.sprawling/CONFIG.toml")));

        assert!(!outside_reserved(Path::new(".git/hooks/pre-run")));
        assert!(!outside_reserved(Path::new("lab/.git/config")));

        assert!(!outside_reserved(Path::new(".SPRAWLING/cas")));
    }
}

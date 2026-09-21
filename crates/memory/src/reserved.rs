// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which bytes belong to the people who work in the city, and which
//! belong to the city's own bookkeeping.
//!
//! One predicate, because two modules ask the same question for
//! different reasons and an answer that differed between them would be
//! a rule with two homes: the checkpoint stages what a person wrote,
//! and the worktree ceiling weighs what a person wrote. The ledger, the
//! object store, the projections and the other nodes' trees all live
//! under [`kernel::RESERVED_PREFIX`] and count as neither.

use std::path::Path;

/// True when `relative` names nothing under the reserved subtree.
///
/// `relative` is read as a path relative to the city root, and the
/// prefix is looked for in every segment rather than only the first: a
/// building's own rules sit at `<building>/.sprawling/`, and they are
/// the city's bookkeeping wherever they sit. Segment comparison is
/// ASCII-case-insensitive for the reason
/// [`kernel::Address::is_reserved`] gives — Windows resolves
/// `.SPRAWLING` to the same directory.
pub(crate) fn outside_reserved(relative: &Path) -> bool {
    !relative.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|segment| segment.eq_ignore_ascii_case(kernel::RESERVED_PREFIX))
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
        // Windows resolves the reserved directory case-insensitively,
        // so this predicate has to as well.
        assert!(!outside_reserved(Path::new(".SPRAWLING/cas")));
    }
}

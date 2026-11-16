// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The patch text of one file between two checkpoints.
//!
//! Its own module because it is the one reading that maps a whole shape
//! rather than handing a value on: `storage::of_file` answers in its own
//! vocabulary, and turning that into the wire's is a responsibility with
//! a name.

use std::path::Path;

/// The patch text of one file between two checkpoints.
///
/// An oid this city never wrote is `Unavailable`, for the reason
/// `Changes` gives: "it changed nothing" and "I cannot read it" are
/// different answers, and a reader's next move differs.
pub(super) fn hunks_answer(
    city_root: &Path,
    oid_a: kernel::GitOid,
    oid_b: kernel::GitOid,
    path: &str,
) -> wire::Answer {
    let Ok(patch) = storage::of_file(city_root, oid_a, storage::Head::Commit(oid_b), path) else {
        return super::prepared::unavailable(format!("Hunks({oid_a}..{oid_b} {path})"));
    };
    wire::Answer::Hunks(Box::new(wire::HunksAnswer {
        oid_a,
        oid_b,
        path: path.to_owned(),
        lines: patch
            .lines
            .into_iter()
            .map(|line| wire::PatchLine {
                number: line.number,
                text: line.text,
            })
            .collect(),
        withheld: patch
            .withheld
            .into_iter()
            .map(|held| wire::Withheld {
                number: held.number,
                reason: held.reason,
            })
            .collect(),
    }))
}

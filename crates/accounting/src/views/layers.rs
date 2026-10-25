// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The prompt layers of one address (`crates/accounting/Spec.lean` §8-38),
//! not answered yet.

use std::path::Path;

use kernel::{Address, AxCode, AxError, B3Hash};

use super::prepared::unavailable_because;

/// `Unavailable` until the layers are read.
pub(super) fn layers_answer(city_root: &Path, addr: Address) -> wire::Answer {
    let stopped = AxError::failure(
        AxCode::InvalidArgs,
        "list the prompt layers",
        format!("{}: {}", city_root.display(), addr.as_str()),
    )
    .with_recovery("not built");
    let _unread = B3Hash::digest(b"");
    unavailable_because("Layers".to_owned(), &stopped)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;

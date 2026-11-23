// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::report::XtaskError;

pub(crate) const CLIENT_MARK: &str = "assets/index-";

pub(crate) fn carries_client(binary: &Path) -> Result<bool, XtaskError> {
    carries(binary, CLIENT_MARK)
}

fn carries(binary: &Path, mark: &str) -> Result<bool, XtaskError> {
    let bytes = std::fs::read(binary).map_err(|source| XtaskError::Io {
        path: binary.display().to_string(),
        source,
    })?;
    Ok(contains(&bytes, mark.as_bytes()))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::{CLIENT_MARK, contains};

    #[test]
    fn a_binary_without_the_client_table_is_named_by_the_gate() {
        assert!(contains(
            b"...assets/index-x1.js...",
            CLIENT_MARK.as_bytes()
        ));
        assert!(!contains(b"a placeholder build", CLIENT_MARK.as_bytes()));
    }
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::report::XtaskError;

pub(crate) const CLIENT_MARK: &str = "assets/index-";

const ENGINE_MARK: &str = "all fuel consumed by WebAssembly";

pub(crate) fn carries_client(binary: &Path) -> Result<bool, XtaskError> {
    carries(binary, CLIENT_MARK)
}

pub(crate) fn carries_engine(binary: &Path) -> Result<bool, XtaskError> {
    carries(binary, ENGINE_MARK)
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
    use super::{CLIENT_MARK, ENGINE_MARK, contains};

    #[test]
    fn a_binary_without_the_client_table_is_named_by_the_gate() {
        assert!(contains(
            b"...assets/index-x1.js...",
            CLIENT_MARK.as_bytes()
        ));
        assert!(!contains(b"a placeholder build", CLIENT_MARK.as_bytes()));
    }

    #[test]
    fn a_binary_without_the_engine_does_not_carry_its_sentence() {
        let with = format!("...{ENGINE_MARK}...");
        assert!(contains(with.as_bytes(), ENGINE_MARK.as_bytes()));
        assert!(!contains(
            b"this build carries no execution engine",
            ENGINE_MARK.as_bytes()
        ));
    }
}

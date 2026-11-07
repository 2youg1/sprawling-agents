// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which promises a built binary keeps, read off the artifact itself.
//!
//! Two of this product's promises are about what is inside the one file
//! a person downloads: it serves the real client rather than a
//! placeholder page, and it can run a guest. Neither is a fact about a
//! manifest — a binary somebody already built carries no record of what
//! it was compiled with — so both are read as bytes, out of the file,
//! by the two commands that must refuse it: the `budget` gate weighs it
//! and `package` assembles it.
//!
//! A mark is a foreign spelling on purpose. What is wanted is a
//! sentence that a build with the thing writes and a build without it
//! cannot, and that sentence belongs to whoever produces the thing.

use std::path::Path;

use crate::report::XtaskError;

/// The request path that only a built bundle produces.
///
/// Vite writes every hashed chunk under `assets/`, and the placeholder
/// page has no such path in its table. One entry rather than a file
/// name: the file names carry a content hash and change on every build.
pub(crate) const CLIENT_MARK: &str = "assets/index-";

/// The trap sentence only wasmtime writes, so a binary holding it is a
/// binary the execution engine was linked into.
///
/// Fuel metering is what this project configures the engine for, so its
/// message is the part of wasmtime that cannot be absent from an engine
/// build; and on the default feature set this tree pulls in no wasm
/// crate at all, so nothing else here can write this sentence.
const ENGINE_MARK: &str = "all fuel consumed by WebAssembly";

/// Whether a built binary carries the real client or only the page
/// shell.
///
/// # Errors
/// Propagates the failure of reading the binary: a release artifact
/// that cannot be read is a finding, not a thing to skip over.
pub(crate) fn carries_client(binary: &Path) -> Result<bool, XtaskError> {
    carries(binary, CLIENT_MARK)
}

/// Whether a built binary carries the execution engine the Python arm
/// runs its guests in.
///
/// The archive is refused without it (`package`), because a release
/// that lacks the engine answers every `python` call with a refusal
/// naming a feature the person who downloaded it cannot turn on.
///
/// # Errors
/// Propagates the failure of reading the binary, for the reason
/// [`carries_client`] gives.
pub(crate) fn carries_engine(binary: &Path) -> Result<bool, XtaskError> {
    carries(binary, ENGINE_MARK)
}

/// Whether a built artifact holds a mark, read as bytes.
fn carries(binary: &Path, mark: &str) -> Result<bool, XtaskError> {
    let bytes = std::fs::read(binary).map_err(|source| XtaskError::Io {
        path: binary.display().to_string(),
        source,
    })?;
    Ok(contains(&bytes, mark.as_bytes()))
}

/// Naive subsequence search; the haystack is read once per gate run.
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
        // A short stand-in rather than a real chunk name: a content hash
        // is exactly the shape `xtask secret` is built to notice.
        assert!(contains(
            b"...assets/index-x1.js...",
            CLIENT_MARK.as_bytes()
        ));
        assert!(!contains(b"a placeholder build", CLIENT_MARK.as_bytes()));
    }

    /// The engine's mark is read the same way, and a binary built
    /// without it does not carry the sentence by accident.
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

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a building's desktop allowlist lives, and the one door that
//! replaces it.
//!
//! **Nothing here parses it**, which is why it is its own module rather
//! than more of `crate::policy`: that module reads a building's rules
//! and this one only decides where a document it never opens is kept.
//! The authority on this file's syntax is the server that reads it at
//! startup (`desktop/src/scope.rs`), and that server fails closed — a
//! file it cannot read permits nothing. A second parser on this side
//! would be a second authority, and one of two authorities eventually
//! reads a file as meaning something the other does not.

use std::path::{Path, PathBuf};

use kernel::{Address, AxError};

use super::governed;

/// The allowlist the desktop connector is started under, window by
/// window.
pub const DESKTOP_SCOPE_FILE: &str = "DESKTOP.toml";

/// Where a building's desktop allowlist lives: beside its rules, in the
/// building's own reserved subtree.
///
/// It is a governing document rather than a product. It states, window
/// by window, what this building's runs may touch on somebody's own
/// machine — so by the reading `DomainReach` rests on, the file that
/// decides what residents may do is never a file residents write.
/// `is_reserved` is true for any address with `.sprawling` in it, so no
/// write domain reaches this path, including `Everything`'s.
#[must_use]
pub fn desktop_scope_path(city_root: &Path, addr: &Address) -> PathBuf {
    governed(city_root, addr).join(DESKTOP_SCOPE_FILE)
}

/// Replaces a building's desktop allowlist with what a person wrote.
///
/// Whole rather than patched, for the reason `city::governed` gives: a
/// person edits this in one box and saves it once, and a partial write
/// would leave the connector scoped by half a line.
///
/// # Errors
/// Propagates a reserved subtree that cannot be created or written.
pub fn write_desktop_scope(
    city_root: &Path,
    addr: &Address,
    text: &str,
) -> Result<PathBuf, AxError> {
    let path = desktop_scope_path(city_root, addr);
    crate::document::replace(&path, text.as_bytes())?;
    Ok(path)
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

    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }

    /// The desktop allowlist is a governing document, not a product. It
    /// says, window by window, what this building's runs may touch on
    /// somebody's machine — so it lands where no write domain reaches,
    /// and a resident cannot widen its own scope.
    #[test]
    fn the_desktop_allowlist_lands_where_no_write_domain_reaches() {
        let dir = tempfile::tempdir().unwrap();
        let lab = addr("lab");
        let written = write_desktop_scope(dir.path(), &lab, "windows = [\"*Notepad*\"]\n").unwrap();
        assert_eq!(
            std::fs::read_to_string(&written).unwrap(),
            "windows = [\"*Notepad*\"]\n"
        );
        assert_eq!(written, desktop_scope_path(dir.path(), &lab));

        let relative = written.strip_prefix(dir.path()).unwrap();
        let slashed = relative
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        let reached = Address::parse(&slashed).unwrap();
        assert!(
            reached.is_reserved(),
            "{} is somewhere a run could write",
            relative.display()
        );
        // It sits beside the rules it pairs with, in the same subtree.
        assert_eq!(
            written.parent(),
            super::super::rules_path(dir.path(), &lab).parent()
        );
    }

    /// A second save replaces the first: one box, one document. And the
    /// bytes are the person's own — this side parses nothing, because
    /// the server that reads it is the authority on its syntax and
    /// fails closed on a file it cannot read.
    #[test]
    fn the_allowlist_is_written_whole_and_not_parsed_here() {
        let dir = tempfile::tempdir().unwrap();
        let lab = addr("lab");
        write_desktop_scope(dir.path(), &lab, "windows = [\"a\"]\n").unwrap();
        let second = write_desktop_scope(dir.path(), &lab, "windows = [").unwrap();
        assert_eq!(std::fs::read_to_string(&second).unwrap(), "windows = [");
    }
}

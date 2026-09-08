// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three documents that govern a city, and the one point they are
//! written through.
//!
//! All three sit in the city's own reserved subtree, which no write
//! domain reaches: the Mayor cannot edit who the Mayor is, the clerk
//! cannot edit what it answers by, and neither can edit how the person
//! wants their city run. That is the whole reason this module exists as
//! one door rather than as three callers each joining a path — a caller
//! that composed its own path could compose one that leaves the subtree,
//! and then "a resident cannot edit its own governance" would hold by
//! habit rather than by construction.

use std::path::{Path, PathBuf};

use kernel::AxError;

use crate::spine_files::hall::{CLERK_FILE, MAYOR_FILE};

/// How this person wants their city run.
///
/// It belongs to no resident, which is why it sits beside the two
/// identity files rather than at somebody's address.
pub const PREFERENCES_FILE: &str = "PREFERENCES.md";

/// Which governed document. A closed set rather than a file name,
/// because where these live is the city's answer and not the caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Governed {
    Mayor,
    Clerk,
    Preferences,
}

impl Governed {
    /// The file name this document is kept under.
    #[must_use]
    pub fn file(self) -> &'static str {
        match self {
            Governed::Mayor => MAYOR_FILE,
            Governed::Clerk => CLERK_FILE,
            Governed::Preferences => PREFERENCES_FILE,
        }
    }

    /// Where this document lives in a city rooted at `city_root`.
    #[must_use]
    pub fn path(self, city_root: &Path) -> PathBuf {
        city_root.join(kernel::RESERVED_PREFIX).join(self.file())
    }
}

/// Writes one governed document whole, creating the reserved subtree if
/// this city has not laid it out yet.
///
/// Whole rather than patched: these are documents a person edits in one
/// box and saves once, and a partial write would leave the city governed
/// by half a sentence. The previous content is not kept here — the
/// Ledger line that announces the write is what a reader goes back to.
///
/// # Errors
/// Propagates a reserved subtree that cannot be created or written.
pub fn write_governed(city_root: &Path, which: Governed, body: &str) -> Result<PathBuf, AxError> {
    let path = which.path(city_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| crate::spine_files::storage(parent, &err))?;
    }
    std::fs::write(&path, body.as_bytes())
        .map_err(|err| crate::spine_files::storage(&path, &err))?;
    Ok(path)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// Every one of the three lands inside the reserved subtree, which
    /// is what keeps a resident from editing what governs it.
    #[test]
    fn all_three_documents_land_where_no_write_domain_reaches() {
        let dir = tempfile::tempdir().unwrap();
        for which in [Governed::Mayor, Governed::Clerk, Governed::Preferences] {
            let path = write_governed(dir.path(), which, "# written by a person\n").unwrap();
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                "# written by a person\n"
            );
            let relative = path.strip_prefix(dir.path()).unwrap();
            assert!(
                relative.starts_with(kernel::RESERVED_PREFIX),
                "{} is outside the reserved subtree",
                relative.display()
            );
        }
    }

    /// A second save replaces the first: one box, one document.
    #[test]
    fn a_second_save_replaces_the_first() {
        let dir = tempfile::tempdir().unwrap();
        write_governed(dir.path(), Governed::Preferences, "first\n").unwrap();
        let path = write_governed(dir.path(), Governed::Preferences, "second\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second\n");
    }
}

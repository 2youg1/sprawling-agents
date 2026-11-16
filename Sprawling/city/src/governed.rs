// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::AxError;

use crate::spine_files::hall::{CLERK_FILE, MAYOR_FILE};

pub const PREFERENCES_FILE: &str = "PREFERENCES.md";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Governed {
    Mayor,
    Clerk,
    Preferences,
}

impl Governed {
    #[must_use]
    pub fn file(self) -> &'static str {
        match self {
            Governed::Mayor => MAYOR_FILE,
            Governed::Clerk => CLERK_FILE,
            Governed::Preferences => PREFERENCES_FILE,
        }
    }

    #[must_use]
    pub fn path(self, city_root: &Path) -> PathBuf {
        city_root.join(kernel::RESERVED_PREFIX).join(self.file())
    }
}

pub fn write_governed(
    city_root: &Path,
    which: Governed,
    base: &str,
    body: &str,
) -> Result<PathBuf, AxError> {
    crate::identity::readable(which, body)?;
    let path = which.path(city_root);
    crate::document::edit_against(&path, base.as_bytes(), body.as_bytes())?;
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

    #[test]
    fn all_three_documents_land_where_no_write_domain_reaches() {
        let dir = tempfile::tempdir().unwrap();
        for which in [Governed::Mayor, Governed::Clerk, Governed::Preferences] {
            let path = write_governed(dir.path(), which, "", "# written by a person\n").unwrap();
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

    #[test]
    fn a_second_save_replaces_the_first() {
        let dir = tempfile::tempdir().unwrap();
        write_governed(
            dir.path(),
            Governed::Preferences,
            "",
            "first
",
        )
        .unwrap();
        let path = write_governed(
            dir.path(),
            Governed::Preferences,
            "first
",
            "second
",
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "second
"
        );
    }
}

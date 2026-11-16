// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::{Address, AxError};

use super::governed;

pub const DESKTOP_SCOPE_FILE: &str = "DESKTOP.toml";

#[must_use]
pub fn desktop_scope_path(city_root: &Path, addr: &Address) -> PathBuf {
    governed(city_root, addr).join(DESKTOP_SCOPE_FILE)
}

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

        assert_eq!(
            written.parent(),
            super::super::rules_path(dir.path(), &lab).parent()
        );
    }

    #[test]
    fn the_allowlist_is_written_whole_and_not_parsed_here() {
        let dir = tempfile::tempdir().unwrap();
        let lab = addr("lab");
        write_desktop_scope(dir.path(), &lab, "windows = [\"a\"]\n").unwrap();
        let second = write_desktop_scope(dir.path(), &lab, "windows = [").unwrap();
        assert_eq!(std::fs::read_to_string(&second).unwrap(), "windows = [");
    }
}

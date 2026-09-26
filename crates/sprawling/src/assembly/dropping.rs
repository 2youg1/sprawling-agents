// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A file a person dropped onto the composer, kept in the city so the
//! words they send can name it (sprawling-SPEC.md 8-119).
//!
//! It lands at `<city>/dropped/<hash>/<name>`: under the city root and
//! outside every building, so every resident's `read` reaches it by its
//! city-relative address and no building's repository gains a file
//! nobody committed; in a directory named by the bytes, so the same
//! file dropped twice is one file and two files of one name are two.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::{Address, AxCode, AxError, B3Hash};

/// The directory under the city root that holds what was dropped.
pub(super) const DROPPED_DIR: &str = "dropped";

/// How many hex characters of the content hash name a file's directory:
/// 64 bits, far past any number of files one person drops.
const HASH_CHARS: usize = 16;

/// The sink the served page's `/drop` route hands its files to.
pub(super) fn dropping(city_root: PathBuf) -> channels::DropSink {
    Arc::new(move |name: &str, bytes: &[u8]| {
        keep_dropped(&city_root, name, bytes).map(|kept| kept.display().to_string())
    })
}

/// Keeps one dropped file and answers the absolute path it is at.
///
/// # Errors
/// `E_INVALID_ARGS` when the name cannot be the last segment of a city
/// address, and `E_STORAGE_FATAL` naming the path when the disk will
/// not take the bytes.
pub(super) fn keep_dropped(city_root: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, AxError> {
    let _ = (city_root, name, bytes, HASH_CHARS);
    let _: Option<(Address, B3Hash)> = None;
    Err(AxError::failure(AxCode::StorageFatal, "keep a dropped file", "not yet")
        .with_recovery("not yet"))
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

    /// The path answered is absolute, lies under the city's `dropped/`,
    /// and holds the bytes that were dropped; the same bytes again
    /// answer the same path.
    #[test]
    fn a_dropped_file_is_kept_under_the_city_and_named_by_its_bytes() {
        let city = tempfile::tempdir().unwrap();
        let kept = keep_dropped(city.path(), "笔记 one.txt", b"four").unwrap();
        let again = keep_dropped(city.path(), "笔记 one.txt", b"four").unwrap();
        let root = std::path::absolute(city.path()).unwrap();
        assert_eq!(
            (
                kept.is_absolute(),
                kept.strip_prefix(root.join(DROPPED_DIR)).is_ok(),
                std::fs::read(&kept).unwrap(),
                kept.file_name().and_then(|name| name.to_str()),
                &again,
            ),
            (true, true, b"four".to_vec(), Some("笔记 one.txt"), &kept)
        );
    }

    /// A name that is not one address segment is refused before anything
    /// is written.
    #[test]
    fn a_name_that_is_not_one_segment_is_refused_and_nothing_is_written() {
        let city = tempfile::tempdir().unwrap();
        for name in ["..", "a/b", "a\\b", ""] {
            let refused = keep_dropped(city.path(), name, b"four").unwrap_err();
            assert_eq!(*refused.code(), AxCode::InvalidArgs, "{name:?}");
        }
        assert!(!city.path().join(DROPPED_DIR).exists());
    }
}

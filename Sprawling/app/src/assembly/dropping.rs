// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kernel::consts_policy::HALL_BUILDING;
use kernel::{Address, AxCode, AxError, B3Hash};

pub(super) const DROPPED_DIR: &str = "dropped";

const HASH_CHARS: usize = 16;

pub(super) fn dropping(city_root: PathBuf) -> wire::DropSink {
    Arc::new(move |name: &str, bytes: &[u8]| {
        keep_dropped(&city_root, name, bytes).map(|kept| kept.display().to_string())
    })
}

pub(super) fn keep_dropped(city_root: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, AxError> {
    let hash = B3Hash::digest(bytes).to_string();
    let folder = hash.get(..HASH_CHARS).unwrap_or(&hash);
    let addr = format!("{HALL_BUILDING}/{DROPPED_DIR}/{folder}/{name}");
    let one_segment = !name.is_empty() && !name.contains(['/', '\\']);
    if !one_segment || Address::parse(&addr).is_err() {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "keep a dropped file",
            format!("{name:?} is not a name a file in this city can have"),
        )
        .with_recovery("rename the file, then drop it again"));
    }
    let root = std::path::absolute(city_root).map_err(|err| stored(city_root, &err))?;
    let dir = root.join(HALL_BUILDING).join(DROPPED_DIR).join(folder);
    let path = dir.join(name);
    if std::fs::read(&path).is_ok_and(|held| held == bytes) {
        return Ok(path);
    }
    std::fs::create_dir_all(&dir).map_err(|err| stored(&dir, &err))?;
    std::fs::write(&path, bytes).map_err(|err| stored(&path, &err))?;
    Ok(path)
}

fn stored(path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "keep a dropped file",
        format!("{}: {err}", path.display()),
    )
    .with_recovery("free space on the city's disk or make that folder writable, then drop it again")
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
    fn a_dropped_file_is_kept_under_the_hall_and_named_by_its_bytes() {
        let city = tempfile::tempdir().unwrap();
        let kept = keep_dropped(city.path(), "笔记 one.txt", b"four").unwrap();
        let again = keep_dropped(city.path(), "笔记 one.txt", b"four").unwrap();
        let root = std::path::absolute(city.path()).unwrap();
        assert_eq!(
            (
                kept.is_absolute(),
                kept.strip_prefix(root.join(HALL_BUILDING).join(DROPPED_DIR))
                    .is_ok(),
                std::fs::read(&kept).unwrap(),
                kept.file_name().and_then(|name| name.to_str()),
                &again,
            ),
            (true, true, b"four".to_vec(), Some("笔记 one.txt"), &kept)
        );
    }

    #[test]
    fn a_name_that_is_not_one_segment_is_refused_and_nothing_is_written() {
        let city = tempfile::tempdir().unwrap();
        for name in ["..", "a/b", "a\\b", ""] {
            let refused = keep_dropped(city.path(), name, b"four").unwrap_err();
            assert_eq!(*refused.code(), AxCode::InvalidArgs, "{name:?}");
        }
        assert!(!city.path().join(HALL_BUILDING).exists());
    }
}

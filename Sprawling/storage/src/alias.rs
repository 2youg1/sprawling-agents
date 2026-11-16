// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use crate::error::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasKind {

    Link,

    HardLink,
}

impl std::fmt::Display for AliasKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AliasKind::Link => f.write_str("link (symlink or junction)"),
            AliasKind::HardLink => f.write_str("hard link"),
        }
    }
}

pub(crate) fn kind_at(path: &Path) -> Result<Option<AliasKind>, StorageError> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(StorageError::Io {
                op: "inspect a write target",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if meta.file_type().is_symlink() {
        return Ok(Some(AliasKind::Link));
    }

    if !meta.is_file() {
        return Ok(None);
    }
    Ok(match links_of(path, &meta)? {
        Some(links) if links > 1 => Some(AliasKind::HardLink),
        Some(_) | None => None,
    })
}

#[cfg(unix)]
fn links_of(_path: &Path, meta: &std::fs::Metadata) -> Result<Option<u64>, StorageError> {
    use std::os::unix::fs::MetadataExt;
    Ok(Some(meta.nlink()))
}

#[cfg(windows)]
fn links_of(path: &Path, _meta: &std::fs::Metadata) -> Result<Option<u64>, StorageError> {
    use std::os::windows::fs::OpenOptionsExt;

    const NO_FOLLOW: u32 = 0x0020_0000 | 0x0200_0000;
    let counted = std::fs::OpenOptions::new()
        .access_mode(0)
        .custom_flags(NO_FOLLOW)
        .open(path)
        .and_then(|file| winapi_util::file::information(&file));
    match counted {
        Ok(info) => Ok(Some(info.number_of_links())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(StorageError::Io {
            op: "count the names of a write target",
            path: path.to_path_buf(),
            source,
        }),
    }
}

pub(crate) fn refused(op: &'static str, path: &Path, kind: AliasKind) -> StorageError {
    StorageError::Alias {
        op,
        path: path.to_path_buf(),
        kind,
    }
}

pub struct WriteTarget(PathBuf);

impl std::fmt::Debug for WriteTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WriteTarget").field(&self.0).finish()
    }
}

impl WriteTarget {

    pub fn at(op: &'static str, path: &Path) -> Result<WriteTarget, StorageError> {
        for parent in path.ancestors().skip(1) {
            if let Some(kind) = kind_at(parent)? {
                return Err(refused(op, parent, kind));
            }
        }
        if let Some(kind) = kind_at(path)? {
            return Err(refused(op, path, kind));
        }
        Ok(WriteTarget(path.to_path_buf()))
    }

    pub fn within(op: &'static str, root: &Path, path: &Path) -> Result<WriteTarget, StorageError> {
        for at in path.ancestors().take_while(|at| *at != root) {
            if let Some(kind) = kind_at(at)? {
                return Err(refused(op, at, kind));
            }
        }
        Ok(WriteTarget(path.to_path_buf()))
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    pub fn replace(self, bytes: &[u8]) -> Result<(), StorageError> {
        crate::bundle::landing::land(
            &mut crate::real_fs::RealFs::new(),
            self,
            bytes,
            crate::bundle::landing::Bits::OfReplaced,
        )
    }

    pub fn create(self, bytes: &[u8]) -> Result<(), StorageError> {
        crate::bundle::landing::create(self, bytes)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
pub(crate) mod tests {
    use super::*;
    use crate::bundle::landing::{Bits, land};
    use crate::real_fs::RealFs;

    #[cfg(windows)]
    #[expect(clippy::disallowed_methods, reason = "test fixture (child D4)")]
    pub(crate) fn place_link(_file: bool, from: &Path, to: &Path) -> bool {
        std::process::Command::new("cmd")
            .args([
                "/c",
                "mklink",
                "/J",
                &to.display().to_string(),
                &from.display().to_string(),
            ])
            .output()
            .is_ok_and(|out| out.status.success())
    }

    #[cfg(not(windows))]
    pub(crate) fn place_link(_file: bool, from: &Path, to: &Path) -> bool {
        std::os::unix::fs::symlink(from, to).is_ok()
    }

    pub(crate) fn place_hard_link(from: &Path, to: &Path) -> bool {
        std::fs::hard_link(from, to).is_ok()
    }

    #[test]
    fn a_plain_name_clears_and_so_does_one_that_is_not_there_yet() {
        let tmp = tempfile::tempdir().unwrap();

        let root = tmp.path().canonicalize().unwrap();
        let plain = root.join("work").join("a.txt");
        std::fs::create_dir_all(plain.parent().unwrap()).unwrap();
        std::fs::write(&plain, b"one").unwrap();
        WriteTarget::at("write a file", &plain).unwrap();
        WriteTarget::at("write a file", &root.join("work").join("new.txt")).unwrap();
    }

    #[test]
    #[allow(
        clippy::permissions_set_readonly_false,
        reason = "the temporary directory cannot be removed around a read-only file on Windows"
    )]
    fn a_second_name_is_a_hard_link_and_nothing_else_is() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let first = root.join("first.txt");
        let second = root.join("second.txt");
        std::fs::write(&first, b"one inode").unwrap();
        std::fs::hard_link(&first, &second).unwrap();
        let read_only = root.join("read-only.txt");
        std::fs::write(&read_only, b"kept").unwrap();
        let mut bits = std::fs::metadata(&read_only).unwrap().permissions();
        bits.set_readonly(true);
        std::fs::set_permissions(&read_only, bits.clone()).unwrap();
        let held = root.join("held.txt");
        std::fs::write(&held, b"held").unwrap();
        let mut open = std::fs::OpenOptions::new();
        open.read(true).write(true);
        #[cfg(windows)]
        std::os::windows::fs::OpenOptionsExt::share_mode(&mut open, 0);
        let holder = open.open(&held).unwrap();

        let judged = [&first, &second, &read_only, &held].map(|path| kind_at(path).unwrap());
        drop(holder);
        bits.set_readonly(false);
        std::fs::set_permissions(&read_only, bits).unwrap();
        assert_eq!(
            judged,
            [
                Some(AliasKind::HardLink),
                Some(AliasKind::HardLink),
                None,
                None
            ]
        );
    }

    #[test]
    fn a_link_at_the_name_or_above_it_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let hooks = tmp.path().join(".git").join("hooks");
        std::fs::create_dir_all(&hooks).unwrap();
        let hook = hooks.join("pre-run");
        std::fs::write(&hook, b"hook-body").unwrap();
        std::fs::create_dir_all(tmp.path().join("work")).unwrap();

        let at_name = tmp.path().join("work").join("a-dir");
        if place_link(true, &hooks, &at_name) {
            let err = WriteTarget::at("write a file", &at_name).unwrap_err();
            assert!(
                matches!(
                    err,
                    StorageError::Alias {
                        kind: AliasKind::Link,
                        ..
                    }
                ),
                "{err}"
            );
        }

        let above = tmp.path().join("work").join("alias-dir");
        if place_link(true, &hooks, &above) {
            let err = WriteTarget::at("write a file", &above.join("pre-run")).unwrap_err();
            assert!(
                matches!(
                    err,
                    StorageError::Alias {
                        kind: AliasKind::Link,
                        ..
                    }
                ),
                "{err}"
            );
        }
        assert_eq!(std::fs::read(&hook).unwrap(), b"hook-body");
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 32, ..proptest::prelude::ProptestConfig::default() })]

        #[test]
        fn an_alias_never_lands(
            kind in 0..3u8,
            protected in proptest::bool::ANY,
        ) {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            let (kept, kept_dir) = if protected {
                let hooks = root.join(".git").join("hooks");
                std::fs::create_dir_all(&hooks).unwrap();
                (hooks.join("kept.txt"), hooks)
            } else {
                let plain = root.join("plain");
                std::fs::create_dir_all(&plain).unwrap();
                (plain.join("kept.txt"), plain)
            };
            std::fs::write(&kept, b"kept").unwrap();
            let zone = root.join("work");
            std::fs::create_dir_all(&zone).unwrap();

            let (target, placed) = match kind {
                0 => {
                    let at = zone.join("out.txt");
                    let placed = place_hard_link(&kept, &at);
                    (at, placed)
                }
                1 => {
                    let at = zone.join("out.txt");
                    let placed = place_link(true, &kept_dir, &at);
                    (at, placed)
                }
                _ => {
                    let above = zone.join("alias-dir");
                    let placed = place_link(true, &kept_dir, &above);
                    (above.join(kept.file_name().unwrap()), placed)
                }
            };

            if placed {
                let mut vfs = RealFs::new();
                match WriteTarget::at("write a bundle file", &target) {
                    Ok(cleared) => {

                        land(&mut vfs, cleared, b"landed", Bits::OfReplaced).unwrap();
                    }
                    Err(StorageError::Alias { .. }) => {}
                    Err(other) => panic!("{other}"),
                }
            }
            proptest::prop_assert_eq!(std::fs::read(&kept).unwrap(), b"kept".to_vec());
        }
    }

    #[test]
    fn two_racing_creates_admit_one() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let name = root.join("draft.md");
        let start = std::sync::Arc::new(std::sync::Barrier::new(8));
        let racers: Vec<_> = (0u8..8)
            .map(|racer| {
                let (root, name, start) = (root.clone(), name.clone(), start.clone());
                std::thread::spawn(move || {
                    let target = WriteTarget::within("create a file", &root, &name).unwrap();
                    start.wait();
                    target.create(&[b'0' + racer]).map(|()| racer)
                })
            })
            .collect();
        let answers: Vec<Result<u8, StorageError>> = racers
            .into_iter()
            .map(|racer| racer.join().unwrap())
            .collect();
        let won: Vec<u8> = answers
            .iter()
            .filter_map(|answer| answer.as_ref().ok().copied())
            .collect();
        assert_eq!(
            won.len(),
            1,
            "exactly one create claims the name: {answers:?}"
        );
        assert!(
            answers
                .iter()
                .filter(|answer| answer.is_err())
                .all(|answer| matches!(answer, Err(StorageError::NameTaken { .. }))),
            "every other create meets the name taken: {answers:?}"
        );
        assert_eq!(std::fs::read(&name).unwrap(), vec![b'0' + won[0]]);
    }
}

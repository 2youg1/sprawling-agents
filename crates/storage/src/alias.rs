// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Aliases, and the write target that refuses them (`crates/storage/spec/Alias.lean` §8-25).
//!
//! A name in a working tree can point at a file other than itself: a
//! symlink or a junction makes the path lead somewhere else, and a hard
//! link gives one inode two names. A write through such a name lands
//! where the name does not say - and a name inside a write domain that
//! points at `.git` is privilege escalation past every address-level
//! check, because the address the gate judged is not the file the disk
//! opens.
//!
//! The rule this module owns: **the alias family is refused whole** -
//! junction, symlink and hard link alike, literally, on every platform.
//! The alternative, skipping an alias and counting it, lands part of a
//! write and breaks the all-or-nothing contract the restore face is
//! built on.

use std::path::{Path, PathBuf};

use crate::error::StorageError;

/// The aliases one name can be. On Windows a junction and a symbolic
/// link are both reparse points and `file_type().is_symlink()` answers
/// true for either, which is why they share a variant: refusing the
/// family is one rule, not one rule per reparse tag. The third member
/// of the family - the hard link - is a regular file whose link count is
/// above one (`crates/storage/spec/Alias.lean` §8-25, `crates/storage/Spec.lean` §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasKind {
    /// A symbolic link or a directory junction: the name leads
    /// somewhere else entirely.
    Link,
    /// A file with more than one name: one write lands in every name at
    /// once.
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

/// What one name is, right now: absent, plain, or an alias.
///
/// Absent is an answer, not a failure - a write target that is not
/// there yet is the ordinary way a file gets created. The question is
/// asked with `symlink_metadata`, so a dangling link answers `Link`
/// rather than disappearing. A hard link is one inode under two names,
/// and a write at one name changes the other, so a regular file with a
/// link count above one is one.
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
    // Only a regular file counts: a directory's link count measures its
    // subdirectories, not aliasing.
    if !meta.is_file() {
        return Ok(None);
    }
    Ok(match links_of(path, &meta)? {
        Some(links) if links > 1 => Some(AliasKind::HardLink),
        Some(_) | None => None,
    })
}

/// How many names the regular file at `path` has; `None` when it is gone
/// since `meta` was read.
#[cfg(unix)]
fn links_of(_path: &Path, meta: &std::fs::Metadata) -> Result<Option<u64>, StorageError> {
    use std::os::unix::fs::MetadataExt;
    Ok(Some(meta.nlink()))
}

/// How many names the regular file at `path` has, read through a handle,
/// because std reports the count on Windows only on the unstable
/// `windows_by_handle` feature. The handle asks for no data access, so it
/// collides with no other handle's share mode - a file another process
/// holds exclusively is counted like any other - and it does not follow
/// a reparse point, although a link never reaches here. `None` when the
/// file is gone since `_meta` was read.
#[cfg(windows)]
fn links_of(path: &Path, _meta: &std::fs::Metadata) -> Result<Option<u64>, StorageError> {
    use std::os::windows::fs::OpenOptionsExt;
    /// `FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS`, as the
    /// Win32 headers spell them.
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

/// The refusal every face greets an alias with.
pub(crate) fn refused(op: &'static str, path: &Path, kind: AliasKind) -> StorageError {
    StorageError::Alias {
        op,
        path: path.to_path_buf(),
        kind,
    }
}

/// A write target the city has cleared: no alias at the name or at any
/// directory above it, so a write here lands exactly where the name
/// says.
///
/// The invariant lives at the one constructor and the field is private,
/// so an unchecked path cannot be spelled - every write face takes this
/// value and nothing else (trybuild `tests/ui/write_target.rs`). What
/// the value proves is that the name held no alias at the moment
/// `at` asked; the window between that question and the write is the
/// filesystem's, which is why the walking faces ask the same rule at
/// their own sampling point.
pub struct WriteTarget(PathBuf);

/// Names the path it cleared and nothing else: there is no hidden
/// state to show and a write target's whole story is where it lands.
impl std::fmt::Debug for WriteTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WriteTarget").field(&self.0).finish()
    }
}

impl WriteTarget {
    /// The one constructor: `path` and every directory above it are
    /// examined, and any alias refuses the write whole.
    ///
    /// # Errors
    /// `StorageError::Alias` naming the alias and its kind;
    /// `StorageError::Io` when a component exists and cannot be examined.
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

    /// Clears `path` for a write inside `root`, the directory the person
    /// chose: `path` and every directory between it and `root` are
    /// examined, and `root` and everything above it are not, because
    /// where a person keeps a city is their placement rather than a
    /// write a run could redirect (`crates/storage/spec/Bundle.lean` §8-12). A `path` outside
    /// `root` is examined up to the filesystem root, as [`WriteTarget::at`]
    /// does.
    ///
    /// The bound is found by comparing path components as spelled, so the
    /// caller builds `path` by joining onto the same `root` it passes; a
    /// `root` in another spelling (a trailing separator, a `.` component,
    /// other case on Windows) is not found, and the walk falls back to the
    /// filesystem root, which is safe but loses the bound.
    ///
    /// # Errors
    /// As [`WriteTarget::at`], for an alias at or below the bound.
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

    /// Replaces the file at this name with `bytes`: a staging file
    /// beside it, flushed, given the replaced file's permissions and
    /// renamed over the name (`crates/storage/Spec.lean` §8-12, §8-32). A reader finds
    /// the old file or the whole new one.
    ///
    /// # Errors
    /// `StorageError::Io` for a write, flush, permission copy or rename
    /// the filesystem refuses; the name keeps its previous content.
    pub fn replace(self, bytes: &[u8]) -> Result<(), StorageError> {
        crate::bundle::landing::land(
            &mut crate::real_fs::RealFs::new(),
            self,
            bytes,
            crate::bundle::landing::Bits::OfReplaced,
        )
    }

    /// Creates the file at this name only when nothing stands there:
    /// the name is claimed by the filesystem at the moment of the write,
    /// so of two racing creates one succeeds (`crates/storage/spec/Alias.lean` §8-32).
    ///
    /// # Errors
    /// `StorageError::NameTaken` when anything stands at the name, with
    /// nothing written; `StorageError::Io` for a create, write or flush
    /// the filesystem refuses.
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

    /// Places a junction at `to` leading to `from` (a symlink off
    /// Windows). `false` when this machine hands out no link at all -
    /// in that arm nothing on the disk has changed.
    #[cfg(windows)]
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
        // Resolved first: `at` walks to the filesystem root, and on macOS
        // every temporary directory sits below `/var`, which is a link.
        let root = tmp.path().canonicalize().unwrap();
        let plain = root.join("work").join("a.txt");
        std::fs::create_dir_all(plain.parent().unwrap()).unwrap();
        std::fs::write(&plain, b"one").unwrap();
        WriteTarget::at("write a file", &plain).unwrap();
        WriteTarget::at("write a file", &root.join("work").join("new.txt")).unwrap();
    }

    /// The hard-link arm is judged by the link count on every platform:
    /// both names of a file with two are hard links, and a file that is
    /// only read-only, or held open by another handle that shares
    /// nothing, is not an alias - the count is read without asking for
    /// any access that could collide with either.
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

    /// The link arm of the alias family: at the name or above it, a
    /// junction or symlink refuses the write whole.
    #[test]
    fn a_link_at_the_name_or_above_it_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let hooks = tmp.path().join(".git").join("hooks");
        std::fs::create_dir_all(&hooks).unwrap();
        let hook = hooks.join("pre-run");
        std::fs::write(&hook, b"hook-body").unwrap();
        std::fs::create_dir_all(tmp.path().join("work")).unwrap();

        // At the name: the name itself leads into `.git`.
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
        // Above the name: the directory the write lands in leads there.
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
        /// 别名永不落盘: whatever alias this machine can make - a hard
        /// link at the name, a link at the name, a link above the name
        /// - aimed at protected or plain metadata, not one byte lands
        /// through it. A machine that grants no link privilege changes
        /// nothing either, which is the same property from the other
        /// side.
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

            // 0: a hard link at the name; 1: a link at the name;
            // 2: a link above the name.
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
                        // Every alias above is refused; a name that
                        // cleared lands in a fresh entry all the same.
                        land(&mut vfs, cleared, b"landed", Bits::OfReplaced).unwrap();
                    }
                    Err(StorageError::Alias { .. }) => {}
                    Err(other) => panic!("{other}"),
                }
            }
            proptest::prop_assert_eq!(std::fs::read(&kept).unwrap(), b"kept".to_vec());
        }
    }

    /// Eight creates of one name, released together: the filesystem
    /// claims the name for one of them, every other meets the name
    /// taken, and the bytes on the disk are the winner's.
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

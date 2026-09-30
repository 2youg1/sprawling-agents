// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Aliases, and the write target that refuses them (storage-SPEC 8-25).
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
//! junction, symlink and hard link alike - literally where this crate
//! reads the link count, and by landing the write in a fresh entry where
//! it does not yet. The alternative, skipping an alias and counting it, lands
//! part of a write and breaks the all-or-nothing contract the restore
//! face is built on.

use std::path::{Path, PathBuf};

use crate::error::StorageError;

/// The aliases one name can be. On Windows a junction and a symbolic
/// link are both reparse points and `file_type().is_symlink()` answers
/// true for either, which is why they share a variant: refusing the
/// family is one rule, not one rule per reparse tag. The third member
/// of the family - the hard link - is classified where this crate reads
/// a link count (Unix `nlink`) and is answered by the write mechanics
/// where it does not yet (Windows; storage-SPEC 8-25 and §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasKind {
    /// A symbolic link or a directory junction: the name leads
    /// somewhere else entirely.
    Link,
    /// A file with more than one name: one write lands in every name at
    /// once. Produced only where a platform can report it.
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
/// and a write at one name changes the other; Unix reports the link
/// count on the metadata; std reports it on Windows only through the
/// unstable `windows_by_handle` feature and the safe third-party reader
/// is not adopted (storage-SPEC §3), so there the write faces answer it
/// by landing a fresh entry instead (storage-SPEC 8-25, §3).
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
    // subdirectories, not aliasing, and Windows is answered by the write
    // faces instead (storage-SPEC §3).
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.is_file() && meta.nlink() > 1 {
            return Ok(Some(AliasKind::HardLink));
        }
    }
    Ok(None)
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
    /// write a run could redirect (storage-SPEC 8-12). A `path` outside
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
                        // The face lands the write in a fresh entry:
                        // every other name of the inode keeps its bytes.
                        land(&mut vfs, cleared, b"landed", Bits::OfReplaced).unwrap();
                    }
                    Err(StorageError::Alias { .. }) => {}
                    Err(other) => panic!("{other}"),
                }
            }
            proptest::prop_assert_eq!(std::fs::read(&kept).unwrap(), b"kept".to_vec());
        }
    }
}

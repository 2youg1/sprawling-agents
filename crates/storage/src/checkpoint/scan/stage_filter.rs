// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The checkpoint admission rule for one staged path: what enters, what is
//! skipped, and what refuses the wave (`crates/storage/Spec.lean` §8-8 and §8-25).

use crate::error::StorageError;

/// The repository's working tree, which staged paths are relative to.
pub(super) fn workdir(repo: &git2::Repository) -> Result<&std::path::Path, StorageError> {
    repo.workdir().ok_or_else(|| StorageError::Checkpoint {
        op: "stage a wave",
        detail: "the city repository is bare".to_owned(),
    })
}

/// The checkpoint's admission rule for one staged path: what enters a
/// checkpoint, what is skipped, and what refuses the wave (`crates/storage/spec/Checkpoint.lean` §8-8).
///
/// git asks with 1 for skip and 0 for stage, and hands over paths
/// relative to the repository's working tree - so the alias question is
/// asked of the joined path, or it asks about the process's directory
/// instead of the city's. Session projections and protected metadata
/// are skipped - their bytes have another home and a checkpoint has no
/// business carrying them. A link is never skipped: one anywhere in the
/// scope refuses the whole wave, because a name that leads into a
/// reserved file would capture reserved bytes under a lying name, and
/// the `file_discarded` restoration would write back through it
/// (`crates/storage/spec/Alias.lean` §8-25).
pub(super) struct StageFilter {
    root: std::path::PathBuf,
    aliases: Vec<(String, crate::alias::AliasKind)>,
    fault: Option<StorageError>,
}

impl StageFilter {
    pub(super) fn new(root: &std::path::Path) -> StageFilter {
        StageFilter {
            root: root.to_path_buf(),
            aliases: Vec::new(),
            fault: None,
        }
    }

    pub(super) fn admit(&mut self, relative: &std::path::Path) -> i32 {
        if crate::sessions::is_session_projection(relative)
            || !crate::reserved::outside_reserved(relative)
        {
            return 1;
        }
        match crate::alias::kind_at(&self.root.join(relative)) {
            Ok(None) => 0,
            Ok(Some(kind)) => {
                self.aliases.push((relative.display().to_string(), kind));
                1
            }
            Err(err) => {
                self.fault = Some(err);
                1
            }
        }
    }

    /// The wave's verdict. The refusal names the first alias in path
    /// order, so the same scope refuses with the same sentence on every
    /// machine.
    pub(super) fn refused(mut self) -> Result<(), StorageError> {
        if let Some(err) = self.fault {
            return Err(err);
        }
        self.aliases.sort_by(|left, right| left.0.cmp(&right.0));
        match self.aliases.into_iter().next() {
            Some((path, kind)) => Err(crate::alias::refused(
                "stage a wave",
                std::path::Path::new(&path),
                kind,
            )),
            None => Ok(()),
        }
    }
}

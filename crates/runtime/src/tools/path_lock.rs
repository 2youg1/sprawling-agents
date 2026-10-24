// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One lock per file an edit lands on, shared by every run in the
//! process.
//!
//! Inside it an edit takes the file's stamp, reads its bytes, judges the
//! admission, replaces the one match and takes the new stamp, so two runs
//! editing one file cannot both land on the same look and lose each
//! other's change (`crates/runtime/spec/Tools/Edit.lean` §8-64). It
//! serialises edit with edit only: `exec` and the person's editor do not
//! take it.
//!
//! The key is the path in the tree the run writes to, so one address in
//! two worktrees is two files and two locks. A path is in the table only
//! while somebody holds it, so the table is as large as the edits in
//! flight.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

/// The city's table of files an edit is landing on.
#[derive(Debug, Clone, Default)]
pub struct PathLocks {
    table: Arc<Table>,
}

#[derive(Debug, Default)]
struct Table {
    held: Mutex<BTreeSet<PathBuf>>,
    freed: Condvar,
}

/// The lock on one path; dropping it lets the next edit of that path in.
#[derive(Debug)]
pub(crate) struct PathGuard {
    table: Arc<Table>,
    path: PathBuf,
}

impl PathLocks {
    /// Waits until no other edit holds `path`, then holds it.
    pub(crate) fn hold(&self, path: &Path) -> PathGuard {
        let mut held = self.table.held();
        while held.contains(path) {
            held = self
                .table
                .freed
                .wait(held)
                .unwrap_or_else(PoisonError::into_inner);
        }
        held.insert(path.to_path_buf());
        PathGuard {
            table: Arc::clone(&self.table),
            path: path.to_path_buf(),
        }
    }
}

impl Table {
    /// A poisoned table is taken back: each entry is inserted or removed
    /// whole, so a thread that died holding the table left no half-entry.
    fn held(&self) -> MutexGuard<'_, BTreeSet<PathBuf>> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        self.table.held().remove(&self.path);
        self.table.freed.notify_all();
    }
}

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
//! two worktrees is two files and two locks. It is compared with its case
//! folded, because Windows and macOS volumes name one file by every
//! spelling of its case, and two runs that spelled it differently would
//! otherwise hold two locks on one file; on a case-sensitive volume the
//! folding only makes two files that differ by case wait on each other.
//! A path is in the table only while somebody holds it, so the table is
//! as large as the edits in flight.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

/// The city's table of files an edit is landing on.
#[derive(Debug, Clone, Default)]
pub struct PathLocks {
    table: Arc<Table>,
}

#[derive(Debug, Default)]
struct Table {
    held: Mutex<BTreeSet<String>>,
    freed: Condvar,
}

/// The lock on one path; dropping it lets the next edit of that path in.
#[derive(Debug)]
pub(crate) struct PathGuard {
    table: Arc<Table>,
    key: String,
}

impl PathLocks {
    /// Waits until no other edit holds `path`, then holds it.
    pub(crate) fn hold(&self, path: &Path) -> PathGuard {
        let key = path.to_string_lossy().to_lowercase();
        let mut held = self.table.held();
        while held.contains(&key) {
            held = self
                .table
                .freed
                .wait(held)
                .unwrap_or_else(PoisonError::into_inner);
        }
        held.insert(key.clone());
        PathGuard {
            table: Arc::clone(&self.table),
            key,
        }
    }
}

impl Table {
    /// A poisoned table is taken back: each entry is inserted or removed
    /// whole, so a thread that died holding the table left no half-entry.
    fn held(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        self.table.held().remove(&self.key);
        self.table.freed.notify_all();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::PathLocks;
    use std::path::Path;
    use std::sync::mpsc;
    use std::time::Duration;

    /// Two spellings of one name on a case-insensitive volume are one
    /// file, so the second edit waits for the first rather than landing
    /// on the same look.
    #[test]
    fn a_path_spelled_in_another_case_waits_for_the_holder() {
        let locks = PathLocks::default();
        let first = locks.hold(Path::new("city/work/Notes.md"));
        let (entered, waited) = mpsc::channel();
        let other = locks.clone();
        let second = std::thread::spawn(move || {
            let _held = other.hold(Path::new("city/work/notes.md"));
            entered.send(()).unwrap();
        });
        assert!(waited.recv_timeout(Duration::from_millis(200)).is_err());
        drop(first);
        waited.recv_timeout(Duration::from_secs(10)).unwrap();
        second.join().unwrap();
    }
}

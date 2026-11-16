// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

#[derive(Debug, Clone, Default)]
pub struct PathLocks {
    table: Arc<Table>,
}

#[derive(Debug, Default)]
struct Table {
    held: Mutex<BTreeSet<String>>,
    freed: Condvar,
}

#[derive(Debug)]
pub(crate) struct PathGuard {
    table: Arc<Table>,
    key: String,
}

impl PathLocks {
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

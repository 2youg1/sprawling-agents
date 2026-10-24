// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A file's stamp, the stamps one run has seen, and the admission an
//! edit gets from them.
//!
//! `crates/runtime/spec/Tools/Edit.lean` §8-64 models the admission and
//! proves what it must hold over every interleaving of runs: a run whose
//! last look at a file was followed by anybody else's write lands nothing
//! there. [`admission`] is that model's decision, arm for arm, and the
//! proptest in `stamp::tests` drives it on random traces.
//!
//! A stamp is the file's length, its modification time and its identity
//! on the volume, read in one query of an open handle. No content is hashed: a
//! read pays for the bytes it shows and nothing else, and an edit's own
//! write lands through a staging file renamed over the name, so its
//! identity changes on every write the tool makes.

use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use kernel::Address;

/// What a file is at one moment, as far as an edit needs to know: its
/// length, its last write time and which file on which volume it is,
/// all read from one query of an open handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    len: u64,
    written: Ticks,
    volume: u64,
    file: u64,
}

/// A last write time as the platform counts it: seconds and nanoseconds
/// on Unix, 100-nanosecond intervals on Windows. Only equality is asked
/// of it, so no conversion between the two is needed.
#[cfg(unix)]
type Ticks = (i64, i64);
#[cfg(windows)]
type Ticks = Option<u64>;

impl Stamp {
    /// The stamp of whatever stands at `path` now, or `None` when nothing
    /// does.
    ///
    /// # Errors
    /// Whatever the platform answers when the file exists and cannot be
    /// opened or asked about.
    pub(crate) fn of(path: &Path) -> io::Result<Option<Stamp>> {
        match File::open(path) {
            Ok(file) => Stamp::of_file(&file).map(Some),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// The stamp of the file `file` is open on, in one `fstat`.
    ///
    /// # Errors
    /// Whatever the platform answers when the handle is asked about.
    #[cfg(unix)]
    pub(crate) fn of_file(file: &File) -> io::Result<Stamp> {
        use std::os::unix::fs::MetadataExt;
        let meta = file.metadata()?;
        Ok(Stamp {
            len: meta.len(),
            written: (meta.mtime(), meta.mtime_nsec()),
            volume: meta.dev(),
            file: meta.ino(),
        })
    }

    /// The stamp of the file `file` is open on, in one
    /// `GetFileInformationByHandle`: std exposes the file index only on
    /// an unstable API, and `winapi-util` asks the same handle through a
    /// safe surface.
    ///
    /// # Errors
    /// Whatever the platform answers when the handle is asked about.
    #[cfg(windows)]
    pub(crate) fn of_file(file: &File) -> io::Result<Stamp> {
        let info = winapi_util::file::information(file)?;
        Ok(Stamp {
            len: info.file_size(),
            written: info.last_write_time(),
            volume: info.volume_serial_number(),
            file: info.file_index(),
        })
    }
}

/// The stamps one run has seen, by the address it read them under.
///
/// One table per run, shared by `read` and `edit`, and held only in
/// memory: every run the ledger can start again - a session continued
/// after a restart, a fork, a successor, a delegate's child - starts
/// with none, so there is nothing to rebuild from the ledger (§8-64).
#[derive(Debug, Clone, Default)]
pub struct ReadStamps {
    seen: Arc<Mutex<BTreeMap<Address, Stamp>>>,
}

impl ReadStamps {
    /// Records a look at `at`, taken between two stamps. Nothing is
    /// recorded when they differ, because the bytes shown may belong to
    /// either side of a write; the next edit is then refused as unread,
    /// which costs a read and never lets a write through.
    pub(crate) fn look(&self, at: &Address, before: Option<Stamp>, after: Option<Stamp>) {
        if let (Some(before), Some(after)) = (before, after)
            && before == after
        {
            self.table().insert(at.clone(), after);
        }
    }

    /// The stamp this run last saw at `at`.
    pub(crate) fn last(&self, at: &Address) -> Option<Stamp> {
        self.table().get(at).cloned()
    }

    /// A poisoned table still holds stamps that were each written whole
    /// by one `insert`, so it is taken back rather than refused.
    fn table(&self) -> std::sync::MutexGuard<'_, BTreeMap<Address, Stamp>> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What the call asks for: to replace text in a file that stands, or to
/// make a file where none does (`old` is empty).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intent {
    Replace,
    Create,
}

/// The admission an edit gets, decided inside the path lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    /// The run's stamp is the file's, or the file is absent and the call
    /// creates it.
    Lands,
    /// The file stands and this run never looked at it.
    Unread,
    /// The file changed after this run's last look.
    Moved,
    /// Nothing stands at the path and the call replaces text.
    Absent,
    /// Something stands at the path and the call creates it.
    Present,
}

/// The decision `crates/runtime/spec/Tools/Edit.lean` names `admission`.
pub(crate) fn admission(
    seen: Option<&Stamp>,
    current: Option<&Stamp>,
    intent: Intent,
) -> Admission {
    match (current, intent) {
        (None, Intent::Create) => Admission::Lands,
        (None, Intent::Replace) => Admission::Absent,
        (Some(_), Intent::Create) => Admission::Present,
        (Some(current), Intent::Replace) => match seen {
            None => Admission::Unread,
            Some(seen) if seen == current => Admission::Lands,
            Some(_) => Admission::Moved,
        },
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;

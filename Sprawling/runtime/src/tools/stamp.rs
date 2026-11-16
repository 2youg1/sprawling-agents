// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use kernel::{Address, TimeMs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    len: u64,
    written: Ticks,
    volume: u64,
    file: u64,
}

#[cfg(unix)]
type Ticks = (i64, i64);
#[cfg(windows)]
type Ticks = Option<u64>;

pub(crate) struct FileFacts {
    stamp: Stamp,
    #[cfg_attr(
        not(any(windows, target_os = "linux", target_os = "macos", test)),
        expect(
            dead_code,
            reason = "unchanged evidence is queried only by handle fast paths"
        )
    )]
    is_file: bool,
}

#[cfg_attr(
    not(any(windows, target_os = "linux", target_os = "macos", test)),
    expect(
        dead_code,
        reason = "unchanged evidence is queried only by handle fast paths"
    )
)]
impl FileFacts {
    pub(crate) fn stamp(&self) -> &Stamp {
        &self.stamp
    }

    pub(crate) fn is_file(&self) -> bool {
        self.is_file
    }

    #[cfg_attr(
        not(any(windows, target_os = "linux", target_os = "macos", test)),
        expect(
            dead_code,
            reason = "unchanged evidence is queried only by handle fast paths"
        )
    )]
    pub(crate) fn len(&self) -> u64 {
        self.stamp.len()
    }
}

impl Stamp {
    #[cfg_attr(
        not(any(windows, target_os = "linux", target_os = "macos", test)),
        expect(
            dead_code,
            reason = "unchanged evidence is queried only by handle fast paths"
        )
    )]
    pub(crate) fn len(&self) -> u64 {
        self.len
    }

    pub(crate) fn racy_at(&self, now: TimeMs) -> bool {
        let Some(modified) = self.modified_ms() else {
            return true;
        };
        now.value().saturating_sub(modified) < 2_000
    }

    #[cfg(unix)]
    fn modified_ms(&self) -> Option<u64> {
        let (seconds, nanoseconds) = self.written;
        let millis = i128::from(seconds)
            .checked_mul(1_000)?
            .checked_add(i128::from(nanoseconds) / 1_000_000)?;
        u64::try_from(millis).ok()
    }

    #[cfg(windows)]
    fn modified_ms(&self) -> Option<u64> {
        const UNIX_EPOCH: u64 = 116_444_736_000_000_000;
        let since_epoch = self.written?.checked_sub(UNIX_EPOCH)?;
        Some(since_epoch / 10_000)
    }

    pub(crate) fn of(path: &Path) -> io::Result<Option<Stamp>> {
        match File::open(path) {
            Ok(file) => Stamp::of_file(&file).map(Some),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    #[cfg(unix)]
    pub(crate) fn of_file(file: &File) -> io::Result<Stamp> {
        Ok(Stamp::facts(file)?.stamp)
    }

    #[cfg(unix)]
    pub(crate) fn facts(file: &File) -> io::Result<FileFacts> {
        use std::os::unix::fs::MetadataExt;
        let meta = file.metadata()?;
        Ok(FileFacts {
            stamp: Stamp {
                len: meta.len(),
                written: (meta.mtime(), meta.mtime_nsec()),
                volume: meta.dev(),
                file: meta.ino(),
            },
            is_file: meta.is_file(),
        })
    }

    #[cfg(windows)]
    pub(crate) fn of_file(file: &File) -> io::Result<Stamp> {
        Ok(Stamp::facts(file)?.stamp)
    }

    #[cfg(windows)]
    pub(crate) fn facts(file: &File) -> io::Result<FileFacts> {
        const FILE_ATTRIBUTE_DIRECTORY: u64 = 0x10;
        let info = winapi_util::file::information(file)?;
        Ok(FileFacts {
            stamp: Stamp {
                len: info.file_size(),
                written: info.last_write_time(),
                volume: info.volume_serial_number(),
                file: info.file_index(),
            },
            is_file: info.file_attributes() & FILE_ATTRIBUTE_DIRECTORY == 0,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReadStamps {
    seen: Arc<Mutex<BTreeMap<Address, Stamp>>>,
}

impl ReadStamps {
    pub(crate) fn look(&self, at: &Address, before: Option<Stamp>, after: Option<Stamp>) {
        if let (Some(before), Some(after)) = (before, after)
            && before == after
        {
            self.table().insert(at.clone(), after);
        }
    }

    pub(crate) fn last(&self, at: &Address) -> Option<Stamp> {
        self.table().get(at).cloned()
    }

    fn table(&self) -> std::sync::MutexGuard<'_, BTreeMap<Address, Stamp>> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Intent {
    Replace,
    Create,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    Lands,

    Unread,

    Moved,

    Absent,

    Present,
}

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

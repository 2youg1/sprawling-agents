// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

use super::Paths;
use super::failure;
use crate::error::Result;

pub(super) const LOCK_POLL: Duration = Duration::from_millis(100);

/// The wait must exceed one installer's budget, or a waiter reports a stuck
/// lock while a legitimate download is still running (`Mei/Spec.lean` §10).
pub(super) const LOCK_WAIT: Duration = super::install::BUDGET;

pub(super) struct Lock {
    file: std::fs::File,
}

impl Lock {
    pub(super) fn acquire(paths: &Paths, wait: Duration) -> Result<Lock> {
        std::fs::create_dir_all(&paths.root).map_err(|err| {
            failure(
                "create the doctor lock directory",
                err.to_string(),
                "keep the per-user doctor directory writable",
            )
            .at(&paths.root)
        })?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&paths.lock)
            .map_err(|err| {
                failure(
                    "open the doctor lock",
                    err.to_string(),
                    "keep the per-user doctor directory writable",
                )
                .at(&paths.lock)
            })?;
        let polls = wait
            .as_millis()
            .checked_div(LOCK_POLL.as_millis())
            .and_then(|polls| u32::try_from(polls).ok())
            .unwrap_or(u32::MAX);
        for _ in 0..polls {
            match file.try_lock() {
                Ok(()) => return Ok(Lock { file }),
                Err(std::fs::TryLockError::WouldBlock) => std::thread::sleep(LOCK_POLL),
                Err(std::fs::TryLockError::Error(err)) => {
                    return Err(failure(
                        "take the doctor lock",
                        err.to_string(),
                        "keep the doctor directory on a filesystem that supports file locking",
                    )
                    .at(&paths.lock));
                }
            }
        }
        Err(failure(
            "wait for another Mei environment check",
            format!("the lock stayed held for {wait:?}"),
            "let the other Mei finish, or stop that process and start this one again",
        )
        .at(&paths.lock))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        if let Err(err) = self.file.unlock() {
            eprintln!("release the doctor lock: {err}");
        }
    }
}

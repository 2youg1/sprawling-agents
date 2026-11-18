// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::time::Duration;

use super::Paths;
use super::failure;
use crate::error::Result;

pub(super) const LOCK_POLL: Duration = Duration::from_millis(100);

pub(super) const LOCK_WAIT: Duration =
    super::install::BUDGET.saturating_add(Duration::from_secs(5));

pub(super) struct Lock {
    file: std::fs::File,
}

impl Lock {
    #[expect(
        clippy::disallowed_methods,
        reason = "Mei's lock adapter samples monotonic time for its polling deadline"
    )]
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
        let deadline = std::time::Instant::now().checked_add(wait).ok_or_else(|| {
            failure(
                "set the doctor lock deadline",
                format!("the wait budget {wait:?} is not representable"),
                "use a finite doctor wait budget",
            )
            .at(&paths.lock)
        })?;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Lock { file }),
                Err(std::fs::TryLockError::WouldBlock) => {
                    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                    if remaining.is_zero() {
                        break;
                    }
                    std::thread::sleep(LOCK_POLL.min(remaining));
                }
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

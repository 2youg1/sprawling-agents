// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Writing the index while another run of this city holds its lock.
//!
//! Specified by `crates/storage/spec/Checkpoint.lean` §8-8.

use crate::error::StorageError;

use super::super::commit::git_err;

/// Writes the index, waiting out a lock another run of this city holds.
///
/// **`.git/index.lock` is taken for the length of one write, and two runs
/// of one building stage their scopes at the same time by design** -
/// `accounting::worker::plans::pursuing` drives every node of a ready set at once,
/// in one repository. The run that loses that race waits and tries
/// again, because the concurrent process is this city and the lock is
/// held for microseconds; refusing would end the run as cancelled and
/// hand its node back as though its own done check had failed.
///
/// **Bounded, because a lock held by a dead process is a different
/// fact.** Twenty-five milliseconds apart, twenty times: a genuinely
/// stuck lock still surfaces as the refusal it is, and that refusal is
/// what a person can act on.
///
/// Retrying is safe because the index being written is the in-memory one
/// this call built, unchanged by a failed write: the second attempt
/// states the same thing as the first.
pub(in crate::checkpoint) fn write_index(index: &mut git2::Index) -> Result<(), StorageError> {
    const ATTEMPTS: u32 = 20;
    const WAIT: std::time::Duration = std::time::Duration::from_millis(25);
    let mut attempt: u32 = 0;
    loop {
        attempt = attempt.saturating_add(1);
        match index.write() {
            Ok(()) => return Ok(()),
            Err(err) if concurrent(&err) && attempt < ATTEMPTS => {}
            Err(err) => return Err(git_err("write index")(err)),
        }
        std::thread::sleep(WAIT);
    }
}

/// Whether git refused because somebody else holds the index lock.
///
/// libgit2 reports that as an index error whose message names the lock
/// file; anything else - a permission problem, a damaged index - is not a
/// collision and is refused on the first attempt.
fn concurrent(err: &git2::Error) -> bool {
    err.class() == git2::ErrorClass::Index && err.message().contains("index.lock")
}

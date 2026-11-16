// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::error::StorageError;

use super::super::commit::git_err;

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

fn concurrent(err: &git2::Error) -> bool {
    err.class() == git2::ErrorClass::Index && err.message().contains("index.lock")
}

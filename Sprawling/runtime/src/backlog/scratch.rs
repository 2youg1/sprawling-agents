// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::atomic::{AtomicU64, Ordering};

use super::BacklogId;

static BACKLOGS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Scratch(u64);

impl Scratch {

    pub(super) fn open() -> Scratch {

        Scratch(BACKLOGS.fetch_add(1, Ordering::Relaxed))
    }

    pub(super) fn dir(self, id: BacklogId) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "sprawling-{}-{}-{}",
            std::process::id(),
            self.0,
            id
        ))
    }
}

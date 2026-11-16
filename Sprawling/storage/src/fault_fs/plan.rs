// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub struct FaultPlan {
    pub cut_at_op: Option<u64>,

    pub cut_on_write: Option<&'static str>,
    pub torn_tail: TornTail,
}

#[derive(Debug, Clone, Copy)]
pub enum TornTail {
    None,
    KeepBytes(u64),
}

pub(crate) struct FileState {
    pub(crate) durable: Vec<u8>,
    pub(crate) live: Vec<u8>,
    pub(crate) durable_entry: bool,
}

pub(crate) struct State {
    pub(crate) files: BTreeMap<PathBuf, FileState>,
    pub(crate) dirs: BTreeSet<PathBuf>,
    pub(crate) op: u64,

    pub(crate) bytes_read: u64,
    pub(crate) plan: FaultPlan,
}

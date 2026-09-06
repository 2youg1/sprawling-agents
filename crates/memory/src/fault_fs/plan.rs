// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! FaultFs: the second Vfs adapter — a deterministic power-loss model
//!. Compiled for tests and for citysim under the
//! `fault` feature; never in a production build.
//!
//! The model is stricter than any real platform so the write discipline
//! it enforces holds on every platform (memory-SPEC 8-2):
//! - every file has two planes: `durable` (survives power loss) and
//!   `live` (what the running process observes). `sync_data` promotes
//!   live to durable; a power cut drops the unsynced delta, except a
//!   `TornTail` prefix that models a torn write reaching the platter.
//! - a created file's directory entry survives only after `sync_dir` on
//!   its parent — even when its bytes were synced (stricter than POSIX).
//! - removals are instantly durable (simplification: the only remover is
//!   tail recovery, and a resurrected empty segment is harmless — reopen
//!   tolerates it).
//! - the op hitting `cut_at_op` fails with "power lost"; the plan is then
//!   consumed, so the same instance serves the powered reopen. Appends
//!   land on `live` before the cut check so the tear can bite the very
//!   write that died.
//!
//! Everything is explicit in [`FaultPlan`]; there is no randomness.

//! Fault plans: which write dies, and how.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub struct FaultPlan {
    /// 1-based op number that dies with "power lost"; `None` = never.
    pub cut_at_op: Option<u64>,
    /// The first append whose bytes contain this needle dies with "power
    /// lost"; `None` = never. Like `cut_at_op` it fires once and is then
    /// spent, so the same instance serves the powered reopen.
    ///
    /// This exists because an ordinal cannot name a write from outside
    /// this crate. A caller above the ledger knows *which line* it wants
    /// to lose, not how many filesystem operations precede it - and that
    /// count changes whenever anything upstream reads one more file, so
    /// an ordinal written there is a number that silently stops meaning
    /// what it meant. Content is the caller's own vocabulary, and it
    /// stays as explicit and as deterministic as the ordinal is.
    pub cut_on_write: Option<&'static str>,
    pub torn_tail: TornTail,
}

/// How much of each file's unsynced delta the platter kept.
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
    pub(crate) plan: FaultPlan,
}

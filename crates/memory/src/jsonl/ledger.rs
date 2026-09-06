// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Ledger types: the struct, its reports, and segment grammar.

use std::path::{Path, PathBuf};

use kernel::{B3Hash, EventRecord, Seq};

use crate::vfs::Vfs;

/// Segment rolling threshold. Internal affair: changing it changes how
/// files are cut, never any observable semantics (memory-SPEC 14).
pub(crate) const SEGMENT_ROLL_BYTES: u64 = 64 * 1024 * 1024;

/// What open found and repaired.
pub struct OpenReport {
    pub recovered: Option<TailTruncation>,
}

pub struct TailTruncation {
    pub dropped_bytes: u64,
}

/// The durable Ledger. See the module doc for the contract.
pub struct JsonlLedger {
    pub(crate) vfs: Box<dyn Vfs>,
    pub(crate) dir: PathBuf,
    pub(crate) seg_path: PathBuf,
    pub(crate) seg_len: u64,
    pub(crate) next_seq: Seq,
    pub(crate) prev: B3Hash,
    pub(crate) roll_bytes: u64,
    /// The write-path observer, called only after the wave is durable.
    pub(crate) observer: Option<WriteObserver>,
}

/// The write-path observer: what a live reader (the control surface's
/// event stream) installs to hear each line once it is durable.
pub type WriteObserver = Box<dyn FnMut(&EventRecord) + Send>;

/// Chain state at the entrance of the last segment.
pub(crate) struct TailBoundary {
    pub(crate) prev: B3Hash,
    pub(crate) next_seq: Seq,
    pub(crate) prior: Option<PriorSegment>,
}

pub(crate) struct PriorSegment {
    pub(crate) path: PathBuf,
    pub(crate) len: u64,
}

pub(crate) fn segment_file_name(first_seq: Seq) -> String {
    format!("ledger-{:020}.jsonl", first_seq.value())
}

pub(crate) fn is_segment(path: &Path) -> bool {
    match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.starts_with("ledger-") && name.ends_with(".jsonl"),
        None => false,
    }
}

/// Complete (`\n`-terminated) lines and the leftover tail bytes.
pub(crate) fn complete_lines(bytes: &[u8]) -> (Vec<&[u8]>, usize) {
    let mut lines = Vec::new();
    let mut consumed = 0usize;
    let mut start = 0usize;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            if let Some(line) = bytes.get(start..index) {
                lines.push(line);
            }
            start = index.saturating_add(1);
            consumed = start;
        }
    }
    (lines, consumed)
}

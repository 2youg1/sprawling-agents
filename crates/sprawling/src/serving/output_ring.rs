// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What running commands already wrote, kept per run so a page that
//! opens mid-command sees it (sprawling-SPEC.md 8-90).

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, MutexGuard, PoisonError};

use channels::LiveOutput;
use kernel::{EventKind, EventRecord, RunId};

/// The most one run keeps: what the runtime reads in one second at its
/// top rate, and enough for the page to fill both of its streams.
const KEPT_BYTES_PER_RUN: usize = 64 * 1024;

/// A bounded tail of live output for every run still running a command.
///
/// Holds a discardable preview: the call's result in the Ledger is the
/// authority, which is why nothing here returns a failure.
#[derive(Default)]
pub(crate) struct OutputRing {
    runs: Mutex<BTreeMap<RunId, Kept>>,
}

#[derive(Default)]
struct Kept {
    pieces: VecDeque<LiveOutput>,
    bytes: usize,
}

impl OutputRing {
    /// Appends a piece to its run's tail, dropping the oldest whole
    /// pieces past the bound. The newest piece always stays.
    pub(crate) fn keep(&self, piece: &LiveOutput) {
        let mut runs = self.runs();
        let kept = runs.entry(piece.run).or_default();
        kept.bytes = kept.bytes.saturating_add(piece.text.len());
        kept.pieces.push_back(piece.clone());
        while kept.bytes > KEPT_BYTES_PER_RUN && kept.pieces.len() > 1 {
            let Some(oldest) = kept.pieces.pop_front() else {
                break;
            };
            kept.bytes = kept.bytes.saturating_sub(oldest.text.len());
        }
    }

    /// Empties a run's tail once its call's result is in the Ledger.
    pub(crate) fn settle(&self, record: &EventRecord) {
        if record.kind() == EventKind::ToolResult {
            self.runs().remove(&record.run());
        }
    }

    /// Every kept piece, each run's in the order it arrived.
    pub(crate) fn so_far(&self) -> Vec<LiveOutput> {
        self.runs()
            .values()
            .flat_map(|kept| kept.pieces.iter().cloned())
            .collect()
    }

    /// Every operation leaves the map whole within one step, so a lock
    /// poisoned by a panic elsewhere still guards a consistent map.
    fn runs(&self) -> MutexGuard<'_, BTreeMap<RunId, Kept>> {
        self.runs.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use super::*;
    use channels::OutputStream;
    use kernel::{EventDraft, GENESIS_PREV, Payload, Seq, TimeMs};

    fn piece(run: RunId, text: &str) -> LiveOutput {
        LiveOutput {
            run,
            stream: OutputStream::Out,
            text: text.to_owned(),
        }
    }

    fn result_of(run: RunId) -> EventRecord {
        let draft = EventDraft {
            run,
            t: TimeMs::new(0),
            who: "mason@lab.1".into(),
            addr: None,
            kind: EventKind::ToolResult,
            data: Payload::new(serde_json::Map::new()).unwrap(),
            ig: false,
        };
        EventRecord::from_draft(draft, Seq::FIRST, GENESIS_PREV)
    }

    /// A page opening mid-command first sees what the command already
    /// wrote, and nothing of that run once its result has landed.
    #[test]
    fn a_page_opening_mid_command_sees_what_was_written_until_the_result_lands() {
        let (building, testing) = (RunId::from_bytes([1; 16]), RunId::from_bytes([2; 16]));
        let ring = OutputRing::default();
        let written = [
            piece(building, "compiling\n"),
            piece(testing, "running 3 tests\n"),
        ];
        written.iter().for_each(|each| ring.keep(each));
        assert_eq!(ring.so_far(), written.to_vec());

        ring.settle(&result_of(building));
        assert_eq!(ring.so_far(), vec![written[1].clone()]);
    }

    /// A run flooding its stdout holds at most the bound, newest last.
    #[test]
    fn a_flooding_run_keeps_only_its_newest_bytes() {
        let run = RunId::from_bytes([3; 16]);
        let ring = OutputRing::default();
        let line = "x".repeat(1024);
        (0..100).for_each(|_| ring.keep(&piece(run, &line)));
        ring.keep(&piece(run, "last\n"));
        let kept = ring.so_far();
        let bytes: usize = kept.iter().map(|each| each.text.len()).sum();
        assert!(bytes <= KEPT_BYTES_PER_RUN, "kept {bytes} bytes");
        assert_eq!(kept.last().map(|each| each.text.as_str()), Some("last\n"));
    }
}

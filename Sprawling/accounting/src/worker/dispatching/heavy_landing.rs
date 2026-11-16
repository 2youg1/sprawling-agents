// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The heavy half of a run's landing, which the run's lane does before
//! it hands the run home (`crates/sprawling/spec/Accounting/Landing.lean`
//! D37): the transcript beside the room and the checkpoint sweep's
//! diff. Both read only the drive and the disk, so the accounting
//! thread, which every relay request waits on, never waits on them.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use kernel::AxError;

use super::super::recording::Notes;
use super::super::workbench::held;
use super::super::{Assignment, Driven, Site};

/// The checkpoint sweep of one drive, read on its lane before the run
/// came home (`crates/sprawling/spec/Accounting/Landing.lean` D37): the
/// diff grows with every file the run wrote, and on the accounting
/// thread every relay request would wait it out.
pub(in crate::worker) struct Swept {
    /// The drive's first checkpoint, which every discarded file is
    /// restored from.
    pub(in crate::worker) base: String,
    /// What the drive deleted since `base`, or why the tree could not
    /// be read; the landing propagates the refusal as it always did.
    pub(in crate::worker) discarded: Result<Vec<kernel::Payload>, AxError>,
}

/// What a lane needs to do the heavy half of its run's landing before
/// it hands the run home (`crates/sprawling/spec/Accounting/Landing.lean`
/// D37): the steps that read only the drive and the disk.
pub(super) struct HeavyLanding {
    pub(super) city_root: PathBuf,
    /// The store the lanes share, which the transcript is pinned in.
    pub(super) store: Arc<Mutex<storage::Cas>>,
    pub(super) notes: Notes,
    pub(super) staged_at: kernel::Seq,
}

impl HeavyLanding {
    /// Writes the transcript of a run that froze and reads what the
    /// drive deleted since its first checkpoint. A drive that failed
    /// before freezing leaves neither: its landing reads the failure.
    pub(super) fn carry_out(
        self,
        at: &Assignment,
        site: &Site,
        driven: &Result<Driven, AxError>,
    ) -> Option<Swept> {
        let driven = driven.as_ref().ok()?;
        if let Ok(frozen) = &driven.outcome {
            self.keep_transcript(frozen, &at.addr);
        }
        driven.checkpointed.first().map(|base| Swept {
            base: base.clone(),
            discarded: storage::Checkpoint::open(&site.write_root)
                .and_then(|mut checkpoint| checkpoint.wave_post(base))
                .map_err(storage::StorageError::into_ax),
        })
    }

    /// Writes what the model saw beside the room it worked in. The
    /// freeze is already on the ledger, so a room that will not take the
    /// file is noted rather than turned into a failed run: the
    /// transcript is the run's copy, not its record.
    fn keep_transcript(&self, frozen: &runtime::Run<runtime::run::Frozen>, addr: &kernel::Address) {
        let kept = frozen.transcript().and_then(|transcript| {
            let mut cas = held(&self.store, "take the lanes' store")?;
            transcript.materialise(&mut cas, &self.city_root, addr)
        });
        let (level, message) = match kept {
            Ok(record) => (
                runtime::diagnostics::Level::Effect,
                format!(
                    "{} written, {} spans redacted",
                    record.address.as_str(),
                    record.redacted
                ),
            ),
            Err(err) => (
                runtime::diagnostics::Level::Refuse,
                format!("transcript not written: {err}"),
            ),
        };
        self.notes
            .write(level, self.staged_at, "runtime::transcript", &message);
    }
}

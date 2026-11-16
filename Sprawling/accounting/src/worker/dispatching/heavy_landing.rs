// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use kernel::AxError;

use super::super::recording::Notes;
use super::super::workbench::held;
use super::super::{Assignment, Driven, Site};

pub(in crate::worker) struct Swept {
    pub(in crate::worker) base: String,

    pub(in crate::worker) discarded: Result<Vec<kernel::Payload>, AxError>,
}

pub(super) struct HeavyLanding {
    pub(super) city_root: PathBuf,

    pub(super) store: Arc<Mutex<storage::Cas>>,
    pub(super) notes: Notes,
    pub(super) staged_at: kernel::Seq,
}

impl HeavyLanding {
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

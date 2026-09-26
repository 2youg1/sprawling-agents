// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ledger as the person's face of `sprawling view` reads it while it
//! is open (sprawling-SPEC.md 8-91): one pass when the viewer opens, then
//! only the lines appended since the last look.

use std::path::{Path, PathBuf};
use std::time::Duration;

use kernel::{EventRecord, RunId, Seq};
use memory::{LedgerIndex, Refreshed};
use sprawling::lineage::{Lineage, RunLine};

use super::ViewError;

/// How long the viewer waits for a key before it looks at the ledger
/// again; a run that starts in a serving city is on the tree within one
/// tick and one fold.
pub(super) const FOLLOW_TICK: Duration = Duration::from_millis(100);

/// One Ledger line of the `records` lens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    pub(super) seq: Seq,
    pub(super) run: RunId,
    pub(super) line: String,
}

/// A resident index and lineage fold over one city's ledger.
pub(super) struct Follow {
    dir: PathBuf,
    index: LedgerIndex,
    lineage: Lineage,
    folded_to: Option<Seq>,
}

/// The whole lineage after a fold, and the lines that fold took in.
pub(super) type Folded = (Vec<RunLine>, Vec<Row>);

impl Follow {
    /// Reads the whole ledger kept in `dir` once.
    pub(super) fn open(dir: &Path) -> Result<(Follow, Folded), ViewError> {
        let mut follow = Follow {
            dir: dir.to_owned(),
            index: LedgerIndex::rebuild(dir)?,
            lineage: Lineage::default(),
            folded_to: None,
        };
        let folded = follow.fold_appended()?;
        Ok((follow, folded))
    }

    /// The lineage and the new lines when the ledger grew since the last
    /// look; nothing when it did not.
    pub(super) fn poll(&mut self) -> Result<Option<Folded>, ViewError> {
        match self.index.refresh(&self.dir)? {
            Refreshed::Unchanged => Ok(None),
            Refreshed::Appended { .. } | Refreshed::Rebuilt => {
                let folded = self.fold_appended()?;
                Ok((!folded.1.is_empty()).then_some(folded))
            }
        }
    }

    fn fold_appended(&mut self) -> Result<Folded, ViewError> {
        let seqs = self.index.seqs();
        let from = self
            .folded_to
            .map_or(0, |last| seqs.partition_point(|seq| *seq <= last));
        let mut reader = self.index.reader(&self.dir);
        let mut rows = Vec::with_capacity(seqs.len().saturating_sub(from));
        for &seq in seqs.iter().skip(from) {
            let line = reader.line_at(seq)?;
            let record = EventRecord::parse_line(&line)?;
            self.lineage.apply(&record)?;
            rows.push(Row {
                seq,
                run: record.run(),
                // The line just parsed as JSON, which is UTF-8 by
                // definition, so the lossy conversion never replaces a byte.
                line: String::from_utf8_lossy(&line).into_owned(),
            });
        }
        if let Some(last) = rows.last() {
            self.folded_to = Some(last.seq);
        }
        Ok((self.lineage.lines().collect(), rows))
    }
}

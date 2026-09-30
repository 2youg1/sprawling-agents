// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The ledger as the person's face of `sprawling view` reads it while it
//! is open (sprawling-SPEC.md 8-117): the newest lines first, the whole
//! fold on a background thread, then only the lines appended since the
//! last look.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Duration;

use accounting::lineage::{Lineage, RunLine};
use kernel::{EventRecord, RunId, Seq};
use storage::{CheckedLine, LedgerIndex, Refreshed, TailLines};

use super::ViewError;

/// How long the viewer waits for a key before it looks at the ledger
/// again; a run that starts in a serving city is on the tree within one
/// tick and one fold.
pub(super) const FOLLOW_TICK: Duration = Duration::from_millis(100);

/// How many of the newest lines the first screen is drawn from.
pub(super) const FIRST_WINDOW_LINES: usize = 1000;

/// One Ledger line of the `records` lens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    pub(super) seq: Seq,
    pub(super) run: RunId,
    pub(super) line: String,
}

/// The whole lineage after a fold, and the lines that fold took in.
pub(super) type Folded = (Vec<RunLine>, Vec<Row>);

/// What a look at the ledger found.
pub(super) enum Polled {
    /// The whole fold arrived: every line, the window's included.
    Filled(Folded),
    /// The lines appended since the last look.
    Appended(Folded),
}

/// One city's ledger while the viewer is open.
pub(super) struct Follow {
    dir: PathBuf,
    reading: Reading,
}

enum Reading {
    Filling(Receiver<Result<(Box<Whole>, Folded), ViewError>>),
    Whole(Box<Whole>),
}

/// A resident index and lineage fold over the whole ledger.
struct Whole {
    index: LedgerIndex,
    lineage: Lineage,
    folded_to: Option<Seq>,
}

impl Follow {
    /// Reads the newest `FIRST_WINDOW_LINES` lines of the ledger kept in
    /// `dir` and starts the whole fold on a thread of its own.
    pub(super) fn open(dir: &Path) -> Result<(Follow, Folded), ViewError> {
        let window = window_of(dir)?;
        let (answer, answered) = channel();
        let whole_dir = dir.to_owned();
        std::thread::Builder::new()
            .name("view-fill".to_owned())
            .spawn(move || answer.send(Whole::fold(&whole_dir)))?;
        let follow = Follow {
            dir: dir.to_owned(),
            reading: Reading::Filling(answered),
        };
        Ok((follow, window))
    }

    /// The whole fold once it arrived, then the lineage and the new lines
    /// when the ledger grew since the last look; nothing otherwise.
    pub(super) fn poll(&mut self) -> Result<Option<Polled>, ViewError> {
        match &mut self.reading {
            Reading::Filling(answered) => match answered.try_recv() {
                Ok(folded) => {
                    let (whole, folded) = folded?;
                    self.reading = Reading::Whole(whole);
                    Ok(Some(Polled::Filled(folded)))
                }
                Err(TryRecvError::Empty) => Ok(None),
                Err(TryRecvError::Disconnected) => Err(ViewError::Write(std::io::Error::other(
                    "the thread folding the whole ledger ended without an answer",
                ))),
            },
            Reading::Whole(whole) => match whole.index.refresh(&self.dir)? {
                Refreshed::Unchanged => Ok(None),
                Refreshed::Appended { .. } | Refreshed::Rebuilt => {
                    let folded = whole.fold_appended(&self.dir)?;
                    Ok((!folded.1.is_empty()).then_some(Polled::Appended(folded)))
                }
            },
        }
    }
}

/// The lineage and the lines of the newest `FIRST_WINDOW_LINES` lines,
/// oldest first. A run whose `run_started` lies before them has no address.
fn window_of(dir: &Path) -> Result<Folded, ViewError> {
    let mut newest = Vec::with_capacity(FIRST_WINDOW_LINES);
    for tail in TailLines::at(dir)?.take(FIRST_WINDOW_LINES) {
        let tail = tail?;
        if let CheckedLine::Known(record) = tail.checked {
            newest.push((record, tail.raw));
        }
    }
    let mut lineage = Lineage::default();
    let mut rows = Vec::with_capacity(newest.len());
    for (record, raw) in newest.into_iter().rev() {
        lineage.apply(&record)?;
        rows.push(row_of(&record, &raw));
    }
    Ok((lineage.lines().collect(), rows))
}

fn row_of(record: &EventRecord, raw: &[u8]) -> Row {
    Row {
        seq: record.seq(),
        run: record.run(),
        // The line parsed as JSON, which is UTF-8 by definition, so the
        // lossy conversion never replaces a byte.
        line: String::from_utf8_lossy(raw).into_owned(),
    }
}

impl Whole {
    fn fold(dir: &Path) -> Result<(Box<Whole>, Folded), ViewError> {
        let mut whole = Box::new(Whole {
            index: LedgerIndex::rebuild(dir)?,
            lineage: Lineage::default(),
            folded_to: None,
        });
        let folded = whole.fold_appended(dir)?;
        Ok((whole, folded))
    }

    fn fold_appended(&mut self, dir: &Path) -> Result<Folded, ViewError> {
        // Walked from the tail, because what grew between two keys is a
        // few lines at the end of a ledger of any length.
        let folded_to = self.folded_to;
        let unfolded: Vec<Seq> = self
            .index
            .seqs()
            .rev()
            .take_while(|seq| folded_to.is_none_or(|last| *seq > last))
            .collect();
        let mut reader = self.index.reader(dir);
        let mut rows = Vec::with_capacity(unfolded.len());
        for &seq in unfolded.iter().rev() {
            let line = reader.line_at(seq)?;
            let record = EventRecord::parse_line(&line)?;
            self.lineage.apply(&record)?;
            rows.push(row_of(&record, &line));
        }
        if let Some(last) = rows.last() {
            self.folded_to = Some(last.seq);
        }
        Ok((self.lineage.lines().collect(), rows))
    }
}

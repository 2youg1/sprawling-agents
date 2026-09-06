// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Projection views: open, apply, read.

use std::path::Path;

use kernel::{EventRecord, Seq, TimeMs};
use redb::{ReadableDatabase, ReadableTable};
use serde_json::Value;

use crate::error::MemoryError;

use super::tables::{
    LAST_APPLIED, META, ProjectionOpenReport, RECYCLE, RUNS, RecycleEntry, RunRow, ViewRebuilt,
    db_err, fold_record, read_last_applied,
};

pub struct Projection {
    pub(crate) db: redb::Database,
    pub(crate) last_applied: Option<Seq>,
}

impl Projection {
    /// Always hands back a usable view.
    ///
    /// A stored view that cannot be read is removed and started again,
    /// because this view is derived and its recorded recovery is to
    /// delete the file and replay. The caller needs no new branch for
    /// that case: the fresh view reports `last_applied() == None`, which
    /// it must already handle as an ordinary first run.
    ///
    /// The removal is attempted once. A file that is merely corrupt heals
    /// on the second open; a directory that cannot be written fails the
    /// second open too and reports, so no error variant has to be told
    /// apart from another.
    pub fn open(path: &Path) -> Result<(Projection, ProjectionOpenReport), MemoryError> {
        let unreadable = match Self::open_once(path) {
            Ok(projection) => {
                return Ok((projection, ProjectionOpenReport { rebuilt: None }));
            }
            Err(failure) => failure.to_string(),
        };
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(db_err("remove unreadable projection")(err.to_string()));
            }
        }
        let projection = Self::open_once(path)?;
        Ok((
            projection,
            ProjectionOpenReport {
                rebuilt: Some(ViewRebuilt { reason: unreadable }),
            },
        ))
    }

    fn open_once(path: &Path) -> Result<Projection, MemoryError> {
        let db =
            redb::Database::create(path).map_err(|e| db_err("open projection")(e.to_string()))?;
        // Creating the tables at open time means every later read finds
        // them, so "empty" and "absent" never need distinguishing.
        let txn = db
            .begin_write()
            .map_err(|e| db_err("begin projection write")(e.to_string()))?;
        {
            txn.open_table(META)
                .map_err(|e| db_err("open meta table")(e.to_string()))?;
            txn.open_table(RUNS)
                .map_err(|e| db_err("open runs table")(e.to_string()))?;
            txn.open_table(RECYCLE)
                .map_err(|e| db_err("open recycle table")(e.to_string()))?;
        }
        txn.commit()
            .map_err(|e| db_err("commit projection open")(e.to_string()))?;
        let last_applied = read_last_applied(&db)?;
        Ok(Projection { db, last_applied })
    }

    /// Folds one record in, inside one transaction. Idempotent by seq:
    /// a record at or below `last_applied` is a no-op, so replaying a
    /// segment cannot double-count.
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), MemoryError> {
        self.apply_all(std::iter::once(record))
    }

    /// Folds a batch inside one transaction - one durability barrier for
    /// the whole fold, mirroring the ledger's own group commit. This is
    /// the rebuild path: per-record transactions put a disk barrier under
    /// every event and rebuilt at about a thousand records a second; one
    /// transaction per batch rebuilds at the budgeted rate. Same fold,
    /// same idempotence; only the barrier moves.
    pub fn apply_all<'a>(
        &mut self,
        records: impl IntoIterator<Item = &'a EventRecord>,
    ) -> Result<(), MemoryError> {
        let mut last = self.last_applied;
        let txn = self
            .db
            .begin_write()
            .map_err(|e| db_err("begin projection write")(e.to_string()))?;
        {
            let mut runs = txn
                .open_table(RUNS)
                .map_err(|e| db_err("open runs table")(e.to_string()))?;
            let mut recycle = txn
                .open_table(RECYCLE)
                .map_err(|e| db_err("open recycle table")(e.to_string()))?;
            let mut folded_any = false;
            for record in records {
                let seq = record.seq();
                if last.is_some_and(|l| seq <= l) {
                    continue;
                }
                fold_record(record, &mut runs, &mut recycle)?;
                last = Some(seq);
                folded_any = true;
            }
            if !folded_any {
                // Nothing new: leave the store byte-identical rather than
                // committing an empty transaction.
                drop(runs);
                drop(recycle);
                txn.abort()
                    .map_err(|e| db_err("abort empty projection write")(e.to_string()))?;
                return Ok(());
            }
            let mut meta = txn
                .open_table(META)
                .map_err(|e| db_err("open meta table")(e.to_string()))?;
            if let Some(seq) = last {
                meta.insert(LAST_APPLIED, seq.value())
                    .map_err(|e| db_err("write last_applied")(e.to_string()))?;
            }
        }
        txn.commit()
            .map_err(|e| db_err("commit projection apply")(e.to_string()))?;
        self.last_applied = last;
        Ok(())
    }

    /// The Recycle Bin, seq-ordered. Restored entries stay listed with
    /// `restored: true` — the bin is a history, not a pending queue.
    pub fn recycle_bin(&self) -> Result<Vec<RecycleEntry>, MemoryError> {
        let txn = self
            .db
            .begin_read()
            .map_err(|e| db_err("begin projection read")(e.to_string()))?;
        let table = txn
            .open_table(RECYCLE)
            .map_err(|e| db_err("open recycle table")(e.to_string()))?;
        let mut out = Vec::new();
        let iter = table
            .iter()
            .map_err(|e| db_err("scan recycle table")(e.to_string()))?;
        for row in iter {
            let (key, value) = row.map_err(|e| db_err("read recycle row")(e.to_string()))?;
            let parsed: Value = serde_json::from_str(value.value())
                .map_err(|e| db_err("parse recycle row")(e.to_string()))?;
            out.push(RecycleEntry {
                seq: Seq::new(key.value()),
                t: TimeMs::new(parsed.get("t").and_then(Value::as_u64).unwrap_or(0)),
                paths: parsed
                    .get("paths")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|v| v.as_str().map(str::to_owned))
                            .collect()
                    })
                    .unwrap_or_default(),
                restoration: parsed
                    .get("restoration")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned(),
                restored: parsed
                    .get("restored")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });
        }
        Ok(out)
    }

    /// The progress view, RunId-ordered.
    pub fn run_rows(&self) -> Result<Vec<RunRow>, MemoryError> {
        let txn = self
            .db
            .begin_read()
            .map_err(|e| db_err("begin projection read")(e.to_string()))?;
        let table = txn
            .open_table(RUNS)
            .map_err(|e| db_err("open runs table")(e.to_string()))?;
        let mut out = Vec::new();
        let iter = table
            .iter()
            .map_err(|e| db_err("scan runs table")(e.to_string()))?;
        for row in iter {
            let (key, value) = row.map_err(|e| db_err("read run row")(e.to_string()))?;
            let parsed: Value = serde_json::from_str(value.value())
                .map_err(|e| db_err("parse run row")(e.to_string()))?;
            out.push(RunRow {
                run: key.value().to_owned(),
                started_t: TimeMs::new(
                    parsed.get("started_t").and_then(Value::as_u64).unwrap_or(0),
                ),
                frozen: parsed
                    .get("frozen")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            });
        }
        Ok(out)
    }

    pub fn last_applied(&self) -> Option<Seq> {
        self.last_applied
    }

    /// The logical content, table by table, key-ordered, one line per
    /// row. Two projections built from the same ledger export the same
    /// bytes — that is the rebuild guarantee, stated where it holds.
    pub fn export_canonical(&self) -> Result<Vec<u8>, MemoryError> {
        let mut out = String::new();
        out.push_str(&format!(
            "meta {LAST_APPLIED} {}\n",
            self.last_applied.map(|s| s.value()).unwrap_or(0)
        ));
        for row in self.run_rows()? {
            out.push_str(&format!(
                "runs {} {} {}\n",
                row.run,
                row.started_t.value(),
                row.frozen.unwrap_or_else(|| "-".to_owned())
            ));
        }
        for entry in self.recycle_bin()? {
            out.push_str(&format!(
                "recycle {} {} {} {} {}\n",
                entry.seq.value(),
                entry.t.value(),
                entry.paths.join(","),
                entry.restoration,
                entry.restored
            ));
        }
        Ok(out.into_bytes())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;

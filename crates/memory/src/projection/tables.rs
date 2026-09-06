// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Projection tables: rows, folds, and the table grammar.

use kernel::{EventKind, EventRecord, Seq, TimeMs};
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use serde_json::Value;

use crate::error::MemoryError;

pub(crate) const META: TableDefinition<&str, u64> = TableDefinition::new("meta");
pub(crate) const RUNS: TableDefinition<&str, &str> = TableDefinition::new("runs");
pub(crate) const RECYCLE: TableDefinition<u64, &str> = TableDefinition::new("recycle");
pub(crate) const LAST_APPLIED: &str = "last_applied";

/// One row of the Recycle Bin: what was discarded, and whether it has
/// since come back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecycleEntry {
    pub seq: Seq,
    pub t: TimeMs,
    pub paths: Vec<String>,
    pub restoration: String,
    pub restored: bool,
}

/// One row of the progress view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRow {
    pub run: String,
    pub started_t: TimeMs,
    pub frozen: Option<String>,
}

/// What open found and repaired.
pub struct ProjectionOpenReport {
    pub rebuilt: Option<ViewRebuilt>,
}

/// The stored view could not be read, so open removed it and started an
/// empty one. `last_applied` is `None` in consequence, and that is the
/// whole instruction to the caller: replay from the start.
pub struct ViewRebuilt {
    /// What the store said before the file went. Removing the file
    /// destroys the only other copy of this sentence, and a view that
    /// resets without saying why teaches nobody anything.
    pub reason: String,
}

pub(crate) fn db_err(op: &'static str) -> impl FnOnce(String) -> MemoryError {
    move |detail| MemoryError::Projection {
        op,
        detail: detail.to_string(),
    }
}

/// One record into the open tables. Shared by the single-record and the
/// batched fold so "what a record means" has exactly one definition.
pub(crate) fn fold_record(
    record: &EventRecord,
    runs: &mut redb::Table<'_, &'static str, &'static str>,
    recycle: &mut redb::Table<'_, u64, &'static str>,
) -> Result<(), MemoryError> {
    let seq = record.seq();
    match record.kind() {
        EventKind::RunStarted => {
            let row = serde_json::json!({
                "started_t": record.t().value(),
                "frozen": Value::Null,
            })
            .to_string();
            runs.insert(record.run().to_string().as_str(), row.as_str())
                .map_err(|e| db_err("insert run row")(e.to_string()))?;
        }
        EventKind::RunFrozen => {
            let key = record.run().to_string();
            let started = runs
                .get(key.as_str())
                .map_err(|e| db_err("read run row")(e.to_string()))?
                .and_then(|guard| {
                    serde_json::from_str::<Value>(guard.value())
                        .ok()?
                        .get("started_t")?
                        .as_u64()
                })
                .unwrap_or_else(|| record.t().value());
            let completion = record
                .data()
                .as_map()
                .get("completion")
                .and_then(Value::as_str)
                .unwrap_or("unspecified")
                .to_owned();
            let row = serde_json::json!({
                "started_t": started,
                "frozen": completion,
            })
            .to_string();
            runs.insert(key.as_str(), row.as_str())
                .map_err(|e| db_err("insert run row")(e.to_string()))?;
        }
        EventKind::FileDiscarded => {
            let data = record.data().as_map();
            let paths: Vec<String> = data
                .get("paths")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let row = serde_json::json!({
                "t": record.t().value(),
                "paths": paths,
                "restoration": restoration_label(data.get("restoration")),
                "restored": false,
            })
            .to_string();
            recycle
                .insert(seq.value(), row.as_str())
                .map_err(|e| db_err("insert recycle row")(e.to_string()))?;
        }
        EventKind::DiscardRestored => {
            // The restore names the discard it undoes; a restore with no
            // matching discard is dropped, not invented.
            if let Some(target) = record
                .data()
                .as_map()
                .get("discard_seq")
                .and_then(Value::as_u64)
            {
                let existing = recycle
                    .get(target)
                    .map_err(|e| db_err("read recycle row")(e.to_string()))?
                    .map(|guard| guard.value().to_owned());
                if let Some(raw) = existing
                    && let Ok(mut parsed) = serde_json::from_str::<Value>(&raw)
                    && let Some(map) = parsed.as_object_mut()
                {
                    map.insert("restored".to_owned(), Value::Bool(true));
                    let row = parsed.to_string();
                    recycle
                        .insert(target, row.as_str())
                        .map_err(|e| db_err("insert recycle row")(e.to_string()))?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn restoration_label(value: Option<&Value>) -> String {
    match value {
        Some(Value::Object(map)) => map
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "unknown".to_owned()),
        Some(Value::String(s)) => s.clone(),
        _ => "unknown".to_owned(),
    }
}

pub(crate) fn read_last_applied(db: &redb::Database) -> Result<Option<Seq>, MemoryError> {
    let txn = db
        .begin_read()
        .map_err(|e| db_err("begin projection read")(e.to_string()))?;
    let table = txn
        .open_table(META)
        .map_err(|e| db_err("open meta table")(e.to_string()))?;
    let found = table
        .get(LAST_APPLIED)
        .map_err(|e| db_err("read last_applied")(e.to_string()))?
        .map(|guard| Seq::new(guard.value()));
    Ok(found)
}

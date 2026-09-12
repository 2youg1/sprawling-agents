// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one run was told, read back out of the record that froze it and
//! the store that kept it.
//!
//! **Nothing here assembles a prefix.** `assembly::freezing` does that
//! once, at run start, and puts every segment in the store; this joins
//! the hashes its `prompt_assembled` record names to the bytes behind
//! them. A second assembly taken now would read files that have moved
//! since and show a person a prompt nobody was ever sent.
//!
//! **A missing object is not an empty segment.** `stored` says which of
//! the two a reader is looking at, because the first is a store that
//! was pruned and the second is a slot the city had nothing to put in.

use kernel::{Address, B3Hash, EventKind, EventRecord, Locator, RunId, Seq};

use super::document::read_bytes;
use super::holding::Views;

impl Views {
    /// The four segments one run was frozen with.
    ///
    /// `None` for a run this city never froze a prefix for and for a
    /// store that will not open, which is what `Unavailable` says: a
    /// run that has not reached its first turn has no prompt yet, and
    /// an empty answer would read as a run that was told nothing.
    pub(super) fn prefix_answer(&mut self, run: RunId) -> Option<channels::PrefixAnswer> {
        let record = self.first_prompt(run)?;
        let store = self.store()?;
        let segments = record
            .data()
            .as_map()
            .get("segments")?
            .as_array()?
            .iter()
            .filter_map(|row| segment_of(row, &store))
            .collect();
        Some(channels::PrefixAnswer { run, segments })
    }

    /// One object of the store, cut to what travels.
    ///
    /// `None` for a locator naming a path rather than an object, and
    /// for an object this city no longer holds. Both are the same
    /// answer to the caller - there is nothing here to read - and the
    /// query handed back with the refusal says which was asked.
    pub(super) fn content_answer(&self, locator: &Locator) -> Option<channels::ContentAnswer> {
        let Locator::Cas { hash, range: _ } = locator else {
            return None;
        };
        let read = read_bytes(&self.store()?.get(hash).ok()?);
        Some(channels::ContentAnswer {
            locator: locator.clone(),
            text: read.text,
            bytes: read.bytes,
            truncated: read.truncated,
            binary: read.binary,
        })
    }

    /// The store this city keeps, opened for reading.
    ///
    /// Opened per question rather than held: this is the one read here
    /// that reaches a directory the fold does not own, and a handle
    /// kept across a rebuild would outlive the city it was opened
    /// under.
    fn store(&self) -> Option<memory::Cas> {
        memory::Cas::open(&self.city_root.join(kernel::RESERVED_PREFIX).join("cas")).ok()
    }

    /// The `prompt_assembled` record that opened this run.
    ///
    /// The oldest one, not the newest: the prefix is frozen once for
    /// the life of a run and every later turn records the same four
    /// hashes, so any of them says the same thing and the first is the
    /// one a run that never got past turn one still has.
    fn first_prompt(&mut self, run: RunId) -> Option<EventRecord> {
        let dir = crate::assembly::ledger_dir(&self.city_root);
        self.index.refresh(&dir).ok()?;
        let seqs: Vec<Seq> = self.index.run_seqs_before(run, None).collect();
        let mut reader = self.index.reader(&dir);
        for seq in seqs.into_iter().rev() {
            let Ok(line) = reader.line_at(seq) else {
                continue;
            };
            let Ok(record) = EventRecord::parse_line(&line) else {
                continue;
            };
            if record.kind() == EventKind::PromptAssembled {
                return Some(record);
            }
        }
        None
    }
}

/// One recorded segment row, joined to the bytes the store holds for it.
///
/// A row whose slot or hash will not read is left out rather than
/// guessed at: a fifth spelling of a slot is one nothing draws.
fn segment_of(row: &serde_json::Value, store: &memory::Cas) -> Option<channels::PrefixSegment> {
    let slot = match row.get("slot")?.as_str()? {
        "city" => channels::PrefixSlot::City,
        "building" => channels::PrefixSlot::Building,
        "resident" => channels::PrefixSlot::Resident,
        "run" => channels::PrefixSlot::Run,
        _ => return None,
    };
    let hash: B3Hash = serde_json::from_value(row.get("hash")?.clone()).ok()?;
    let held = store.get(&hash).ok();
    Some(channels::PrefixSegment {
        slot,
        hash,
        bytes: number(row, "len"),
        text: held
            .as_ref()
            .map_or_else(String::new, |bytes| read_bytes(bytes).text),
        stored: held.is_some(),
        sources: row
            .get("sources")
            .and_then(serde_json::Value::as_array)
            .map(|rows| rows.iter().filter_map(source_of).collect())
            .unwrap_or_default(),
    })
}

/// One source row, as the assembler wrote it down. A row with no
/// address names no document and is left out.
fn source_of(row: &serde_json::Value) -> Option<channels::PrefixSource> {
    Some(channels::PrefixSource {
        addr: Address::parse(row.get("addr")?.as_str()?).ok()?,
        kept: number(row, "kept"),
        dropped: number(row, "dropped"),
    })
}

/// One recorded count. A key the record does not carry reads as zero,
/// which is what a payload written before that key existed means.
fn number(row: &serde_json::Value, key: &str) -> u64 {
    row.get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

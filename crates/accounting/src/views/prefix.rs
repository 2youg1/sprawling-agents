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

use std::path::Path;

use kernel::{Address, B3Hash, EventKind, EventRecord, Locator, RunId, Seq};

use super::document::read_bytes;
use super::holding::Views;
use super::prepared::{LedgerAsk, unavailable};

/// What a `Prefix` question takes out of the views: where the run's
/// first prompt sits on the ledger, and the ledger that holds the line.
/// The line and the store are read by [`PrefixAsk::read`], after the
/// views are released.
pub(crate) struct PrefixAsk {
    ledger: LedgerAsk,
    run: RunId,
    first: Option<Seq>,
}

impl Views {
    pub(super) fn prefix_ask(&self, run: RunId) -> PrefixAsk {
        PrefixAsk {
            ledger: self.ledger_ask(),
            run,
            first: self.first_prompts.get(&run).copied(),
        }
    }
}

impl PrefixAsk {
    /// The four segments the run was frozen with.
    ///
    /// `Unavailable` for a run this city never froze a prefix for and
    /// for a ledger or store that will not open: a run that has not
    /// reached its first turn has no prompt yet, and an empty answer
    /// would read as a run that was told nothing.
    pub(super) fn read(self) -> wire::Answer {
        match self.segments() {
            Some(segments) => wire::Answer::Prefix(Box::new(wire::PrefixAnswer {
                run: self.run,
                segments,
            })),
            None => unavailable(format!("Prefix({})", self.run)),
        }
    }

    fn segments(&self) -> Option<Vec<wire::PrefixSegment>> {
        let record = self.first_prompt()?;
        let store = store(&self.ledger.city_root)?;
        Some(
            record
                .data()
                .as_map()
                .get("segments")?
                .as_array()?
                .iter()
                .filter_map(|row| segment_of(row, &store))
                .collect(),
        )
    }

    /// The `prompt_assembled` record that opened this run.
    ///
    /// The oldest one, not the newest: the prefix is frozen once for
    /// the life of a run and every later turn records the same four
    /// hashes, so any of them says the same thing and the first is the
    /// one a run that never got past turn one still has.
    fn first_prompt(&self) -> Option<EventRecord> {
        let seq = self.first?;
        let line = {
            let (index, dir) = self.ledger.indexed()?;
            index.reader(&dir).line_at(seq).ok()?
        };
        EventRecord::parse_line(&line)
            .ok()
            .filter(|record| record.kind() == EventKind::PromptAssembled)
    }
}

/// One object of the store, cut to what travels.
///
/// `None` for a locator naming a path rather than an object, and
/// for an object this city no longer holds. Both are the same
/// answer to the caller - there is nothing here to read - and the
/// query handed back with the refusal says which was asked.
pub(super) fn content_answer(city_root: &Path, locator: &Locator) -> Option<wire::ContentAnswer> {
    let Locator::Cas { hash, range: _ } = locator else {
        return None;
    };
    let read = read_bytes(&store(city_root)?.get(hash).ok()?);
    Some(wire::ContentAnswer {
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
fn store(city_root: &Path) -> Option<storage::Cas> {
    storage::Cas::open(&kernel::layout::CityLayout::new(city_root).cas()).ok()
}

/// One recorded segment row, joined to the bytes the store holds for it.
///
/// A row whose slot or hash will not read is left out rather than
/// guessed at: a fifth spelling of a slot is one nothing draws.
fn segment_of(row: &serde_json::Value, store: &storage::Cas) -> Option<wire::PrefixSegment> {
    let slot = match row.get("slot")?.as_str()? {
        "city" => wire::PrefixSlot::City,
        "building" => wire::PrefixSlot::Building,
        "resident" => wire::PrefixSlot::Resident,
        "run" => wire::PrefixSlot::Run,
        _ => return None,
    };
    let hash: B3Hash = serde_json::from_value(row.get("hash")?.clone()).ok()?;
    let held = store.get(&hash).ok();
    Some(wire::PrefixSegment {
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
fn source_of(row: &serde_json::Value) -> Option<wire::PrefixSource> {
    Some(wire::PrefixSource {
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

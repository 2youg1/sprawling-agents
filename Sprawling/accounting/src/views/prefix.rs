// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::event::record::PromptTools;
use kernel::{Address, B3Hash, EventKind, EventRecord, Locator, RunId, Seq};

use super::document::read_bytes;
use super::holding::Views;
use super::prepared::{LedgerAsk, unavailable, unavailable_because};

pub struct PrefixAsk {
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

    pub(super) fn prompt_shape_ask(&self, run: RunId) -> PromptShapeAsk {
        PromptShapeAsk {
            ledger: self.ledger_ask(),
            run,
            latest: self.latest_shapes.get(&run).copied(),
        }
    }
}

pub struct PromptShapeAsk {
    ledger: LedgerAsk,
    run: RunId,
    latest: Option<Seq>,
}

impl PromptShapeAsk {
    pub(super) fn read(self) -> wire::Answer {
        let Some(seq) = self.latest else {
            return unavailable(format!("PromptShape({})", self.run));
        };
        match self.shape_at(seq) {
            Ok(answer) => wire::Answer::PromptShape(Box::new(answer)),
            Err(stopped) => unavailable_because(format!("PromptShape({})", self.run), &stopped),
        }
    }

    fn shape_at(&self, seq: Seq) -> Result<wire::PromptShapeAnswer, kernel::AxError> {
        let (index, dir) = self
            .ledger
            .indexed()
            .map_err(storage::StorageError::into_ax)?;
        let line = index
            .reader(&dir)
            .line_at(seq)
            .map_err(storage::StorageError::into_ax)?;
        let record = EventRecord::parse_line(&line)?;
        if record.kind() != EventKind::PromptShapeCompared {
            return Err(kernel::AxError::failure(
                kernel::AxCode::InvalidArgs,
                "read PromptShapeCompared",
                format!("seq {}", seq.value()),
            )
            .with_recovery("the indexed line is not a prompt_shape_compared record"));
        }
        let compared: kernel::event::record::PromptShapeCompared = record.data().read()?;
        Ok(wire::PromptShapeAnswer {
            run: self.run,
            tools: compared.tools,
            conversation: compared.run,
            upper_bound: compared.upper_bound,
        })
    }
}

impl PrefixAsk {
    pub(super) fn read(self) -> wire::Answer {
        match self.segments() {
            Some((segments, tools)) => wire::Answer::Prefix(Box::new(wire::PrefixAnswer {
                run: self.run,
                segments,
                tools,
            })),
            None => unavailable(format!("Prefix({})", self.run)),
        }
    }

    fn segments(&self) -> Option<(Vec<wire::PrefixSegment>, Option<PromptTools>)> {
        let record = self.first_prompt()?;
        let store = store(&self.ledger.city_root)?;
        let data = record.data().as_map();
        let segments = data
            .get("segments")?
            .as_array()?
            .iter()
            .filter_map(|row| segment_of(row, &store))
            .collect();

        let tools = data
            .get("tools")
            .and_then(|table| serde_json::from_value(table.clone()).ok());
        Some((segments, tools))
    }

    fn first_prompt(&self) -> Option<EventRecord> {
        let seq = self.first?;
        let line = {
            let (index, dir) = self.ledger.indexed().ok()?;
            index.reader(&dir).line_at(seq).ok()?
        };
        EventRecord::parse_line(&line)
            .ok()
            .filter(|record| record.kind() == EventKind::PromptAssembled)
    }
}

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

fn store(city_root: &Path) -> Option<storage::Cas> {
    storage::Cas::open(&kernel::layout::CityLayout::new(city_root).cas()).ok()
}

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

fn source_of(row: &serde_json::Value) -> Option<wire::PrefixSource> {
    Some(wire::PrefixSource {
        addr: Address::parse(row.get("addr")?.as_str()?).ok()?,
        kept: number(row, "kept"),
        dropped: number(row, "dropped"),
    })
}

fn number(row: &serde_json::Value, key: &str) -> u64 {
    row.get(key)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

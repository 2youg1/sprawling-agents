// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Frozen prefix assembly: four segments, assembled
//! once at Run start, never reassembled within the Run. Volatile types
//! (TimeMs, usage, signals) have no conversion into `FrozenSegment` — the
//! absence of those impls is the isolation guarantee (15.3-4).

use std::collections::BTreeSet;

use kernel::consts_policy::STARTUP_BUDGET_TOKENS;
use kernel::{Address, AxCode, AxError, B3Hash, Payload, SystemBlock};
use serde_json::{Map, Value, json};

mod segment;

pub use segment::{FrozenSegment, SegmentSlot, SegmentSource};

/// Documents inside one segment join on this separator; the rebuilder
/// (replay) reuses it — one authority for the concatenation rule.
pub(crate) const DOC_JOIN: &str = "\n\n";

/// The in-place truncation marker. English on purpose — the prefix faces the English window.
pub(crate) fn truncation_marker(dropped: u64) -> String {
    format!("[truncated: {dropped} bytes]")
}

/// One prefix source document: address plus its frozen bytes, `None`
/// when missing or unreadable (the skip itself is accounted).
#[derive(Debug, Clone)]
pub struct SourceDoc {
    pub addr: Address,
    pub bytes: Option<Vec<u8>>,
}

/// Per-slot byte budgets. Callers derive them from config; the default
/// splits the startup budget evenly (tokens ≈ bytes/4, four slots).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentCaps {
    pub city: u64,
    pub building: u64,
    pub resident: u64,
    pub run: u64,
}

impl SegmentCaps {
    pub fn startup_default() -> SegmentCaps {
        // STARTUP_BUDGET_TOKENS tokens * 4 bytes/token / 4 slots.
        let per_slot = STARTUP_BUDGET_TOKENS;
        SegmentCaps {
            city: per_slot,
            building: per_slot,
            resident: per_slot,
            run: per_slot,
        }
    }
}

/// The four source lists plus caps: everything `build_prefix` needs.
#[derive(Debug, Clone)]
pub struct PrefixPlan {
    pub city: Vec<SourceDoc>,
    pub building: Vec<SourceDoc>,
    pub resident: Vec<SourceDoc>,
    pub run: Vec<SourceDoc>,
    pub caps: SegmentCaps,
}

/// Builds the four segments from source documents: UTF-8 only, per-slot
/// caps with explicit truncation markers, cross-slot dedup by address
/// (first slot wins), skips accounted in the notes. The notes travel
/// inside the returned prefix and surface in `prompt_assembled` — they
/// are what makes the prefix offline-rebuildable (A15, C16).
pub fn build_prefix(plan: PrefixPlan) -> Result<FrozenPrefix, AxError> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut skipped = Vec::new();
    let mut built = Vec::new();
    let slots = [
        (SegmentSlot::City, &plan.city, plan.caps.city),
        (SegmentSlot::Building, &plan.building, plan.caps.building),
        (SegmentSlot::Resident, &plan.resident, plan.caps.resident),
        (SegmentSlot::Run, &plan.run, plan.caps.run),
    ];
    for (slot, docs, cap) in slots {
        let (segment, note) = build_segment(slot, docs, cap, &mut seen)?;
        skipped.push(note);
        built.push(segment);
    }
    let mut iter = built.into_iter();
    let (Some(city), Some(building), Some(resident), Some(run)) =
        (iter.next(), iter.next(), iter.next(), iter.next())
    else {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "build prefix",
            "segment construction lost a slot",
        ));
    };
    let mut prefix = FrozenPrefix::assemble(city, building, resident, run)?;
    prefix.skipped = Some(skipped);
    Ok(prefix)
}

fn floor_char_boundary(s: &str, max: usize) -> usize {
    if max >= s.len() {
        return s.len();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    end
}

#[expect(
    clippy::arithmetic_side_effects,
    reason = "budget arithmetic on usize values bounded by document sizes already held \
              in memory; every subtraction is guarded by a preceding comparison"
)]
fn build_segment(
    slot: SegmentSlot,
    docs: &[SourceDoc],
    cap: u64,
    seen: &mut BTreeSet<String>,
) -> Result<(FrozenSegment, Vec<Value>), AxError> {
    let cap = usize::try_from(cap).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "build prefix segment",
            "cap exceeds usize",
        )
    })?;
    let mut text = String::new();
    let mut sources = Vec::new();
    let mut skipped = Vec::new();
    for doc in docs {
        let addr = doc.addr.as_str().to_owned();
        if seen.contains(&addr) {
            skipped.push(json!({ "addr": addr, "reason": "duplicate" }));
            continue;
        }
        let Some(bytes) = &doc.bytes else {
            skipped.push(json!({ "addr": addr, "reason": "unreadable" }));
            continue;
        };
        let Ok(body) = std::str::from_utf8(bytes) else {
            skipped.push(json!({ "addr": addr, "reason": "not_utf8" }));
            continue;
        };
        let joiner = if text.is_empty() { 0 } else { DOC_JOIN.len() };
        let used = text.len() + joiner;
        let remaining = cap.saturating_sub(used);
        if body.len() <= remaining {
            if joiner > 0 {
                text.push_str(DOC_JOIN);
            }
            text.push_str(body);
            seen.insert(addr.clone());
            sources.push(SegmentSource::whole(
                doc.addr.clone(),
                u64::try_from(body.len()).unwrap_or(u64::MAX),
            ));
            continue;
        }
        // Reserve marker room with the worst-case digit count (dropped
        // can only shrink once bytes are kept), then floor to a char
        // boundary so the segment stays valid UTF-8.
        let worst_marker = truncation_marker(u64::try_from(body.len()).unwrap_or(u64::MAX)).len();
        if remaining <= worst_marker {
            skipped.push(json!({ "addr": addr, "reason": "no_budget" }));
            continue;
        }
        let kept = floor_char_boundary(body, remaining - worst_marker);
        let dropped = u64::try_from(body.len() - kept).map_err(|_| {
            AxError::failure(
                AxCode::InvalidArgs,
                "build prefix segment",
                "length overflow",
            )
        })?;
        if joiner > 0 {
            text.push_str(DOC_JOIN);
        }
        text.push_str(body.get(..kept).unwrap_or_default());
        text.push_str(&truncation_marker(dropped));
        seen.insert(addr.clone());
        sources.push(SegmentSource {
            addr: doc.addr.clone(),
            kept: u64::try_from(kept).unwrap_or(u64::MAX),
            dropped,
        });
    }
    Ok((
        FrozenSegment::assembled(slot, text.into_bytes(), sources),
        skipped,
    ))
}

/// The assembled prefix: city, building, resident, run — in that order,
/// enforced at the only constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenPrefix {
    city: FrozenSegment,
    building: FrozenSegment,
    resident: FrozenSegment,
    run: FrozenSegment,
    /// What each slot left out, in slot order, as `build_prefix` found
    /// it. `None` is a prefix assembled from bytes the caller already
    /// held, which skipped nothing because it was never offered a list
    /// of documents.
    skipped: Option<Vec<Vec<Value>>>,
}

impl FrozenPrefix {
    /// Slot order is the type: a segment in the wrong position is
    /// `E_INVALID_ARGS` (fail-closed, never silently reordered).
    pub fn assemble(
        city: FrozenSegment,
        building: FrozenSegment,
        resident: FrozenSegment,
        run: FrozenSegment,
    ) -> Result<FrozenPrefix, AxError> {
        let expected = [
            (SegmentSlot::City, city.slot()),
            (SegmentSlot::Building, building.slot()),
            (SegmentSlot::Resident, resident.slot()),
            (SegmentSlot::Run, run.slot()),
        ];
        for (want, got) in expected {
            if want != got {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "assemble frozen prefix",
                    format!("slot {} holds a {} segment", want.as_str(), got.as_str()),
                )
                .with_recovery("pass segments in slot order: city, building, resident, run"));
            }
        }
        Ok(FrozenPrefix {
            city,
            building,
            resident,
            run,
            skipped: None,
        })
    }

    /// The wire form of the frozen prefix: four system blocks, every one
    /// an explicit cache breakpoint — exactly `CACHE_BREAKPOINTS_MAX`.
    /// Segments must be UTF-8 (build_prefix guarantees it; hand-built
    /// test prefixes must comply to reach the wire).
    pub fn system_blocks(&self) -> Result<Vec<SystemBlock>, AxError> {
        self.segments()
            .iter()
            .map(|segment| {
                let text = std::str::from_utf8(segment.bytes())
                    .map_err(|_| {
                        AxError::failure(
                            AxCode::InvalidArgs,
                            "render system blocks",
                            format!("{} segment is not utf-8", segment.slot().as_str()),
                        )
                    })?
                    .to_owned();
                Ok(SystemBlock { text, cache: true })
            })
            .collect()
    }

    pub fn segments(&self) -> [&FrozenSegment; 4] {
        [&self.city, &self.building, &self.resident, &self.run]
    }

    pub fn segment_hashes(&self) -> [B3Hash; 4] {
        [
            *self.city.hash(),
            *self.building.hash(),
            *self.resident.hash(),
            *self.run.hash(),
        ]
    }

    /// The `prompt_assembled` payload: four rows, each naming its slot,
    /// its hash, its length, the documents it was assembled from and
    /// what it left out.
    ///
    /// One branch, not two. The source rows are read back by an offline
    /// rebuild (A15, C16) and by the page that shows a person what
    /// their agent was told, and they come from the segments themselves
    /// — so a prefix assembled at the city's desk records the same rows
    /// under the same keys as one built from a plan, and a segment made
    /// of no documents says so with an empty list.
    ///
    /// # Errors
    /// A segment longer than `u64` can hold, and a payload the Ledger
    /// refuses.
    pub fn prompt_payload(&self) -> Result<Payload, AxError> {
        let mut segments = Vec::new();
        for (index, segment) in self.segments().into_iter().enumerate() {
            let mut entry = Map::new();
            entry.insert(
                "slot".to_owned(),
                Value::String(segment.slot().as_str().to_owned()),
            );
            entry.insert("hash".to_owned(), Value::String(segment.hash().to_string()));
            let len = u64::try_from(segment.bytes().len()).map_err(|_| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "encode prompt payload",
                    "segment length exceeds u64",
                )
            })?;
            entry.insert("len".to_owned(), Value::Number(len.into()));
            entry.insert(
                "sources".to_owned(),
                Value::Array(segment.sources().iter().map(SegmentSource::row).collect()),
            );
            let left_out = self
                .skipped
                .as_ref()
                .and_then(|all| all.get(index))
                .cloned()
                .unwrap_or_default();
            entry.insert("skipped".to_owned(), Value::Array(left_out));
            segments.push(Value::Object(entry));
        }
        let mut map = Map::new();
        map.insert("segments".to_owned(), Value::Array(segments));
        map.insert(
            "breakpoints".to_owned(),
            json!(["city", "building", "resident", "run"]),
        );
        Payload::new(map)
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

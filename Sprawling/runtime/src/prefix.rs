// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use kernel::consts_policy::{BYTES_PER_TOKEN, PREFIX_SLOTS, STARTUP_BUDGET_TOKENS};
use kernel::event::record::{PromptAssembled, PromptSegment, PromptSkip, PromptTools, SkipReason};
use kernel::{Address, AxCode, AxError, B3Hash, Payload, SystemBlock};

use crate::elision::{self, Elided};

mod breakpoint;
mod segment;

pub(crate) mod shape;
pub mod warmth;

pub use breakpoint::{Breakpoint, BreakpointPlan};

pub(crate) use segment::ANOTHER_ADDRESS;
pub use segment::{FrozenSegment, SegmentSlot, SegmentSource};

pub(crate) const DOC_JOIN: &str = "\n\n";

#[derive(Debug, Clone)]
pub struct SourceDoc {
    pub addr: Address,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentCaps {
    pub city: u64,
    pub building: u64,
    pub resident: u64,
    pub run: u64,
}

impl SegmentCaps {
    pub fn startup_default() -> SegmentCaps {
        let per_slot = STARTUP_BUDGET_TOKENS.saturating_mul(BYTES_PER_TOKEN) / PREFIX_SLOTS;
        SegmentCaps {
            city: per_slot,
            building: per_slot,
            resident: per_slot,
            run: per_slot,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrefixPlan {
    pub city: Vec<SourceDoc>,
    pub building: Vec<SourceDoc>,
    pub resident: Vec<SourceDoc>,
    pub run: Vec<SourceDoc>,
    pub caps: SegmentCaps,
}

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
        )
        .with_recovery(
            "report this against runtime::prefix: a prefix is four segments, city, \
             building, resident and run, and fewer than four were built",
        ));
    };
    let mut prefix = FrozenPrefix::assemble(city, building, resident, run)?;
    prefix.skipped = Some(skipped);
    Ok(prefix)
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
) -> Result<(FrozenSegment, Vec<PromptSkip>), AxError> {
    let cap = usize::try_from(cap).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "build prefix segment",
            "cap exceeds usize",
        )
        .with_recovery(
            "lower this slot's byte cap in `[prefix]`: it is larger than this machine \
             can address",
        )
    })?;
    let mut text = String::new();
    let mut sources = Vec::new();
    let mut skipped = Vec::new();
    for doc in docs {
        let addr = doc.addr.as_str().to_owned();
        let left_out = |reason: SkipReason| PromptSkip {
            addr: doc.addr.clone(),
            reason,
        };
        if seen.contains(&addr) {
            skipped.push(left_out(SkipReason::Duplicate));
            continue;
        }
        let Some(bytes) = &doc.bytes else {
            skipped.push(left_out(SkipReason::Unreadable));
            continue;
        };
        let Ok(body) = std::str::from_utf8(bytes) else {
            skipped.push(left_out(SkipReason::NotUtf8));
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
            sources.push(SegmentSource::whole(doc.addr.clone(), bytes));
            continue;
        }
        let worst_marker = elision::marker_room(body.len());
        if remaining <= worst_marker {
            skipped.push(left_out(SkipReason::NoBudget));
            continue;
        }
        let kept = elision::boundary_before(body, remaining.saturating_sub(worst_marker));

        let cut = elision::splice(body, kept, body.len(), Elided::Tail);
        if joiner > 0 {
            text.push_str(DOC_JOIN);
        }
        text.push_str(&cut.text);
        seen.insert(addr.clone());

        let kept_bytes = body.as_bytes().get(..kept).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "build prefix segment",
                format!("{addr}: cut at {kept} lies outside the document"),
            )
            .with_recovery("report this: the elision boundary left the document it was cutting")
        })?;
        sources.push(SegmentSource::cut(
            doc.addr.clone(),
            kept_bytes,
            cut.dropped.get(),
        ));
    }
    Ok((
        FrozenSegment::assembled(slot, text.into_bytes(), sources),
        skipped,
    ))
}

pub use segment::verified_system_hashes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenPrefix {
    city: FrozenSegment,
    building: FrozenSegment,
    resident: FrozenSegment,
    run: FrozenSegment,

    skipped: Option<Vec<Vec<PromptSkip>>>,

    tools: Option<PromptTools>,
}

impl FrozenPrefix {
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
            tools: None,
        })
    }

    #[must_use]
    pub fn with_tools(self, tools: PromptTools) -> FrozenPrefix {
        FrozenPrefix {
            tools: Some(tools),
            ..self
        }
    }

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
                        .with_recovery(format!(
                            "save the documents feeding the {} slot as UTF-8; a model \
                             prompt carries text and nothing else",
                            segment.slot().as_str()
                        ))
                    })?
                    .to_owned();
                Ok(SystemBlock {
                    text,
                    cache: BreakpointPlan::marks_edge(segment.slot()),
                })
            })
            .collect()
    }

    pub fn shared_part(&self) -> kernel::SharedPrefix {
        std::num::NonZeroUsize::new(
            self.segments()
                .iter()
                .take_while(|segment| segment.slot().shared_by_building())
                .count(),
        )
        .map_or(
            kernel::SharedPrefix::Unshared,
            kernel::SharedPrefix::Leading,
        )
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

    pub fn verified_segment_hashes(&self) -> Result<[B3Hash; 4], AxError> {
        let [city, building, resident, run] = self.segments();
        Ok([
            city.verified_hash()?,
            building.verified_hash()?,
            resident.verified_hash()?,
            run.verified_hash()?,
        ])
    }

    pub fn prompt_payload(&self, plan: &BreakpointPlan) -> Result<Payload, AxError> {
        let mut segments = Vec::new();
        for (index, segment) in self.segments().into_iter().enumerate() {
            let len = u64::try_from(segment.bytes().len()).map_err(|_| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "encode prompt payload",
                    "segment length exceeds u64",
                )
                .with_recovery(
                    "lower the slot byte caps in `[prefix]`: one frozen segment is \
                     longer than a byte count this city can hold",
                )
            })?;
            segments.push(PromptSegment {
                slot: segment.slot().as_str().to_owned(),
                hash: *segment.hash(),
                len,
                sources: segment.sources().iter().map(SegmentSource::row).collect(),
                skipped: self
                    .skipped
                    .as_ref()
                    .and_then(|all| all.get(index))
                    .cloned()
                    .unwrap_or_default(),
            });
        }
        Payload::of(&PromptAssembled {
            segments,
            breakpoints: plan
                .breakpoints()
                .into_iter()
                .map(|breakpoint| breakpoint.as_str().to_owned())
                .collect(),
            tools: self.tools.clone(),
        })
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

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the money went. Four independent cuts of
//! one authoritative total: by Run, by actor, by prefix segment, by
//! tool.
//!
//! The invariant that makes the report trustworthy (A20) is that each
//! cut sums to exactly the billed total — not approximately, not after
//! rounding. Percentages cannot promise that, so nothing here is a
//! percentage: shares are split by the largest-remainder method, which
//! distributes the floor divisions and then hands the leftover units
//! out one at a time, in a fixed order. What is billed is what is
//! attributed, to the microdollar.
//!
//! The total comes from `model_returned.billed_usd_micros` and nowhere
//! else. This module never prices a call — that authority is
//! `gateway::cost`, and a second one would be a second answer.

//! Attribution reports: what each wave billed.

use super::split::{add, quantify, segment_weights, split};

/// The bucket a call falls into when the ledger gives no basis for a
/// finer split — an honest "unsplit", never a silent drop.
pub(crate) const NO_TOOL: &str = "no_tool";
pub(crate) const NO_SKILL: &str = "no_skill";
pub(crate) const NO_SEGMENT: &str = "unattributed";
pub(crate) const WINDOW_SLOT: &str = "window";

use std::collections::BTreeMap;

use kernel::{EventKind, EventRecord, UsdMicros};
use serde_json::Value;

use crate::error::MemoryError;

#[derive(Default)]
pub struct Attribution {
    by_run: BTreeMap<String, u64>,
    by_actor: BTreeMap<String, u64>,
    by_segment: BTreeMap<String, u64>,
    by_tool: BTreeMap<String, u64>,
    by_skill: BTreeMap<String, u64>,
    total: u64,
    /// Basis for the next model_returned: the most recent prefix shape
    /// and the tool bytes returned since the previous call.
    segment_weights: Vec<(String, u64)>,
    tool_weights: BTreeMap<String, u64>,
    skill_weights: BTreeMap<String, u64>,
}

pub struct AttributionReport {
    pub total: UsdMicros,
    pub by_run: Vec<(String, UsdMicros)>,
    pub by_actor: Vec<(String, UsdMicros)>,
    pub by_segment: Vec<(String, UsdMicros)>,
    pub by_tool: Vec<(String, UsdMicros)>,
    pub by_skill: Vec<(String, UsdMicros)>,
}

impl Attribution {
    pub fn new() -> Attribution {
        Attribution::default()
    }

    /// Folds one record in. Only `model_returned` moves money; the
    /// other two kinds set the basis on which the next call is split.
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), MemoryError> {
        match record.kind() {
            EventKind::PromptAssembled => {
                self.segment_weights = segment_weights(record.data().as_map());
            }
            EventKind::ToolResult => {
                let name = record
                    .data()
                    .as_map()
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unnamed")
                    .to_owned();
                let bytes = record
                    .data()
                    .as_map()
                    .get("bytes")
                    .and_then(Value::as_u64)
                    .unwrap_or(1);
                let slot = self.tool_weights.entry(name).or_insert(0);
                *slot = slot.saturating_add(bytes);
                // The SKILL a call was made under, when one was: P1's
                // Library fills this; until then every call is unsplit.
                let skill = record
                    .data()
                    .as_map()
                    .get("skill")
                    .and_then(Value::as_str)
                    .unwrap_or(NO_SKILL)
                    .to_owned();
                let slot = self.skill_weights.entry(skill).or_insert(0);
                *slot = slot.saturating_add(bytes);
            }
            EventKind::ModelReturned => {
                let Some(billed) = record
                    .data()
                    .as_map()
                    .get("billed_usd_micros")
                    .and_then(Value::as_u64)
                else {
                    // A call with no authoritative amount attributes
                    // nothing. Estimating here would invent money.
                    self.tool_weights.clear();
                    self.skill_weights.clear();
                    return Ok(());
                };
                self.total = self.total.saturating_add(billed);
                add(&mut self.by_run, &record.run().to_string(), billed);
                add(&mut self.by_actor, record.who(), billed);
                for (bucket, share) in split(billed, &self.segment_weights, NO_SEGMENT) {
                    add(&mut self.by_segment, &bucket, share);
                }
                let tool_basis: Vec<(String, u64)> = self
                    .tool_weights
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
                for (bucket, share) in split(billed, &tool_basis, NO_TOOL) {
                    add(&mut self.by_tool, &bucket, share);
                }
                let skill_basis: Vec<(String, u64)> = self
                    .skill_weights
                    .iter()
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
                for (bucket, share) in split(billed, &skill_basis, NO_SKILL) {
                    add(&mut self.by_skill, &bucket, share);
                }
                // The wave is settled; the next call starts a new basis.
                self.tool_weights.clear();
                self.skill_weights.clear();
            }
            _ => {}
        }
        Ok(())
    }

    pub fn report(&self) -> AttributionReport {
        AttributionReport {
            total: UsdMicros::new(self.total),
            by_run: quantify(&self.by_run),
            by_actor: quantify(&self.by_actor),
            by_segment: quantify(&self.by_segment),
            by_tool: quantify(&self.by_tool),
            by_skill: quantify(&self.by_skill),
        }
    }
}

#[cfg(test)]
mod tests;

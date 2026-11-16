// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::split::{add, quantify, segment_weights, split};

pub(crate) const NO_TOOL: &str = "no_tool";
pub(crate) const NO_SKILL: &str = "no_skill";
pub(crate) const NO_SEGMENT: &str = "unattributed";
pub(crate) const WINDOW_SLOT: &str = "window";

pub(crate) const KEEP_WARM: &str = "keep_warm";

use std::collections::BTreeMap;

use kernel::event::record::{CacheRenewed, RenewalOutcome};
use kernel::{Address, EventKind, EventRecord, RunId, UsdMicros};
use serde_json::Value;

use crate::error::StorageError;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Attribution {
    by_run: BTreeMap<String, u64>,
    by_actor: BTreeMap<String, u64>,
    by_segment: BTreeMap<String, u64>,
    by_tool: BTreeMap<String, u64>,
    by_skill: BTreeMap<String, u64>,
    total: u64,

    segment_weights: BTreeMap<RunId, Vec<(String, u64)>>,
    tool_weights: BTreeMap<String, u64>,
    skill_weights: BTreeMap<String, u64>,
    unpriced: Unpriced,
    #[serde(default)]
    renewals: Renewals,
}

pub struct AttributionReport {
    pub total: UsdMicros,
    pub by_run: Vec<(String, UsdMicros)>,
    pub by_actor: Vec<(String, UsdMicros)>,
    pub by_segment: Vec<(String, UsdMicros)>,
    pub by_tool: Vec<(String, UsdMicros)>,
    pub by_skill: Vec<(String, UsdMicros)>,
    pub unpriced: Unpriced,
    pub renewals: Renewals,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Unpriced {
    pub calls: u64,
    pub tokens: u64,
}

impl Unpriced {

    fn count(&mut self, data: &serde_json::Map<String, Value>) {
        self.calls = self.calls.saturating_add(1);
        self.tokens = self.tokens.saturating_add(usage_tokens(data));
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Renewals {
    #[serde(default)]
    pub billed: UsdMicros,
    pub read: u64,
    pub late: u64,
    pub unreported: u64,
    pub refused: u64,
}

impl Renewals {
    fn count(&mut self, outcome: RenewalOutcome) {
        let slot = match outcome {
            RenewalOutcome::Read => &mut self.read,
            RenewalOutcome::Late => &mut self.late,
            RenewalOutcome::Unreported => &mut self.unreported,
            RenewalOutcome::Refused => &mut self.refused,
        };
        *slot = slot.saturating_add(1);
    }
}

impl Attribution {
    pub fn new() -> Attribution {
        Attribution::default()
    }

    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "five kinds carry money, its basis or a renewal; the rest of the vocabulary is silent here"
    )]
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), StorageError> {
        match record.kind() {
            EventKind::CacheRenewed => self.renewed(record),
            EventKind::PromptAssembled => {
                self.segment_weights
                    .insert(record.run(), segment_weights(record.data().as_map()));
            }
            EventKind::RunFrozen => {
                self.segment_weights.remove(&record.run());
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

                    self.unpriced.count(record.data().as_map());
                    self.tool_weights.clear();
                    self.skill_weights.clear();
                    return Ok(());
                };
                self.total = self.total.saturating_add(billed);
                add(&mut self.by_run, &record.run().to_string(), billed);
                add(&mut self.by_actor, record.who(), billed);
                let segment_basis = self
                    .segment_weights
                    .get(&record.run())
                    .map_or(&[][..], Vec::as_slice);
                for (bucket, share) in split(billed, segment_basis, NO_SEGMENT) {
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

                self.tool_weights.clear();
                self.skill_weights.clear();
            }
            _ => {}
        }
        Ok(())
    }

    fn renewed(&mut self, record: &EventRecord) {
        let line = match record.data().read::<CacheRenewed>() {
            Ok(line) => line,
            Err(_unreadable) => {
                self.renewals.count(RenewalOutcome::Unreported);
                return;
            }
        };
        self.renewals.count(line.outcome());
        match line {
            CacheRenewed::Refused { .. } => {}
            CacheRenewed::Answered {
                billed_usd_micros: None,
                ..
            } => self.unpriced.count(record.data().as_map()),
            CacheRenewed::Answered {
                billed_usd_micros: Some(billed),
                ..
            } => {
                let micros = billed.get();
                self.total = self.total.saturating_add(micros);
                self.renewals.billed =
                    UsdMicros::new(self.renewals.billed.get().saturating_add(micros));
                let room = record.addr().map_or(record.who(), Address::as_str);
                add(&mut self.by_actor, room, micros);
                for cut in [
                    &mut self.by_run,
                    &mut self.by_segment,
                    &mut self.by_tool,
                    &mut self.by_skill,
                ] {
                    add(cut, KEEP_WARM, micros);
                }
            }
        }
    }

    pub fn billed_to(&self, run: &RunId) -> Option<UsdMicros> {
        self.by_run
            .get(&run.to_string())
            .copied()
            .map(UsdMicros::new)
    }

    pub fn retain_runs(&mut self, keep: impl Fn(&RunId) -> bool) {
        self.by_run
            .retain(|name, _| RunId::parse(name).map_or(true, |run| keep(&run)));
    }

    pub fn report(&self) -> AttributionReport {
        AttributionReport {
            total: UsdMicros::new(self.total),
            by_run: quantify(&self.by_run),
            by_actor: quantify(&self.by_actor),
            by_segment: quantify(&self.by_segment),
            by_tool: quantify(&self.by_tool),
            by_skill: quantify(&self.by_skill),
            unpriced: self.unpriced,
            renewals: self.renewals,
        }
    }
}

fn usage_tokens(data: &serde_json::Map<String, Value>) -> u64 {
    let usage = data.get("usage");
    ["input_tokens", "output_tokens"]
        .into_iter()
        .filter_map(|field| usage.and_then(|u| u.get(field)).and_then(Value::as_u64))
        .fold(0, u64::saturating_add)
}

#[cfg(test)]
mod tests;

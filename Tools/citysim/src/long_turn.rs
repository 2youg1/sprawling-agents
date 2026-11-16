// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::atomic::{AtomicU32, Ordering};

use kernel::{
    Address, AxCode, AxError, ClockStampGranularity, ContentBlock, CostTier, Effect, FrozenConfig,
    ModelReturn, Payload, RenderIntent, RunId, Temporal, Tool, ToolCall, ToolMeta, ToolName,
    ToolOutcome, WriteDomain,
};
use runtime::bench::ToolBench;
use serde_json::Value;

use crate::executor::{Scenario, run_scenario};
use crate::script_model::{ScriptModel, concluding};

pub const STEP_BYTES: usize = 1024;

const NOTES: &str = "notes";

pub enum Pauses {
    Never,

    Every {
        steps: u32,
        at: Box<dyn Fn(u32) -> Result<(), AxError> + Send + Sync>,
    },
}

pub struct TurnReading {
    pub completion: &'static str,

    pub windows: Vec<u64>,

    pub ledger_bytes: u64,
}

pub fn long_turn(steps: u32, pauses: Pauses) -> Result<TurnReading, AxError> {
    let report = run_scenario(scenario(steps, pauses)?)?;
    let mut ledger_bytes: u64 = 0;
    for line in &report.lines {
        ledger_bytes = ledger_bytes.saturating_add(counted(line.len())?);
    }
    Ok(TurnReading {
        completion: report.completion,
        windows: windows_of(&report.lines)?,
        ledger_bytes,
    })
}

fn counted(len: usize) -> Result<u64, AxError> {
    u64::try_from(len).map_err(|err| {
        AxError::failure(AxCode::InvalidArgs, "count a long turn", err.to_string())
            .with_recovery("run a shorter turn: this length does not fit a u64")
    })
}

fn windows_of(lines: &[Vec<u8>]) -> Result<Vec<u64>, AxError> {
    let unreadable = |detail: String| {
        AxError::failure(AxCode::InvalidArgs, "read a long turn's window", detail).with_recovery(
            "the prompt_shape_compared line changed shape; read its upper bound where \
                 kernel::event::record::PromptShapeCompared now keeps it",
        )
    };
    let mut windows = Vec::new();
    for line in lines {
        let record: Value =
            serde_json::from_slice(line).map_err(|err| unreadable(err.to_string()))?;
        if record.get("kind").and_then(Value::as_str) != Some("prompt_shape_compared") {
            continue;
        }
        let bound = record
            .get("data")
            .and_then(|data| data.get("upper_bound"))
            .and_then(Value::as_u64)
            .ok_or_else(|| unreadable("a line with no upper_bound".to_owned()))?;
        windows.push(bound);
    }
    Ok(windows)
}

fn scenario(steps: u32, pauses: Pauses) -> Result<Scenario, AxError> {
    let addr = Address::parse("sim/lobby/room1")?;
    let mut bench = ToolBench::new(WriteDomain::new(vec![addr.clone()])?);
    bench.register(Box::new(Notes {
        meta: notes_meta()?,
        read: AtomicU32::new(0),
        pauses,
    }))?;
    let mut script = Vec::new();
    for step in 1..=steps {
        script.push(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: "reading the notes again".to_owned(),
            }])?,
            vec![ToolCall {
                id: format!("call-{step:06}"),
                name: ToolName::parse(NOTES)?,
                args: Payload::empty(),
            }],
        ));
    }
    script.push(concluding("the notes were read to the end")?);
    Ok(Scenario {
        run: RunId::parse("0198f6a2-7c4a-7bbb-9d1e-0000000000f7")?,
        who: "worker@sim.1".to_owned(),
        addr,
        task: "read the notes at every step".to_owned(),
        goal: "a long turn, then stop".to_owned(),
        job_md: "# JOB\nRead the notes until they are done.".to_owned(),
        model: ScriptModel::new(script),
        bench,
        config: FrozenConfig {
            clock_stamp: ClockStampGranularity::Off,
            clock_zones: Vec::new(),
            sandbox: kernel::SandboxLimits::default(),
            mcp: Vec::new(),
            mcp_references: Vec::new(),
            skill_pins: std::collections::BTreeMap::new(),
            effort: None,
            second_threshold: None,
            search: kernel::config::SearchConfiguration::Default,
            catalog: kernel::config::CatalogExposure::default(),
        },
        checkpoint: None,
        cancel: None,
        steer: None,
        sieve: None,
    })
}

fn notes_meta() -> Result<ToolMeta, AxError> {
    Ok(ToolMeta {
        name: ToolName::parse(NOTES)?,
        disclosure: "the notes, as they stand at this step".to_owned(),
        params: Payload::empty(),
        effect: Effect::Read,
        cost_tier: CostTier::Free,
        timeout: None,
        render: RenderIntent::Generic,
        temporal: Temporal::Timeless,
    })
}

struct Notes {
    meta: ToolMeta,
    read: AtomicU32,
    pauses: Pauses,
}

impl Tool for Notes {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
        let read = self.read.fetch_add(1, Ordering::SeqCst).saturating_add(1);
        let mut map = serde_json::Map::new();
        map.insert("text".to_owned(), Value::String(notes_at(read)?));
        let outcome = ToolOutcome {
            result: Payload::new(map)?,
            attachments: Vec::new(),
        };
        match &self.pauses {
            Pauses::Every { steps, at } if read.is_multiple_of(*steps) => at(read)?,
            Pauses::Every { .. } | Pauses::Never => {}
        }
        Ok(outcome)
    }
}

fn notes_at(read: u32) -> Result<String, AxError> {
    let head = format!("step {read:06} ");
    let turn = usize::try_from(read % 26).map_err(|err| {
        AxError::failure(
            AxCode::InvalidArgs,
            "write a long turn's notes",
            err.to_string(),
        )
        .with_recovery("run this simulator on a target whose usize holds 26")
    })?;
    Ok(head
        .chars()
        .chain(
            ('a'..='z')
                .cycle()
                .skip(turn)
                .take(STEP_BYTES.saturating_sub(head.len())),
        )
        .collect())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn a_long_turn_grows_its_window_by_one_step_at_a_time() {
        for steps in [40u32, 80] {
            let reading = long_turn(steps, Pauses::Never).unwrap();
            let increments: Vec<u64> = reading
                .windows
                .windows(2)
                .map(|pair| pair[1] - pair[0])
                .collect();
            let first = increments.first().copied().unwrap_or(0);
            assert_eq!(
                (reading.completion, reading.windows.len(), first > 0),
                ("done", usize::try_from(steps).unwrap() + 1, true),
                "a long turn of {steps} steps"
            );
            assert!(
                increments.iter().all(|step| *step == first),
                "every step adds {first} bytes at {steps} steps: {increments:?}"
            );
        }
    }
}

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod parse;

pub use parse::parse;

pub const HEADER: [&str; 28] = [
    "schema_version",
    "observation_id",
    "task_attempt_id",
    "request_id",
    "span_start",
    "span_end",
    "round",
    "build",
    "arm",
    "agent",
    "series",
    "workload",
    "case",
    "cohort",
    "state",
    "sessions",
    "metric",
    "unit",
    "index",
    "value",
    "failure_class",
    "missing_reason",
    "censored",
    "usage_complete",
    "selected",
    "in_flight",
    "counter_source",
    "condition",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    None,

    Infrastructure,

    Capability,

    Guard,

    Timeout,
}

#[derive(Debug, Clone)]
pub struct Observation {
    pub observation_id: String,

    pub task_attempt_id: Option<String>,

    pub request_id: Option<String>,

    pub span_start: Option<f64>,

    pub span_end: Option<f64>,

    pub round: String,

    pub build: String,

    pub arm: String,

    pub agent: String,

    pub series: String,

    pub workload: String,

    pub case: String,

    pub cohort: String,

    pub state: String,

    pub sessions: Option<f64>,

    pub metric: String,

    pub unit: String,

    pub index: Option<u64>,

    pub value: f64,

    pub failure_class: FailureClass,

    pub missing_reason: Option<String>,

    pub censored: bool,

    pub usage_complete: bool,

    pub selected: bool,

    pub in_flight: Option<f64>,

    pub counter_source: String,

    pub condition: String,
}

impl Observation {
    fn numeric_cell(&self) -> bool {
        self.selected && !self.censored && self.effective_missing_reason().is_none()
    }

    pub fn effective_missing_reason(&self) -> Option<&str> {
        self.missing_reason.as_deref().or_else(|| {
            (matches!(self.metric.as_str(), "input_tokens" | "output_tokens")
                && !self.usage_complete)
                .then_some("token usage is incomplete")
        })
    }

    pub fn quantitative(&self) -> Option<f64> {
        self.numeric_cell().then_some(self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellKey {
    pub build: String,

    pub arm: String,

    pub agent: String,

    pub series: String,

    pub workload: String,

    pub case: String,

    pub state: String,

    pub sessions: Option<String>,

    pub metric: String,

    pub unit: String,

    pub counter_source: String,

    pub condition: String,
}

impl CellKey {
    pub fn of(row: &Observation) -> Self {
        Self {
            build: row.build.clone(),
            arm: row.arm.clone(),
            agent: row.agent.clone(),
            series: row.series.clone(),
            workload: row.workload.clone(),
            case: row.case.clone(),
            state: row.state.clone(),
            sessions: row.sessions.map(|value| value.to_string()),
            metric: row.metric.clone(),
            unit: row.unit.clone(),
            counter_source: row.counter_source.clone(),
            condition: row.condition.clone(),
        }
    }

    pub fn id(&self) -> String {
        let sessions = self.sessions.as_deref().unwrap_or("");
        [
            self.build.as_str(),
            self.arm.as_str(),
            self.agent.as_str(),
            self.series.as_str(),
            self.workload.as_str(),
            self.case.as_str(),
            self.state.as_str(),
            sessions,
            self.metric.as_str(),
            self.unit.as_str(),
            self.counter_source.as_str(),
            self.condition.as_str(),
        ]
        .iter()
        .map(|part| format!("{}:{part}", part.len()))
        .collect::<Vec<_>>()
        .join("|")
    }

    pub fn label(&self) -> String {
        self.id()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests;

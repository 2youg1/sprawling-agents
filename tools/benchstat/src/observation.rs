// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The 28-column observation schema and its stable cell identity.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::BenchError;
use crate::manifest::{DesignStatus, design_status};

/// Exact production header, in its required order.
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

/// Failure classification, which also controls outcome eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// Normal completion.
    None,
    /// Provider or transport failure.
    Infrastructure,
    /// Eligible check failure.
    Capability,
    /// Declared guard stopped the attempt.
    Guard,
    /// Deadline stopped the attempt.
    Timeout,
}

/// One validated metric row.
#[derive(Debug, Clone)]
pub struct Observation {
    /// Stable row id.
    pub observation_id: String,
    /// Owning task attempt, when this is a task metric.
    pub task_attempt_id: Option<String>,
    /// Owning invocation within that attempt.
    pub request_id: Option<String>,
    /// Monotonic span start relative to the attempt.
    pub span_start: Option<f64>,
    /// Monotonic span end relative to the attempt.
    pub span_end: Option<f64>,
    /// Interleaved round.
    pub round: String,
    /// Measured build.
    pub build: String,
    /// Experiment arm.
    pub arm: String,
    /// Subject.
    pub agent: String,
    /// Registry series.
    pub series: String,
    /// Workload name.
    pub workload: String,
    /// Cell case.
    pub case: String,
    /// Independent cohort.
    pub cohort: String,
    /// Memory phase or another declared state.
    pub state: String,
    /// Session level for a memory fit.
    pub sessions: Option<f64>,
    /// Raw metric name.
    pub metric: String,
    /// Raw unit.
    pub unit: String,
    /// Position in collection order.
    pub index: Option<u64>,
    /// Finite raw value or lower bound.
    pub value: f64,
    /// Failure class.
    pub failure_class: FailureClass,
    /// Concrete missingness reason.
    pub missing_reason: Option<String>,
    /// Whether value is right-censored.
    pub censored: bool,
    /// Whether token usage is complete.
    pub usage_complete: bool,
    /// Whether this is the pre-registered primary attempt.
    pub selected: bool,
    /// Actual queue depth.
    pub in_flight: Option<f64>,
    /// Instrument and adapter.
    pub counter_source: String,
    /// Configuration condition.
    pub condition: String,
}

impl Observation {
    fn numeric_cell(&self) -> bool {
        self.selected && !self.censored && self.missing_reason.is_none()
    }

    /// Whether this row contributes to an ordinary quantitative estimate.
    pub fn quantitative(&self) -> Option<f64> {
        self.numeric_cell().then_some(self.value)
    }
}

/// The fields that define one analysis cell. Round and cohort stay inside it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellKey {
    /// Build.
    pub build: String,
    /// Arm.
    pub arm: String,
    /// Subject.
    pub agent: String,
    /// Series.
    pub series: String,
    /// Workload.
    pub workload: String,
    /// Case.
    pub case: String,
    /// State.
    pub state: String,
    /// Session level, absent for pooled fits.
    pub sessions: Option<String>,
    /// Metric.
    pub metric: String,
    /// Unit.
    pub unit: String,
    /// Counter adapter.
    pub counter_source: String,
    /// Configuration.
    pub condition: String,
}

impl CellKey {
    /// Construct the identity from one observation.
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

    /// The length-prefixed identity used by manifests and seeds.
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

    /// A short human label used in the Markdown report.
    pub fn label(&self) -> String {
        let case = if self.case.is_empty() {
            "all"
        } else {
            self.case.as_str()
        };
        let sessions = self
            .sessions
            .as_ref()
            .map(|sessions| format!(" sessions={sessions}"))
            .unwrap_or_default();
        format!(
            "{}/{}/{}{}/{} ({})",
            self.agent, self.series, case, sessions, self.metric, self.unit
        )
    }
}

/// Decode and validate the complete CSV.
///
/// # Errors
/// Rejects malformed records, duplicate ids, invalid units and broken task joins.
pub fn parse(bytes: &[u8]) -> Result<Vec<Observation>, BenchError> {
    if bytes.contains(&b'\r') {
        return Err(invalid(
            "results.csv line endings",
            "convert the file to LF only, including its final record",
        ));
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(bytes);
    let headers = reader.headers().map_err(csv_error)?.clone();
    if headers.len() != HEADER.len()
        || !headers
            .iter()
            .zip(HEADER.iter())
            .all(|(actual, expected)| actual == *expected)
    {
        return Err(invalid(
            "results.csv header",
            "use the exact 28-column header from methodology.md in its declared order",
        ));
    }
    let mut rows = Vec::new();
    let mut ids = BTreeSet::new();
    for result in reader.records() {
        if rows.len() >= 1_000_000 {
            return Err(invalid(
                "results.csv row count",
                "split the round so it holds at most 1000000 rows",
            ));
        }
        let record = result.map_err(csv_error)?;
        let row = observation(&record)?;
        if !ids.insert(row.observation_id.clone()) {
            return Err(invalid(
                &format!("duplicate observation_id {}", row.observation_id),
                "give every metric row a unique stable id",
            ));
        }
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(invalid(
            "results.csv observations",
            "publish at least one raw observation",
        ));
    }
    validate_task_joins(&rows)?;
    Ok(rows)
}

fn observation(record: &csv::StringRecord) -> Result<Observation, BenchError> {
    let schema = field(record, 0)?;
    if schema != "1" {
        return Err(invalid("schema_version", "write schema_version = 1"));
    }
    let observation_id = field(record, 1)?.to_owned();
    if observation_id.is_empty() {
        return Err(invalid(
            "observation_id",
            "give every metric row a non-empty stable id",
        ));
    }
    let series = field(record, 10)?.to_owned();
    if design_status(&series) == Some(DesignStatus::Unsupported) || design_status(&series).is_none()
    {
        return Err(invalid(
            &format!("series {series}"),
            "use a measurable series from the design registry",
        ));
    }
    let metric = field(record, 16)?.to_owned();
    let unit = field(record, 17)?.to_owned();
    validate_unit(&metric, &unit)?;
    let value = number(field(record, 19)?, "value")?;
    if !value.is_finite() {
        return Err(invalid("value", "write only finite observations"));
    }
    if matches!(
        metric.as_str(),
        "functional" | "contract" | "normal_completion"
    ) && value != 0.0
        && value != 1.0
    {
        return Err(invalid(
            &format!("outcome metric {metric}"),
            "rate rows carry only 0 or 1",
        ));
    }
    let task_attempt_id = optional(field(record, 2)?);
    let request_id = optional(field(record, 3)?);
    let span_start = optional_number(field(record, 4)?, "span_start")?;
    let span_end = optional_number(field(record, 5)?, "span_end")?;
    if request_id.is_some() && task_attempt_id.is_none() {
        return Err(invalid(
            &format!("request {}", request_id.unwrap_or_default()),
            "every request id needs its task_attempt_id",
        ));
    }
    if matches!(
        metric.as_str(),
        "wall" | "model_latency" | "tool_latency" | "queue_latency"
    ) && (span_start.is_none() || span_end.is_none())
    {
        return Err(invalid(
            &format!("span for {metric}"),
            "task wall and request latency rows need both monotonic endpoints",
        ));
    }
    let failure_class = match field(record, 20)? {
        "none" => FailureClass::None,
        "infrastructure" => FailureClass::Infrastructure,
        "capability" => FailureClass::Capability,
        "guard" => FailureClass::Guard,
        "timeout" => FailureClass::Timeout,
        _ => {
            return Err(invalid(
                "failure_class",
                "use none, infrastructure, capability, guard or timeout",
            ));
        }
    };
    if metric == "normal_completion"
        && ((matches!(failure_class, FailureClass::Guard | FailureClass::Timeout) && value != 0.0)
            || (failure_class == FailureClass::None && value != 1.0))
    {
        return Err(invalid(
            "normal_completion/failure_class",
            "normal exit is 1; guard and timeout are 0",
        ));
    }
    Ok(Observation {
        observation_id,
        task_attempt_id,
        request_id,
        span_start,
        span_end,
        round: field(record, 6)?.to_owned(),
        build: field(record, 7)?.to_owned(),
        arm: field(record, 8)?.to_owned(),
        agent: field(record, 9)?.to_owned(),
        series,
        workload: field(record, 11)?.to_owned(),
        case: field(record, 12)?.to_owned(),
        cohort: field(record, 13)?.to_owned(),
        state: field(record, 14)?.to_owned(),
        sessions: optional_number(field(record, 15)?, "sessions")?,
        metric,
        unit,
        index: match field(record, 18)? {
            "" => None,
            text => Some(text.parse::<u64>().map_err(|_| {
                invalid("index", "write a non-negative integer or leave index empty")
            })?),
        },
        value,
        failure_class,
        missing_reason: optional(field(record, 21)?),
        censored: boolean(field(record, 22)?, "censored")?,
        usage_complete: boolean(field(record, 23)?, "usage_complete")?,
        selected: boolean(field(record, 24)?, "selected")?,
        in_flight: optional_number(field(record, 25)?, "in_flight")?,
        counter_source: field(record, 26)?.to_owned(),
        condition: field(record, 27)?.to_owned(),
    })
}

fn validate_task_joins(rows: &[Observation]) -> Result<(), BenchError> {
    let mut owners = BTreeMap::<String, (&str, &str, &str, &str, &str, &str)>::new();
    let mut walls = BTreeMap::<String, (f64, f64)>::new();
    let mut model_requests = BTreeMap::<String, BTreeSet<String>>::new();
    for row in rows {
        let Some(id) = &row.task_attempt_id else {
            continue;
        };
        let identity = (
            row.build.as_str(),
            row.arm.as_str(),
            row.agent.as_str(),
            row.workload.as_str(),
            row.case.as_str(),
            row.round.as_str(),
        );
        if owners.get(id).is_some_and(|first| *first != identity) {
            return Err(invalid(
                &format!("task ownership for {id}"),
                "keep subject, build, arm, workload, case and round consistent within an attempt",
            ));
        }
        owners.insert(id.clone(), identity);
        if row.metric == "wall" {
            let (Some(start), Some(end)) = (row.span_start, row.span_end) else {
                return Err(invalid(
                    &format!("wall span for {id}"),
                    "task wall rows need both monotonic endpoints",
                ));
            };
            if start != 0.0 || end < start {
                return Err(invalid(
                    &format!("wall span for {id}"),
                    "start the task wall at zero and end it at or after the start",
                ));
            }
            let duration = end - start;
            if (duration - seconds(row)?).abs() > 1e-9_f64.max(seconds(row)?.abs() * 1e-9) {
                return Err(invalid(
                    &format!("wall duration for {id}"),
                    "make value equal span_end - span_start after conversion to seconds",
                ));
            }
            if walls.insert(id.clone(), (start, end)).is_some() {
                return Err(invalid(
                    &format!("duplicate wall span for {id}"),
                    "keep exactly one wall span per task attempt",
                ));
            }
        }
        if row.metric == "model_latency"
            && let Some(request) = &row.request_id
        {
            model_requests
                .entry(id.clone())
                .or_default()
                .insert(request.clone());
        }
    }
    for row in rows {
        if matches!(
            row.metric.as_str(),
            "wall"
                | "requests"
                | "input_tokens"
                | "output_tokens"
                | "functional"
                | "contract"
                | "normal_completion"
        ) && row.task_attempt_id.is_none()
        {
            return Err(invalid(
                &format!("task metric {}", row.metric),
                "every task aggregate needs task_attempt_id",
            ));
        }
        if matches!(
            row.metric.as_str(),
            "model_latency" | "tool_latency" | "queue_latency"
        ) && row.request_id.is_none()
        {
            return Err(invalid(
                &format!("request metric {}", row.metric),
                "every request span needs request_id",
            ));
        }
        if let Some(id) = &row.task_attempt_id {
            if let Some(bounds) = walls.get(id) {
                validate_within(row, *bounds)?;
            }
            if row.metric == "requests" && model_requests.contains_key(id) {
                let actual = model_requests.get(id).map_or(0, BTreeSet::len);
                let expected = u32::try_from(actual).map_err(|_| {
                    invalid(
                        &format!("request count for {id}"),
                        "too many distinct model invocation ids",
                    )
                })?;
                if row.value != f64::from(expected) {
                    return Err(invalid(
                        &format!("request count for {id}"),
                        "make requests equal the number of distinct model invocation ids",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn validate_within(row: &Observation, bounds: (f64, f64)) -> Result<(), BenchError> {
    if matches!(
        row.metric.as_str(),
        "model_latency" | "tool_latency" | "queue_latency"
    ) {
        match (row.span_start, row.span_end) {
            (Some(start), Some(end)) if start >= bounds.0 && end >= start && end <= bounds.1 => {
                let duration = end - start;
                if (duration - seconds(row)?).abs() > 1e-9_f64.max(seconds(row)?.abs() * 1e-9) {
                    return Err(invalid(
                        &format!("request duration {}", row.observation_id),
                        "make value equal span_end - span_start",
                    ));
                }
            }
            _ => {
                return Err(invalid(
                    &format!("request span {}", row.observation_id),
                    "keep every request span ordered and inside its task wall",
                ));
            }
        }
    }
    Ok(())
}

fn seconds(row: &Observation) -> Result<f64, BenchError> {
    match row.unit.as_str() {
        "s" => Ok(row.value),
        "ms" => Ok(row.value / 1e3),
        "us" => Ok(row.value / 1e6),
        "ns" => Ok(row.value / 1e9),
        _ => Err(invalid(
            &format!("time unit {} for {}", row.unit, row.observation_id),
            "use ns, us, ms or s for wall and request spans",
        )),
    }
}

fn validate_unit(metric: &str, unit: &str) -> Result<(), BenchError> {
    let valid = match metric {
        "latency" | "wall" | "model_latency" | "tool_latency" | "queue_latency" => {
            matches!(unit, "ns" | "us" | "ms" | "s")
        }
        "memory_private" | "read_bytes" | "write_bytes" => matches!(unit, "bytes" | "MiB"),
        "requests" => unit == "requests",
        "input_tokens" | "output_tokens" => unit == "tokens",
        "functional" | "contract" | "normal_completion" => unit == "ratio",
        _ => matches!(
            unit,
            "count" | "ratio" | "ns" | "us" | "ms" | "s" | "bytes" | "MiB"
        ),
    };
    if valid {
        Ok(())
    } else {
        Err(invalid(
            &format!("unit {unit} for metric {metric}"),
            "use the metric's registered raw unit; convert before publishing",
        ))
    }
}

fn field(record: &csv::StringRecord, index: usize) -> Result<&str, BenchError> {
    record.get(index).ok_or_else(|| {
        invalid(
            &format!(
                "column {} ({})",
                index,
                HEADER.get(index).unwrap_or(&"unknown")
            ),
            "supply every column, using an empty field where the schema permits it",
        )
    })
}

fn optional(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_owned())
}

fn number(text: &str, subject: &str) -> Result<f64, BenchError> {
    text.parse::<f64>().map_err(|_| {
        invalid(
            subject,
            "write a finite decimal number, not an empty or non-numeric field",
        )
    })
}

fn optional_number(text: &str, subject: &str) -> Result<Option<f64>, BenchError> {
    if text.is_empty() {
        Ok(None)
    } else {
        let value = number(text, subject)?;
        if value.is_finite() {
            Ok(Some(value))
        } else {
            Err(invalid(subject, "write only finite values"))
        }
    }
}

fn boolean(text: &str, subject: &str) -> Result<bool, BenchError> {
    match text {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(invalid(subject, "write 0 or 1")),
    }
}

fn csv_error(error: csv::Error) -> BenchError {
    BenchError::input(
        "parse results.csv",
        error.to_string(),
        "fix the quoted CSV record named by the parser",
    )
}

fn invalid(subject: &str, recovery: &str) -> BenchError {
    BenchError::input("validate results.csv", subject.to_owned(), recovery)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::{HEADER, parse};

    #[test]
    fn task_spans_convert_milliseconds_and_count_model_invocations() {
        assert_eq!(
            parse(task_csv("1").as_bytes()).map(|rows| rows.len()),
            Ok(3)
        );
        let error = parse(task_csv("2").as_bytes()).unwrap_err();
        assert_eq!(error.code, "E_INVALID_INPUT");
        assert!(error.subject.contains("request count"));
    }

    fn task_csv(requests: &str) -> String {
        format!(
            "{}\n1,w,a,,0,1,r,b,arm,agent,task_wall,work,case,c,,,wall,ms,0,1000,none,,0,1,1,,fixture,\n1,n,a,,,,r,b,arm,agent,task_requests,work,case,c,,,requests,requests,0,{requests},none,,0,1,1,,fixture,\n1,m,a,r,0.2,0.6,r,b,arm,agent,request_latency,work,case,c,,,model_latency,ms,0,400,none,,0,1,1,,fixture,\n",
            HEADER.join(",")
        )
    }
}

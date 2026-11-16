// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use crate::error::BenchError;
use crate::manifest::{DesignStatus, design_status};

use super::{FailureClass, HEADER, Observation};

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

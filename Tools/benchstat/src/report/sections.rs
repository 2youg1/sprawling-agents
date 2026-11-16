// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde_json::{Map, Value};

use crate::error::BenchError;

pub(super) fn decomposition_section(object: &Map<String, Value>) -> Result<String, BenchError> {
    let rows = object
        .get("task_decompositions")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("task_decompositions is not an object"))?;
    let unavailable = rows
        .values()
        .filter(|row| row.get("wall").is_none())
        .count();
    let mut text = format!(
        "\n### Task wall decompositions\n\n{} attempts have no complete request-span decomposition.\n\n| Attempt | Wall (s) | Covered (s) | Uncovered (s) | Overlap (s) | Status |\n| --- | ---: | ---: | ---: | ---: | --- |\n",
        unavailable
    );
    for (id, row) in rows {
        if row.get("wall").is_none() {
            continue;
        }
        text.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            escape(id),
            row.get("wall")
                .map(display)
                .unwrap_or_else(|| "unavailable".to_owned()),
            row.get("covered")
                .map(display)
                .unwrap_or_else(|| "unavailable".to_owned()),
            row.get("uncovered")
                .map(display)
                .unwrap_or_else(|| "unavailable".to_owned()),
            row.get("overlap")
                .map(display)
                .unwrap_or_else(|| "unavailable".to_owned()),
            row.get("status")
                .and_then(Value::as_str)
                .unwrap_or("unavailable")
        ));
    }
    Ok(text)
}

pub(super) fn analysis_provenance(object: &Map<String, Value>) -> Result<String, BenchError> {
    let parameters = object
        .get("parameters")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("parameters is not an object"))?;
    let field = |name: &str| {
        object
            .get(name)
            .and_then(Value::as_str)
            .unwrap_or("unavailable")
    };
    Ok(format!(
        "### Analysis provenance\n\n- Analysis revision: `{}`\n- Method version: `{}`\n- Estimator version: `{}`\n- Resamples: {}\n- Confidence: {}\n- Alpha: {}\n- Power: {}\n- Bootstrap unit: `{}`\n- Seed: `{}`\n\nReproduce with `benchstat export <round> --write`, `benchstat verify <round>` and `benchstat report <round> --write` at the recorded analysis revision.\n\n",
        field("analysis_revision"),
        field("method_version"),
        field("estimator_version"),
        parameters
            .get("resamples")
            .map(display)
            .unwrap_or_else(|| "unavailable".to_owned()),
        parameters
            .get("confidence")
            .map(display)
            .unwrap_or_else(|| "unavailable".to_owned()),
        parameters
            .get("alpha")
            .map(display)
            .unwrap_or_else(|| "unavailable".to_owned()),
        parameters
            .get("power")
            .map(display)
            .unwrap_or_else(|| "unavailable".to_owned()),
        parameters
            .get("bootstrap_unit")
            .and_then(Value::as_str)
            .unwrap_or("unavailable"),
        parameters
            .get("seed_string")
            .and_then(Value::as_str)
            .unwrap_or("unavailable"),
    ))
}

pub(super) fn coverage_and_claims(object: &Map<String, Value>) -> Result<String, BenchError> {
    let mut text = String::from(
        "### Coverage\n\n| Series | Status | Reason | Reopen when |\n| --- | --- | --- | --- |\n",
    );
    for row in object
        .get("coverage")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("coverage is not an array"))?
    {
        let series = row
            .get("series")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let status = row
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let reason = row.get("reason").and_then(Value::as_str).unwrap_or("");
        let reopen = row.get("reopen_when").and_then(Value::as_str).unwrap_or("");
        text.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            escape(series),
            escape(status),
            escape(reason),
            escape(reopen)
        ));
    }
    text.push_str(
        "\n### Claims\n\n| Claim | Series | Status | Limitation |\n| --- | --- | --- | --- |\n",
    );
    for claim in object
        .get("claims")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("claims is not an array"))?
    {
        text.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            escape(claim.get("id").and_then(Value::as_str).unwrap_or("unknown")),
            escape(
                claim
                    .get("series")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            ),
            escape(
                claim
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            ),
            escape(claim.get("reason").and_then(Value::as_str).unwrap_or(""))
        ));
    }
    Ok(text)
}

pub(super) fn limitations(cell: &Value) -> String {
    let mut parts = cell
        .get("missing")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(rows) = cell.get("censored").and_then(Value::as_array) {
        parts.extend(rows.iter().map(|row| {
            format!(
                ">={} {}",
                row.get("lower_bound")
                    .map(display)
                    .unwrap_or_else(|| "unavailable".to_owned()),
                row.get("unit").and_then(Value::as_str).unwrap_or("")
            )
        }));
    }
    if parts.is_empty() {
        "none".to_owned()
    } else {
        parts.join("; ")
    }
}

pub(super) fn shape_display(cell: &Value) -> String {
    let dip = cell
        .get("dip")
        .map(display)
        .unwrap_or_else(|| "unavailable".to_owned());
    let Some(shape) = cell.get("shape") else {
        return format!("dip={dip}; shape unavailable");
    };
    if shape.get("modes").is_none()
        || shape.get("status").and_then(Value::as_str) == Some("unavailable")
    {
        return format!(
            "dip={dip}; {}",
            shape
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("shape unavailable")
        );
    }
    format!(
        "dip={dip}; dip_p=unavailable; modes={} cuts={} shares={} support={}",
        shape
            .get("modes")
            .map(display)
            .unwrap_or_else(|| "[]".to_owned()),
        shape
            .get("antimodes")
            .map(display)
            .unwrap_or_else(|| "[]".to_owned()),
        shape
            .get("shares")
            .map(display)
            .unwrap_or_else(|| "[]".to_owned()),
        shape
            .get("support")
            .map(display)
            .unwrap_or_else(|| "[]".to_owned())
    )
}

pub(super) fn quantile(cell: &Value, point: &str, interval: &str) -> String {
    let estimate = cell
        .get(point)
        .map(display)
        .unwrap_or_else(|| "unavailable".to_owned());
    let Some(interval) = cell.get(interval) else {
        return estimate;
    };
    let bca = interval.get("bca");
    let low = bca.and_then(|value| value.get("low")).map(display);
    let high = bca.and_then(|value| value.get("high")).map(display);
    match (low, high) {
        (Some(low), Some(high)) => format!(
            "{estimate} [{low}, {high}] ({})",
            interval
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unavailable")
        ),
        _ => format!(
            "{estimate} ({})",
            interval
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unavailable")
        ),
    }
}

pub(super) fn display(value: &Value) -> String {
    match value {
        Value::Null => "unavailable".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => display_number(value),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

pub(super) fn display_number(value: &serde_json::Number) -> String {
    let Some(number) = value.as_f64() else {
        return value.to_string();
    };
    if number == 0.0 {
        return "0".to_owned();
    }
    let magnitude = number.abs();
    if magnitude < 0.000001 {
        return format!("{number:.6e}");
    }
    if magnitude >= 1e15 {
        return value.to_string();
    }
    let mut text = format!("{number:.6}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" { "0".to_owned() } else { text }
}

pub(super) fn escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

pub(super) fn invalid(subject: &str) -> BenchError {
    BenchError::input(
        "render benchmark report",
        subject.to_owned(),
        "regenerate exports.json with the matching analyzer revision",
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::display_number;

    #[test]
    fn report_numbers_are_readable_without_losing_small_nonzero_values() {
        let ordinary = serde_json::Number::from_f64(76.04910000000001).unwrap();
        assert_eq!(display_number(&ordinary), "76.0491");
        let tiny = serde_json::Number::from_f64(0.0000004).unwrap();
        assert_ne!(display_number(&tiny), "0");
        assert!(display_number(&tiny).contains('e'));
    }
}

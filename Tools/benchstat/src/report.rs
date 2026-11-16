// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Canonical exports and the generated Markdown section.

use serde_json::Value;

use crate::error::BenchError;

mod sections;

use sections::{
    analysis_provenance, coverage_and_claims, decomposition_section, display, escape, invalid,
    limitations, quantile, shape_display,
};

/// Opening marker of the one generated README section.
pub const BEGIN: &str = "<!-- benchstat:begin -->";
/// Closing marker of the one generated README section.
pub const END: &str = "<!-- benchstat:end -->";

/// Serialize sorted JSON with a final LF and no timestamp.
///
/// # Errors
/// Returns an internal error if the already validated value cannot be serialized.
pub fn canonical(value: &Value) -> Result<String, BenchError> {
    let mut text = serde_json::to_string_pretty(value).map_err(|error| {
        BenchError::numeric(
            "serialize exports.json",
            error.to_string(),
            "remove the non-finite or unsupported export value",
        )
    })?;
    text.push('\n');
    Ok(text)
}

/// Render only values present in exports, never reopening raw observations.
///
/// # Errors
/// Rejects an exports document whose top-level report fields have the wrong shape.
pub fn markdown(exports: &Value) -> Result<String, BenchError> {
    let object = exports
        .as_object()
        .ok_or_else(|| invalid("exports root is not an object"))?;
    let mut text = String::from("## Generated benchmark analysis\n\n");
    text.push_str("The JSON export is the machine-readable authority for every number below. Unavailable analyses and deferred coverage remain explicit.\n\n");
    text.push_str(&analysis_provenance(object)?);
    text.push_str(&coverage_and_claims(object)?);
    text.push_str("\n### Quantitative cells\n\n| Cell | n | Missing / censored | P50 (interval status) | P95 (interval status) | P99 (interval status) | P99 publishable | Drift | Shape (descriptive) |\n| --- | ---: | --- | ---: | ---: | ---: | --- | --- | --- |\n");
    for (id, cell) in object
        .get("cells")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("cells is not an object"))?
    {
        if cell.get("successes").is_some() {
            continue;
        }
        let label = cell.get("label").and_then(Value::as_str).unwrap_or(id);
        let n = cell
            .get("n")
            .map(display)
            .unwrap_or_else(|| "unavailable".to_owned());
        let limitations = limitations(cell);
        let p50 = quantile(cell, "p50", "p50_ci");
        let p95 = quantile(cell, "p95", "p95_ci");
        let p99 = quantile(cell, "p99", "p99_ci");
        let publishable = cell
            .get("p99_publishable")
            .and_then(|value| value.get("value"))
            .map(display)
            .unwrap_or_else(|| "unavailable".to_owned());
        let drift = cell
            .get("pettitt_p_adjusted")
            .map(display)
            .unwrap_or_else(|| "unavailable".to_owned());
        let shape = shape_display(cell);
        text.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            escape(label),
            n,
            limitations,
            p50,
            p95,
            p99,
            publishable,
            drift,
            shape
        ));
    }
    text.push_str("\n### Outcome summaries\n\n| Cell | Successes / eligible | Planned / attempted | Not attempted | Guard | Timeout | Infrastructure excluded | Interval status | Contrast |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |\n");
    for (id, cell) in object
        .get("outcome_summaries")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("cells is not an object"))?
    {
        if cell.get("successes").is_none() {
            continue;
        }
        let label = cell.get("label").and_then(Value::as_str).unwrap_or(id);
        let successes = cell
            .get("successes")
            .map(display)
            .unwrap_or_else(|| "0".to_owned());
        let eligible = cell
            .get("eligible")
            .map(display)
            .unwrap_or_else(|| "0".to_owned());
        let planned = cell
            .get("planned")
            .map(display)
            .unwrap_or_else(|| "0".to_owned());
        let attempted = cell
            .get("attempted")
            .map(display)
            .unwrap_or_else(|| "0".to_owned());
        let not_attempted = cell
            .get("not_attempted")
            .map(display)
            .unwrap_or_else(|| "0".to_owned());
        let guard = cell
            .get("guard")
            .map(display)
            .unwrap_or_else(|| "0".to_owned());
        let timeout = cell
            .get("timeout")
            .map(display)
            .unwrap_or_else(|| "0".to_owned());
        let infrastructure = cell
            .get("infrastructure_excluded")
            .map(display)
            .unwrap_or_else(|| "0".to_owned());
        let interval = cell
            .get("wilson_ci")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("unavailable");
        let contrast = cell.get("contrast").map_or_else(
            || "unavailable".to_owned(),
            |contrast| {
                format!(
                    "{} [{}, {}] ({})",
                    contrast
                        .get("risk_difference")
                        .map(display)
                        .unwrap_or_else(|| "unavailable".to_owned()),
                    contrast
                        .get("low")
                        .map(display)
                        .unwrap_or_else(|| "unavailable".to_owned()),
                    contrast
                        .get("high")
                        .map(display)
                        .unwrap_or_else(|| "unavailable".to_owned()),
                    contrast
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unavailable")
                )
            },
        );
        text.push_str(&format!(
            "| {} | {} / {} | {} / {} | {} | {} | {} | {} | {} | {} |\n",
            escape(label),
            successes,
            eligible,
            planned,
            attempted,
            not_attempted,
            guard,
            timeout,
            infrastructure,
            interval,
            contrast
        ));
    }
    text.push_str(&decomposition_section(object)?);
    text.push_str("\n### Memory fits\n\n| Cell | Slope | Intercept | R² | Slope interval |\n| --- | ---: | ---: | ---: | --- |\n");
    for (id, fitted) in object
        .get("memory_fits")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("memory_fits is not an object"))?
    {
        text.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            escape(id),
            fitted
                .get("slope")
                .map(display)
                .unwrap_or_else(|| "unavailable".to_owned()),
            fitted
                .get("intercept")
                .map(display)
                .unwrap_or_else(|| "unavailable".to_owned()),
            fitted
                .get("r_squared")
                .map(display)
                .unwrap_or_else(|| "unavailable".to_owned()),
            fitted
                .get("slope_ci")
                .map(|interval| format!(
                    "[{}, {}] ({})",
                    interval
                        .get("low")
                        .map(display)
                        .unwrap_or_else(|| "unavailable".to_owned()),
                    interval
                        .get("high")
                        .map(display)
                        .unwrap_or_else(|| "unavailable".to_owned()),
                    interval
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unavailable")
                ))
                .unwrap_or_else(|| "unavailable".to_owned())
        ));
    }
    text.push_str("\nMemory sizing (target, achieved half-width and precision status):\n\n");
    for (id, fitted) in object
        .get("memory_fits")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("memory_fits is not an object"))?
    {
        if let Some(sizing) = fitted.get("sizing") {
            text.push_str(&format!(
                "- `{id}`: status={}; precision_met={}; target_half_width={}; achieved_half_width={}\n",
                sizing.get("status").map(display).unwrap_or_else(|| "unavailable".to_owned()),
                sizing.get("precision_met").map(display).unwrap_or_else(|| "unavailable".to_owned()),
                sizing.get("target_half_width").map(display).unwrap_or_else(|| "unavailable".to_owned()),
                sizing.get("achieved_half_width").map(display).unwrap_or_else(|| "unavailable".to_owned())
            ));
        }
    }
    Ok(text)
}

/// Replace exactly one generated section, preserving all prose around it.
///
/// # Errors
/// Rejects missing, duplicated or reversed markers.
pub fn replace_generated(readme: &str, generated: &str) -> Result<String, BenchError> {
    if readme.matches(BEGIN).count() != 1 || readme.matches(END).count() != 1 {
        return Err(BenchError::input(
            "read README.md generated markers",
            "expected exactly one begin and one end marker".to_owned(),
            "add `<!-- benchstat:begin -->` and `<!-- benchstat:end -->` once each",
        ));
    }
    let (before, remainder) = readme
        .split_once(BEGIN)
        .ok_or_else(|| invalid("missing begin marker"))?;
    let (_, after) = remainder
        .split_once(END)
        .ok_or_else(|| invalid("missing end marker"))?;
    Ok(format!("{before}{BEGIN}\n{generated}{END}{after}"))
}

/// Extract the generated section for byte comparison without treating prose as generated.
///
/// # Errors
/// Rejects the same malformed marker layout as [`replace_generated`].
pub fn generated_section(readme: &str) -> Result<String, BenchError> {
    replace_generated(readme, "")?;
    let (_, remainder) = readme
        .split_once(BEGIN)
        .ok_or_else(|| invalid("missing begin marker"))?;
    let (body, _) = remainder
        .split_once(END)
        .ok_or_else(|| invalid("missing end marker"))?;
    Ok(body.to_owned())
}

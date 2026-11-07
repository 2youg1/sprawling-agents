// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Convert the historical v0.0.11 subset into a local-only long-form round.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{Path, PathBuf};

use benchstat::observation::HEADER;
use serde_json::json;
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let source = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing historical round directory")?;
    let destination = args
        .next()
        .map(PathBuf::from)
        .ok_or("missing local output directory")?;
    let revision = args.next().ok_or("missing analyzer Git SHA")?;
    if args.next().is_some() {
        return Err("usage: convert_v0011 <source> <local-output> <analyzer-git-sha>".into());
    }
    convert(&source, &destination, &revision)
}

fn convert(source: &Path, destination: &Path, revision: &str) -> Result<(), Box<dyn Error>> {
    let mut reader = csv::Reader::from_path(source.join("results.csv"))?;
    let headers = reader.headers()?.clone();
    let mut rows = Vec::<Vec<String>>::new();
    for record in reader.records() {
        let record = record?;
        let old = headers
            .iter()
            .zip(record.iter())
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect::<BTreeMap<_, _>>();
        expand(&old, &mut rows)?;
    }
    for (index, row) in rows.iter_mut().enumerate() {
        set(row, 1, &format!("o{index}"))?;
    }
    std::fs::create_dir_all(destination)?;
    let csv_path = destination.join("results.csv");
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::Any(b'\n'))
        .from_path(&csv_path)?;
    writer.write_record(HEADER)?;
    for row in rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    let bytes = std::fs::read(&csv_path)?;
    let digest = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let present = present_series(&csv_path)?;
    let registry = [
        "startup",
        "first_pairing",
        "cold_boot_gui",
        "browser_reload",
        "local_tool",
        "request_latency",
        "task_wall",
        "task_requests",
        "task_tokens",
        "task_outcomes",
        "session_memory",
        "sustained_memory",
        "real_model_capacity",
        "task_io",
        "monitor_beat",
        "view_backlog",
    ];
    let coverage = registry
        .iter()
        .map(|series| {
            if present.contains(*series) {
                json!({"series": series, "status": "measured", "reason": null, "reopen_when": null, "measured_by": "v0.0.11 historical instruments", "reading": "results.csv"})
            } else {
                json!({"series": series, "status": "deferred", "reason": "the historical subset did not collect this series", "reopen_when": "a future round collects the series under the new method", "measured_by": null, "reading": null})
            }
        })
        .collect::<Vec<_>>();
    let manifest = json!({
        "schema_version": 1,
        "analysis_revision": revision.to_owned(),
        "method_version": "1",
        "estimator_version": "1",
        "seed_string": "v0.0.11-local-cross-validation",
        "resamples": 10000,
        "confidence": 0.95,
        "alpha": 0.05,
        "power": 0.8,
        "bootstrap_unit": "cohort",
        "input_hashes": {"results.csv": digest},
        "coverage": coverage,
        "claims": [],
        "cell_plans": [],
        "provenance": "Local-only conversion of the unchanged historical v0.0.11 subset; not a new published round",
        "limitations": "No pre-registered cell plans, request spans, raw memory time series or dependent-null modality calibration; failure classes are inferred from historical fields"
    });
    std::fs::write(
        destination.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    std::fs::write(
        destination.join("README.md"),
        "# Local v0.0.11 cross-validation\n\nThis is an unranked, local-only conversion of the historical subset. It is not a new published round. Missing pre-registration, request spans, raw memory series and dependent-null calibration remain visible.\n\n<!-- benchstat:begin -->\npending\n<!-- benchstat:end -->\n",
    )?;
    Ok(())
}

fn expand(
    old: &BTreeMap<String, String>,
    rows: &mut Vec<Vec<String>>,
) -> Result<(), Box<dyn Error>> {
    let series = field(old, "series")?;
    if matches!(
        series,
        "help"
            | "project_first_answer"
            | "browser_reload"
            | "first_read"
            | "first_create"
            | "ax_following"
    ) {
        let mapped = match series {
            "help" | "project_first_answer" => "startup",
            "browser_reload" => "browser_reload",
            _ => "local_tool",
        };
        let values: Vec<f64> = serde_json::from_str(field(old, "values_json")?)?;
        for (index, value) in values.iter().enumerate() {
            let case = if series == "ax_following" {
                format!("{}_steady", field(old, "case")?)
            } else {
                field(old, "case")?.to_owned()
            };
            rows.push(base(
                old,
                mapped,
                "latency",
                field(old, "unit")?,
                &value.to_string(),
                &case,
                index,
            )?);
        }
        if series == "ax_following" && !field(old, "first")?.is_empty() {
            rows.push(base(
                old,
                mapped,
                "latency",
                field(old, "unit")?,
                field(old, "first")?,
                &format!("{}_first", field(old, "case")?),
                0,
            )?);
        }
        return Ok(());
    }
    if series == "local_ram" {
        rows.push(base(
            old,
            "session_memory",
            "memory_private",
            field(old, "unit")?,
            field(old, "p50")?,
            field(old, "case")?,
            0,
        )?);
        return Ok(());
    }
    expand_task(old, rows)
}

fn expand_task(
    old: &BTreeMap<String, String>,
    rows: &mut Vec<Vec<String>>,
) -> Result<(), Box<dyn Error>> {
    let series = field(old, "series")?;
    let deadline = if series == "gpt_coding" { 300.0 } else { 180.0 };
    let wall = field(old, "wall_seconds")?.parse::<f64>()?;
    let normal = field(old, "normal_completion")?;
    let failure = if normal == "0" && wall >= deadline {
        "timeout"
    } else if normal == "1" {
        "none"
    } else {
        "capability"
    };
    let identity = serde_json::to_vec(old)?;
    let attempt = format!(
        "{}-{}-{}-{}",
        series,
        field(old, "agent")?,
        field(old, "case")?,
        Sha256::digest(&identity)
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    rows.push(task(
        old,
        "task_wall",
        "wall",
        "s",
        field(old, "wall_seconds")?,
        failure,
        "",
        &attempt,
        true,
    )?);
    rows.push(task(
        old,
        "task_requests",
        "requests",
        "requests",
        field(old, "model_requests")?,
        failure,
        "",
        &attempt,
        false,
    )?);
    for (metric, field_name) in [
        ("functional", "functional_passed"),
        ("contract", "contract_passed"),
        ("normal_completion", "normal_completion"),
    ] {
        let raw = field(old, field_name)?;
        let (value, missing) = if raw.is_empty() {
            ("0", "historical field was not evaluated")
        } else {
            (raw, "")
        };
        rows.push(task(
            old,
            "task_outcomes",
            metric,
            "ratio",
            value,
            failure,
            missing,
            &attempt,
            false,
        )?);
    }
    for (metric, field_name) in [
        ("input_tokens", "known_input_tokens"),
        ("output_tokens", "known_output_tokens"),
    ] {
        let value = field(old, field_name)?;
        if !value.is_empty() {
            rows.push(task(
                old,
                "task_tokens",
                metric,
                "tokens",
                value,
                failure,
                "",
                &attempt,
                false,
            )?);
        }
    }
    for (metric, field_name) in [("read_bytes", "read_bytes"), ("write_bytes", "write_bytes")] {
        let value = field(old, field_name)?;
        if !value.is_empty() {
            rows.push(task(
                old, "task_io", metric, "bytes", value, failure, "", &attempt, false,
            )?);
        }
    }
    Ok(())
}

fn base(
    old: &BTreeMap<String, String>,
    series: &str,
    metric: &str,
    unit: &str,
    value: &str,
    case: &str,
    index: usize,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut row = vec![String::new(); HEADER.len()];
    set(&mut row, 0, "1")?;
    set(&mut row, 6, "v0.0.11-local")?;
    set(&mut row, 7, "v0.0.11")?;
    set(&mut row, 8, "historical")?;
    set(&mut row, 9, field(old, "agent")?)?;
    set(&mut row, 10, series)?;
    set(&mut row, 11, field(old, "series")?)?;
    set(&mut row, 12, case)?;
    set(&mut row, 13, field(old, "cohort")?)?;
    set(&mut row, 14, field(old, "state")?)?;
    set(&mut row, 15, field(old, "sessions")?)?;
    set(&mut row, 16, metric)?;
    set(&mut row, 17, unit)?;
    if series != "session_memory" {
        set(&mut row, 18, &index.to_string())?;
    }
    set(&mut row, 19, value)?;
    set(&mut row, 20, "none")?;
    set(&mut row, 22, "0")?;
    set(&mut row, 23, "1")?;
    set(&mut row, 24, field(old, "selected")?)?;
    set(&mut row, 26, "v0.0.11 historical instrument")?;
    set(&mut row, 27, field(old, "condition")?)?;
    Ok(row)
}

#[expect(
    clippy::too_many_arguments,
    reason = "example-only conversion keeps each historical field explicit at its call site"
)]
fn task(
    old: &BTreeMap<String, String>,
    series: &str,
    metric: &str,
    unit: &str,
    value: &str,
    failure: &str,
    missing: &str,
    attempt: &str,
    wall: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut row = base(old, series, metric, unit, value, field(old, "case")?, 0)?;
    set(&mut row, 2, attempt)?;
    if wall {
        set(&mut row, 4, "0")?;
        set(&mut row, 5, field(old, "wall_seconds")?)?;
        if failure == "timeout" {
            set(&mut row, 22, "1")?;
        }
    }
    set(&mut row, 20, failure)?;
    set(&mut row, 21, missing)?;
    set(&mut row, 23, field(old, "usage_complete")?)?;
    Ok(row)
}

fn present_series(csv_path: &Path) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let mut reader = csv::Reader::from_path(csv_path)?;
    let headers = reader.headers()?.clone();
    let index = headers
        .iter()
        .position(|name| name == "series")
        .ok_or("series column missing")?;
    let mut series = BTreeSet::new();
    for row in reader.records() {
        if let Some(value) = row?.get(index) {
            series.insert(value.to_owned());
        }
    }
    Ok(series)
}

fn field<'a>(row: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str, Box<dyn Error>> {
    row.get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("historical column {name} missing").into())
}

fn set(row: &mut [String], index: usize, value: &str) -> Result<(), Box<dyn Error>> {
    *row.get_mut(index).ok_or("long-form column out of bounds")? = value.to_owned();
    Ok(())
}

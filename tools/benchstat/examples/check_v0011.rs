// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Independently compare a local reanalysis with historical raw observations.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use benchstat::error::{BenchError, Severity};
use benchstat::estimators::{Quantile, nearest_rank, range};
use benchstat::observation::CellKey;
use serde_json::Value;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let source = args.next().ok_or("missing historical round directory")?;
    let exports = args.next().ok_or("missing local exports.json")?;
    if args.next().is_some() {
        return Err("usage: check_v0011 <historical-round> <local-exports.json>".into());
    }
    check(Path::new(&source), Path::new(&exports))
}

fn check(source: &Path, exports_path: &Path) -> Result<(), Box<dyn Error>> {
    let exports: Value = serde_json::from_slice(&std::fs::read(exports_path)?)?;
    let cells = exports
        .get("cells")
        .and_then(Value::as_object)
        .ok_or_else(|| BenchError {
            code: "E_INVALID_INPUT",
            action: "read exported cells".to_owned(),
            subject: format!(
                "{}: cells is missing or not an object",
                exports_path.display()
            ),
            recovery:
                "regenerate exports.json with `benchstat export <round-dir> --write`, then retry"
                    .to_owned(),
            severity: Severity::Rejected,
        })?;
    let mut reader = csv::Reader::from_path(source.join("results.csv"))?;
    let headers = reader.headers()?.clone();
    let mut pooled = BTreeMap::<String, Vec<f64>>::new();
    let mut published_rows = 0_u64;
    for record in reader.records() {
        let record = record?;
        let old = headers
            .iter()
            .zip(record.iter())
            .collect::<BTreeMap<_, _>>();
        let series = field(&old, "series")?;
        if !matches!(
            series,
            "help"
                | "project_first_answer"
                | "browser_reload"
                | "first_read"
                | "first_create"
                | "ax_following"
        ) {
            continue;
        }
        let mapped = match series {
            "help" | "project_first_answer" => "startup",
            "browser_reload" => "browser_reload",
            _ => "local_tool",
        };
        let case = if series == "ax_following" {
            format!("{}_steady", field(&old, "case")?)
        } else {
            field(&old, "case")?.to_owned()
        };
        let key = CellKey {
            build: "v0.0.11".to_owned(),
            arm: "historical".to_owned(),
            agent: field(&old, "agent")?.to_owned(),
            series: mapped.to_owned(),
            workload: series.to_owned(),
            case,
            state: field(&old, "state")?.to_owned(),
            sessions: if field(&old, "sessions")?.is_empty() {
                None
            } else {
                Some(field(&old, "sessions")?.parse::<f64>()?.to_string())
            },
            metric: "latency".to_owned(),
            unit: field(&old, "unit")?.to_owned(),
            counter_source: "v0.0.11 historical instrument".to_owned(),
            condition: field(&old, "condition")?.to_owned(),
        };
        let label = key.id();
        let values: Vec<f64> = serde_json::from_str(field(&old, "values_json")?)?;
        let mut sorted = values.clone();
        sorted.sort_by(f64::total_cmp);
        compare_number(
            &label,
            "published n",
            Some(field(&old, "n")?.parse::<f64>()?),
            f64::from(u32::try_from(sorted.len())?),
        )?;
        compare_number(
            &label,
            "published P50",
            Some(field(&old, "p50")?.parse::<f64>()?),
            nearest_rank(&sorted, Quantile::P50).ok_or("empty historical cell")?,
        )?;
        compare_number(
            &label,
            "published maximum",
            Some(field(&old, "maximum")?.parse::<f64>()?),
            range(&sorted).ok_or("empty historical cell")?.1,
        )?;
        if !field(&old, "p99")?.is_empty() {
            compare_number(
                &label,
                "published P99",
                Some(field(&old, "p99")?.parse::<f64>()?),
                nearest_rank(&sorted, Quantile::P99).ok_or("empty historical cell")?,
            )?;
        }
        pooled.entry(label).or_default().extend(values);
        published_rows = published_rows
            .checked_add(1)
            .ok_or("comparison count overflow")?;
    }
    if published_rows != 81 {
        return Err(format!("expected 81 historical rows, found {published_rows}").into());
    }
    let mut pooled_cells = 0_u64;
    for (label, values) in pooled {
        let cell = cells
            .get(label.as_str())
            .ok_or_else(|| format!("no exported cell for {label}"))?;
        let mut sorted = values;
        sorted.sort_by(f64::total_cmp);
        compare(
            &label,
            "pooled n",
            cell.get("n"),
            f64::from(u32::try_from(sorted.len())?),
        )?;
        compare(
            &label,
            "pooled P50",
            cell.get("p50"),
            nearest_rank(&sorted, Quantile::P50).ok_or("empty pooled cell")?,
        )?;
        compare(
            &label,
            "pooled P95",
            cell.get("p95"),
            nearest_rank(&sorted, Quantile::P95).ok_or("empty pooled cell")?,
        )?;
        compare(
            &label,
            "pooled P99",
            cell.get("p99"),
            nearest_rank(&sorted, Quantile::P99).ok_or("empty pooled cell")?,
        )?;
        compare(
            &label,
            "pooled maximum",
            cell.get("max"),
            range(&sorted).ok_or("empty pooled cell")?.1,
        )?;
        pooled_cells = pooled_cells.checked_add(1).ok_or("pooled count overflow")?;
    }

    let outcomes = exports
        .get("outcome_summaries")
        .and_then(Value::as_object)
        .ok_or("outcome summaries missing")?
        .len();
    println!(
        "historical comparison: {published_rows} published rows and {pooled_cells} independently pooled cells match; {outcomes} outcome summaries present"
    );
    Ok(())
}

fn compare(
    label: &str,
    field_name: &str,
    actual: Option<&Value>,
    expected: f64,
) -> Result<(), Box<dyn Error>> {
    compare_number(label, field_name, actual.and_then(Value::as_f64), expected)
}

fn compare_number(
    label: &str,
    field_name: &str,
    actual: Option<f64>,
    expected: f64,
) -> Result<(), Box<dyn Error>> {
    let actual = actual.ok_or_else(|| format!("{label} has no numeric {field_name}"))?;
    let tolerance = 1e-9_f64.max(expected.abs() * 1e-12);
    if (actual - expected).abs() > tolerance {
        return Err(format!("{label} {field_name}: expected {expected}, found {actual}").into());
    }
    Ok(())
}

fn field<'a>(row: &BTreeMap<&str, &'a str>, name: &str) -> Result<&'a str, Box<dyn Error>> {
    row.get(name)
        .copied()
        .ok_or_else(|| format!("historical column {name} missing").into())
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn invalid_cells_preserves_export_path_and_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("exports.json");
        for json in ["{}", r#"{"cells":null}"#, r#"{"cells":[]}"#] {
            std::fs::write(&path, json).unwrap();
            let error = check(dir.path(), &path).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!(
                    "E_INVALID_INPUT read exported cells: {}: cells is missing or not an object; regenerate exports.json with `benchstat export <round-dir> --write`, then retry",
                    path.display()
                )
            );
        }
    }
}

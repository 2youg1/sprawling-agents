// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::error::BenchError;
use crate::estimators::{Quantile, nearest_rank, range};
use crate::manifest::Manifest;
use crate::observation::Observation;
use crate::resampling::{Interval, SeedStream};

pub(super) fn segments(rows: &[Observation], ordered: bool) -> Vec<Vec<f64>> {
    if !ordered {
        return vec![rows.iter().filter_map(Observation::quantitative).collect()];
    }
    let mut groups = BTreeMap::<(&str, &str), Vec<f64>>::new();
    for row in rows {
        if let Some(value) = row.quantitative() {
            groups
                .entry((&row.round, &row.cohort))
                .or_default()
                .push(value);
        }
    }
    groups.into_values().collect()
}

pub(super) fn cohort_range(rows: &[Observation]) -> Result<Option<f64>, BenchError> {
    let mut groups = BTreeMap::<String, Vec<f64>>::new();
    for row in rows {
        if let Some(value) = row.quantitative() {
            groups.entry(row.cohort.clone()).or_default().push(value);
        }
    }
    let medians = groups
        .into_values()
        .map(|mut values| {
            values.sort_by(f64::total_cmp);
            nearest_rank(&values, Quantile::P50)
        })
        .collect::<Option<Vec<_>>>();
    let Some(medians) = medians else {
        return Ok(None);
    };
    let Some((low, high)) = range(&medians) else {
        return Ok(None);
    };
    let centre = nearest_rank(&medians, Quantile::P50).unwrap_or(0.0);
    if centre == 0.0 {
        Ok(None)
    } else {
        Ok(Some((high - low) / centre))
    }
}

pub(super) fn interval_json(interval: Interval, confidence: f64) -> Result<Value, BenchError> {
    let bca = match interval.bca {
        Some((low, high)) => json!({"low": low, "high": high}),
        None => Value::Null,
    };
    Ok(json!({
        "estimator": "moving_block_bootstrap_bca_with_percentile_sensitivity",
        "confidence": confidence,
        "bca": bca,
        "percentile": {"low": interval.percentile.0, "high": interval.percentile.1},
        "status": interval.status,
        "reason": interval.reason,
        "block_lengths": interval.block_lengths,
        "usable_replicates": interval.usable_replicates,
    }))
}

pub(super) fn unavailable(reason: &str) -> Result<Value, BenchError> {
    Ok(json!({"status": "unavailable", "reason": reason}))
}

pub(super) fn stream(
    manifest: &Manifest,
    round: &str,
    cell: &str,
    statistic: &str,
) -> Result<SeedStream, BenchError> {
    SeedStream::new(&[&manifest.seed_string, round, cell, statistic])
}

pub(super) fn round_name(rows: &[Observation]) -> String {
    let mut rounds = rows
        .iter()
        .map(|row| row.round.as_str())
        .collect::<Vec<_>>();
    rounds.sort_unstable();
    rounds.dedup();
    rounds.join("+")
}

pub(super) fn number(value: f64) -> Result<Value, BenchError> {
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| numeric("export number", "non-finite estimate"))
}

pub(super) fn optional_number(value: Option<f64>) -> Result<Value, BenchError> {
    value.map_or(Ok(Value::Null), number)
}

pub(super) fn numbers(values: &[f64]) -> Result<Value, BenchError> {
    Ok(Value::Array(
        values
            .iter()
            .map(|value| number(*value))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

pub(super) fn numeric(action: &str, subject: &str) -> BenchError {
    BenchError::numeric(
        action,
        subject.to_owned(),
        "inspect the cell and its pre-registered analysis",
    )
}

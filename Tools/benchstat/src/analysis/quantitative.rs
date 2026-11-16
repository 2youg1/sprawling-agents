// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::error::BenchError;
use crate::estimators::{
    Quantile, coefficient_of_variation, cusum, nearest_rank, pettitt, range, split_half_ratio,
    tail_counts,
};
use crate::manifest::{CellPlan, Manifest};
use crate::observation::{CellKey, Observation};
use crate::resampling::{BlockBootstrap, ResamplingContract};
use crate::shape;

use super::memory::sizing;
use super::support::{
    cohort_range, interval_json, number, numbers, numeric, optional_number, segments, stream,
    unavailable,
};

pub(super) struct QuantitativeContext<'a> {
    pub(super) manifest: &'a Manifest,
    pub(super) plan: Option<&'a CellPlan>,
    pub(super) round: &'a str,
}

pub(super) fn decompositions(rows: &[Observation]) -> Result<Value, BenchError> {
    let mut attempts = BTreeMap::<&str, Vec<&Observation>>::new();
    for row in rows {
        if let Some(attempt) = row.task_attempt_id.as_deref() {
            attempts.entry(attempt).or_default().push(row);
        }
    }
    let mut output = Map::new();
    for (attempt, rows) in attempts {
        let wall = rows.iter().find(|row| row.metric == "wall" && row.selected);
        let Some(wall) = wall else {
            output.insert(attempt.to_owned(), unavailable("no task wall span")?);
            continue;
        };
        let spans = rows
            .iter()
            .filter(|row| {
                matches!(
                    row.metric.as_str(),
                    "model_latency" | "tool_latency" | "queue_latency"
                ) && row.selected
            })
            .collect::<Vec<_>>();
        if spans.is_empty() {
            output.insert(
                attempt.to_owned(),
                unavailable("no request spans; decomposition is unavailable")?,
            );
            continue;
        }
        let (Some(wall_start), Some(wall_end)) = (wall.span_start, wall.span_end) else {
            output.insert(
                attempt.to_owned(),
                unavailable("task wall endpoints are missing")?,
            );
            continue;
        };
        let wall_duration = wall_end - wall_start;
        let mut by_metric = Map::new();
        let mut all = Vec::new();
        for metric in ["model_latency", "tool_latency", "queue_latency"] {
            let intervals = spans
                .iter()
                .filter(|row| row.metric == metric)
                .map(|row| match (row.span_start, row.span_end) {
                    (Some(start), Some(end)) => Ok((start, end)),
                    _ => Err(BenchError::input(
                        "decompose task wall",
                        attempt.to_owned(),
                        "publish both endpoints for every request span",
                    )),
                })
                .collect::<Result<Vec<_>, BenchError>>()?;
            if intervals.is_empty() {
                continue;
            }
            let (covered, total) = span_coverage(&intervals)?;
            all.extend(intervals);
            by_metric.insert(
                metric.to_owned(),
                json!({"covered": covered, "overlap": total - covered}),
            );
        }
        let (covered, total) = span_coverage(&all)?;
        output.insert(
            attempt.to_owned(),
            json!({
                "status": if wall.censored { "censored" } else { "analyzed" },
                "estimator": "union_of_request_spans_with_uncovered_residual",
                "wall": wall_duration,
                "covered": covered,
                "uncovered": (wall_duration - covered).max(0.0),
                "overlap": total - covered,
                "by_metric": by_metric,
            }),
        );
    }
    Ok(Value::Object(output))
}

pub(super) fn span_coverage(spans: &[(f64, f64)]) -> Result<(f64, f64), BenchError> {
    let mut ordered = spans.to_vec();
    ordered.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.total_cmp(&right.1))
    });
    let total = ordered.iter().map(|(start, end)| end - start).sum::<f64>();
    let mut covered = 0.0;
    let mut current: Option<(f64, f64)> = None;
    for (start, end) in ordered {
        match current {
            Some((current_start, current_end)) if start <= current_end => {
                current = Some((current_start, current_end.max(end)));
            }
            _ => {
                if let Some((current_start, current_end)) = current {
                    covered += current_end - current_start;
                }
                current = Some((start, end));
            }
        }
    }
    if let Some((start, end)) = current {
        covered += end - start;
    }
    if !covered.is_finite() || !total.is_finite() {
        return Err(numeric("span coverage", "non-finite interval arithmetic"));
    }
    Ok((covered, total))
}

pub(super) fn quantitative_cell(
    key: &CellKey,
    rows: &[Observation],
    context: QuantitativeContext<'_>,
    drift: &mut Vec<(String, f64)>,
) -> Result<Value, BenchError> {
    let QuantitativeContext {
        manifest,
        plan,
        round,
    } = context;
    let id = key.id();
    let values = rows
        .iter()
        .filter_map(Observation::quantitative)
        .collect::<Vec<_>>();
    let mut sorted = values.clone();
    sorted.sort_by(f64::total_cmp);
    if sorted.is_empty() {
        return Ok(json!({
            "key": id,
            "label": key.label(),
            "status": "unavailable",
            "reason": "no selected, uncensored, non-missing quantitative observations",
            "missing": rows.iter().filter(|row| row.selected).filter_map(|row| row.effective_missing_reason()).collect::<Vec<_>>(),
        }));
    }
    let n = sorted.len();
    let (min, max) = range(&sorted).ok_or_else(|| numeric("cell range", &id))?;
    let ordered = rows.iter().all(|row| row.index.is_some());
    let segments = segments(rows, ordered);
    let bootstrap = BlockBootstrap::new(segments, ordered)?;
    let mut output = Map::new();
    output.insert("key".to_owned(), json!(id));
    output.insert("label".to_owned(), json!(key.label()));
    output.insert("status".to_owned(), json!("analyzed"));
    output.insert("n".to_owned(), json!(n));
    output.insert(
        "censored".to_owned(),
        json!(
            rows.iter()
                .filter(|row| row.selected && row.censored)
                .map(|row| json!({
                    "task_attempt_id": row.task_attempt_id,
                    "lower_bound": row.value,
                    "unit": row.unit,
                }))
                .collect::<Vec<_>>()
        ),
    );
    output.insert(
        "missing".to_owned(),
        json!(
            rows.iter()
                .filter(|row| row.selected && row.effective_missing_reason().is_some())
                .map(|row| row.effective_missing_reason())
                .collect::<Vec<_>>()
        ),
    );
    output.insert("min".to_owned(), number(min)?);
    output.insert("max".to_owned(), number(max)?);
    output.insert("ecdf".to_owned(), numbers(&sorted)?);
    output.insert(
        "collection_order".to_owned(),
        json!(if ordered { "preserved" } else { "missing" }),
    );
    output.insert("block_lengths".to_owned(), json!(bootstrap.lengths()));
    for quantile in [Quantile::P50, Quantile::P95, Quantile::P99] {
        let estimate = nearest_rank(&sorted, quantile).ok_or_else(|| numeric("quantile", &id))?;
        output.insert(quantile.name().to_owned(), number(estimate)?);
        let statistic = format!("{}_ci", quantile.name());
        let mut stream = stream(manifest, round, &id, &statistic)?;
        let interval = bootstrap.quantile_interval(
            &values,
            quantile,
            ResamplingContract {
                resamples: manifest.resamples,
                confidence: manifest.confidence,
            },
            &mut stream,
        )?;
        output.insert(statistic, interval_json(interval, manifest.confidence)?);
    }
    output.insert(
        "cv".to_owned(),
        optional_number(coefficient_of_variation(&values))?,
    );
    if let Some((ratio, over2, over5)) = tail_counts(&values) {
        output.insert("max_over_p99".to_owned(), number(ratio)?);
        output.insert("over_2x_p99".to_owned(), json!(over2));
        output.insert("over_5x_p99".to_owned(), json!(over5));
    }
    output.insert(
        "split_half_ratio".to_owned(),
        optional_number(split_half_ratio(&values))?,
    );
    output.insert("cusum".to_owned(), optional_number(cusum(&values))?);
    output.insert(
        "cohort_range_over_median".to_owned(),
        optional_number(cohort_range(rows)?)?,
    );
    let dip = shape::dip(&values).map_err(|error| {
        BenchError::numeric(
            "compute dip for cell",
            format!("{id}: {error}"),
            "inspect the repeated values and hull indices in this cell",
        )
    })?;
    output.insert("dip".to_owned(), number(dip.statistic)?);
    output.insert(
        "dip_modal_interval".to_owned(),
        json!({"low": dip.low, "high": dip.high}),
    );
    output.insert(
        "dip_p".to_owned(),
        json!({
            "value": Value::Null,
            "status": "unavailable",
            "reason": "no validated dependent-null calibration is registered",
            "estimator": "dependent_unimodal_null_required",
        }),
    );
    if ordered {
        if let Some((statistic, change)) = pettitt(&values) {
            let mut stream = stream(manifest, round, &id, "pettitt")?;
            let p = bootstrap.pettitt_p(&values, manifest.resamples, &mut stream)?;
            output.insert("pettitt".to_owned(), number(statistic)?);
            output.insert("pettitt_change_index".to_owned(), json!(change));
            output.insert("pettitt_p".to_owned(), optional_number(p)?);
            if let Some(p) = p {
                drift.push((id.clone(), p));
            }
        } else {
            output.insert(
                "pettitt".to_owned(),
                unavailable("fewer than four ordered observations")?,
            );
        }
    } else {
        output.insert(
            "pettitt".to_owned(),
            unavailable("collection order was not preserved")?,
        );
    }
    if let Some(plan) = plan {
        output.insert("sizing".to_owned(), sizing(&sorted, plan, &output)?);
        let mut shape_stream = stream(manifest, round, &id, "kde_shape")?;
        let shape = shape::shape(&values, &bootstrap, manifest.resamples, &mut shape_stream)?;
        output.insert(
            "shape".to_owned(),
            json!({
                "status": if shape.modes.is_empty() { "unavailable" } else { "descriptive" },
                "reason": if shape.modes.is_empty() { Some("no original peak reaches 95% bootstrap support") } else { None },
                "original_modes": shape.original_modes,
                "estimator": "gaussian_kde_sheather_jones_ste_4097_scalar_fft",
                "bandwidth": shape.bandwidth,
                "grid": shape.grid,
                "density": shape.density,
                "modes": shape.modes,
                "antimodes": shape.antimodes,
                "shares": shape.shares,
                "support": shape.support,
                "unmatched": shape.unmatched,
                "intervals": Value::Null,
                "interval_status": "not_applicable_without_calibrated_multimodality",
            }),
        );
    } else {
        output.insert(
            "sizing".to_owned(),
            unavailable("no pre-registered cell plan")?,
        );
        output.insert(
            "shape".to_owned(),
            unavailable("no pre-registered shape analysis")?,
        );
    }
    let p99 = output.get("p99").cloned().unwrap_or(Value::Null);
    let p99_interval = output
        .get("p99_ci")
        .and_then(Value::as_object)
        .filter(|interval| interval.get("status").and_then(Value::as_str) == Some("bca"));
    let half_width = p99_interval
        .and_then(|interval| interval.get("bca"))
        .and_then(Value::as_object)
        .and_then(|interval| {
            let low = interval.get("low").and_then(Value::as_f64)?;
            let high = interval.get("high").and_then(Value::as_f64)?;
            Some((high - low) / 2.0)
        });
    let publishable = match (p99.as_f64(), half_width) {
        (Some(estimate), Some(width)) => n >= 200 && width <= 0.25 * estimate,
        _ => false,
    };
    output.insert("p99_publishable".to_owned(), json!({
        "value": publishable,
        "reason": if publishable { Value::Null } else { json!("requires n >= 200 and a finite BCa half-width no greater than 25% of P99") },
    }));
    Ok(Value::Object(output))
}

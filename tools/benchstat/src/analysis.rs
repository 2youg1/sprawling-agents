// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whole-round projection from validated observations to canonical export values.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

use crate::error::BenchError;
use crate::estimators::{
    PowerContract, Proportion, Quantile, coefficient_of_variation, cusum,
    exact_independent_required_n, fit, holm, nearest_rank, newcombe, pettitt, range,
    split_half_ratio, tail_counts, wilson,
};
use crate::manifest::{CellPlan, Headline, Manifest};
use crate::observation::{CellKey, FailureClass, Observation};
use crate::resampling::{BlockBootstrap, Interval, ResamplingContract, SeedStream, order_quantile};
use crate::shape;

/// Analyze every eligible cell in explicit key order.
///
/// # Errors
/// Returns an input or numerical error when a declared analysis cannot be computed.
pub fn analyze(manifest: &Manifest, rows: &[Observation]) -> Result<Value, BenchError> {
    let plans = manifest
        .cell_plans
        .iter()
        .map(|plan| (plan.key.clone(), plan))
        .collect::<BTreeMap<_, _>>();
    let mut grouped = BTreeMap::<CellKey, Vec<Observation>>::new();
    for row in rows {
        grouped
            .entry(CellKey::of(row))
            .or_default()
            .push(row.clone());
    }
    let mut cells = Map::new();
    let mut drift = Vec::<(String, f64)>::new();
    for (key, mut observations) in grouped {
        observations.sort_by(|left, right| {
            left.round
                .cmp(&right.round)
                .then_with(|| left.cohort.cmp(&right.cohort))
                .then_with(|| left.index.cmp(&right.index))
                .then_with(|| left.observation_id.cmp(&right.observation_id))
        });
        let id = key.id();
        let plan = plans.get(&id).copied();
        let value = if matches!(
            key.metric.as_str(),
            "functional" | "contract" | "normal_completion"
        ) {
            outcome_cell(
                &key,
                &observations,
                plan.is_some_and(|plan| plan.independent_unpaired),
                plan.map(|plan| (manifest, plan)),
            )?
        } else {
            quantitative_cell(
                &key,
                &observations,
                QuantitativeContext {
                    manifest,
                    plan,
                    round: &round_name(&observations),
                },
                &mut drift,
            )?
        };
        cells.insert(id, value);
    }
    let drift_adjusted = holm(&drift.iter().map(|(_, p)| *p).collect::<Vec<_>>());
    for ((id, _), adjusted) in drift.iter().zip(drift_adjusted) {
        if let Some(cell) = cells.get_mut(id).and_then(Value::as_object_mut) {
            cell.insert("pettitt_p_adjusted".to_owned(), number(adjusted)?);
        }
    }
    let claims = manifest
        .claims
        .iter()
        .map(|claim| {
            let measured = manifest
                .coverage
                .iter()
                .any(|row| row.series == claim.series && row.status == "measured");
            Ok(json!({
                "id": claim.id,
                "series": claim.series,
                "status": if measured { "unavailable" } else { "deferred" },
                "primary_metric": claim.primary_metric,
                "direction": claim.direction,
                "minimum_worthwhile_change": claim.minimum_worthwhile_change,
                "falsification_condition": claim.falsification_condition,
                "evidence": rows.iter().filter(|row| row.series == claim.series
                    && row.metric == claim.primary_metric && row.unit == claim.unit
                    && row.quantitative().is_some()).map(|row| json!({
                        "observation_id": row.observation_id, "cell": CellKey::of(row).id(), "value": row.value
                    })).collect::<Vec<_>>(),
                "reason": if !measured { "required coverage is not measured" }
                    else if !rows.iter().any(|row| row.series == claim.series && row.metric == claim.primary_metric
                        && row.unit == claim.unit && row.quantitative().is_some()) { "no eligible primary metric observations in the registered unit" }
                    else { "claim has no executable cell contrast or falsification rule; free-text conditions require explicit evaluation" },
                "estimator": "pre_registered_claim_ladder",
            }))
        })
        .collect::<Result<Vec<_>, BenchError>>()?;
    let memory_fits = memory_fits(manifest, rows)?;
    let outcome_summaries = outcome_summaries(manifest, rows)?;
    let task_decompositions = decompositions(rows)?;
    Ok(json!({
        "schema_version": 1,
        "analysis_revision": manifest.analysis_revision,
        "method_version": manifest.method_version,
        "estimator_version": manifest.estimator_version,
        "parameters": {
            "resamples": manifest.resamples,
            "confidence": manifest.confidence,
            "alpha": manifest.alpha,
            "power": manifest.power,
            "bootstrap_unit": manifest.bootstrap_unit,
            "seed_string": manifest.seed_string,
        },
        "coverage": manifest.coverage,
        "claims": claims,
        "cells": cells,
        "memory_fits": memory_fits,
        "outcome_summaries": outcome_summaries,
        "task_decompositions": task_decompositions,
    }))
}

struct QuantitativeContext<'a> {
    manifest: &'a Manifest,
    plan: Option<&'a CellPlan>,
    round: &'a str,
}

fn decompositions(rows: &[Observation]) -> Result<Value, BenchError> {
    let attempts = rows
        .iter()
        .filter_map(|row| row.task_attempt_id.clone())
        .collect::<BTreeSet<_>>();
    let mut output = Map::new();
    for attempt in attempts {
        let wall = rows.iter().find(|row| {
            row.task_attempt_id.as_deref() == Some(&attempt) && row.metric == "wall" && row.selected
        });
        let Some(wall) = wall else {
            output.insert(attempt, unavailable("no task wall span")?);
            continue;
        };
        let spans = rows
            .iter()
            .filter(|row| {
                row.task_attempt_id.as_deref() == Some(&attempt)
                    && matches!(
                        row.metric.as_str(),
                        "model_latency" | "tool_latency" | "queue_latency"
                    )
                    && row.selected
            })
            .collect::<Vec<_>>();
        if spans.is_empty() {
            output.insert(
                attempt,
                unavailable("no request spans; decomposition is unavailable")?,
            );
            continue;
        }
        let (Some(wall_start), Some(wall_end)) = (wall.span_start, wall.span_end) else {
            output.insert(attempt, unavailable("task wall endpoints are missing")?);
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
                        attempt.clone(),
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
            attempt,
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

fn span_coverage(spans: &[(f64, f64)]) -> Result<(f64, f64), BenchError> {
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

fn quantitative_cell(
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
    if let Some(cv) = coefficient_of_variation(&values) {
        output.insert("cv".to_owned(), number(cv)?);
    } else {
        output.insert("cv".to_owned(), Value::Null);
    }
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

fn outcome_cell(
    key: &CellKey,
    rows: &[Observation],
    independent: bool,
    sizing: Option<(&Manifest, &CellPlan)>,
) -> Result<Value, BenchError> {
    let mut successes = 0_u64;
    let mut eligible = 0_u64;
    let mut missing = 0_u64;
    let mut infrastructure = 0_u64;
    let mut planned = 0_u64;
    let mut attempted = 0_u64;
    let mut not_attempted_count = 0_u64;
    let mut guard = 0_u64;
    let mut timeout = 0_u64;
    for row in rows.iter().filter(|row| row.selected) {
        planned = planned
            .checked_add(1)
            .ok_or_else(|| numeric("outcome count", "planned overflow"))?;
        if not_attempted(row.missing_reason.as_deref()) {
            not_attempted_count = not_attempted_count
                .checked_add(1)
                .ok_or_else(|| numeric("outcome count", "not-attempted overflow"))?;
            missing = missing
                .checked_add(1)
                .ok_or_else(|| numeric("outcome count", "missing overflow"))?;
            continue;
        }
        attempted = attempted
            .checked_add(1)
            .ok_or_else(|| numeric("outcome count", "attempted overflow"))?;
        if row.failure_class == FailureClass::Infrastructure {
            infrastructure = infrastructure
                .checked_add(1)
                .ok_or_else(|| numeric("outcome count", "infrastructure overflow"))?;
            continue;
        }
        match row.failure_class {
            FailureClass::Guard => {
                guard = guard
                    .checked_add(1)
                    .ok_or_else(|| numeric("outcome count", "guard overflow"))?;
            }
            FailureClass::Timeout => {
                timeout = timeout
                    .checked_add(1)
                    .ok_or_else(|| numeric("outcome count", "timeout overflow"))?;
            }
            FailureClass::None | FailureClass::Infrastructure | FailureClass::Capability => {}
        }
        if row.missing_reason.is_some() {
            missing = missing
                .checked_add(1)
                .ok_or_else(|| numeric("outcome count", "missing overflow"))?;
            continue;
        }
        eligible = eligible
            .checked_add(1)
            .ok_or_else(|| numeric("outcome count", "eligible overflow"))?;
        if row.value == 1.0 {
            successes = successes
                .checked_add(1)
                .ok_or_else(|| numeric("outcome count", "success overflow"))?;
        }
    }
    let interval = if independent {
        wilson(successes, eligible, 0.95)?
    } else {
        None
    };
    let required_n = match sizing {
        Some((manifest, plan)) if independent => {
            match (plan.assumed_first, plan.assumed_second, plan.family_size) {
                (Some(first), Some(second), Some(family_size)) => {
                    exact_independent_required_n(PowerContract {
                        first,
                        second,
                        alpha: manifest.alpha,
                        power: manifest.power,
                        family_size,
                        ceiling: plan.resource_ceiling,
                    })?
                }
                _ => None,
            }
        }
        _ => None,
    };
    Ok(json!({
        "key": key.id(),
        "label": key.label(),
        "status": "analyzed",
        "metric": key.metric,
        "successes": successes,
        "eligible": eligible,
        "planned": planned,
        "attempted": attempted,
        "not_attempted": not_attempted_count,
        "guard": guard,
        "timeout": timeout,
        "missing": missing,
        "infrastructure_excluded": infrastructure,
        "rate": if eligible == 0 { Value::Null } else { number(f64::from(u32::try_from(successes).map_err(|_| numeric("outcome rate", "success count too large"))?) / f64::from(u32::try_from(eligible).map_err(|_| numeric("outcome rate", "eligible count too large"))?))? },
        "wilson_ci": if independent {
            json!({"status": "independent_unpaired", "low": interval.map(|v| v.0), "high": interval.map(|v| v.1), "estimator": "wilson_score_95"})
        } else {
            json!({"status": "unavailable", "reason": "repeated or paired attempts require a registered round-aware interval"})
        },
        "required_n": {
            "status": if required_n.is_some() { "sized" } else { "unavailable" },
            "value": required_n,
            "estimator": "exact_binomial_score_test_power_at_alpha_over_K",
        },
        "ranking": if independent { "eligible_for_independent_contrast" } else { "unavailable" },
    }))
}

fn not_attempted(reason: Option<&str>) -> bool {
    reason.is_some_and(|reason| reason == "not attempted" || reason.starts_with("not attempted:"))
}

fn outcome_summaries(manifest: &Manifest, rows: &[Observation]) -> Result<Value, BenchError> {
    let plans = manifest
        .cell_plans
        .iter()
        .map(|plan| (plan.key.as_str(), plan))
        .collect::<BTreeMap<_, _>>();
    let mut groups = BTreeMap::<CellKey, Vec<Observation>>::new();
    for row in rows.iter().filter(|row| {
        matches!(
            row.metric.as_str(),
            "functional" | "contract" | "normal_completion"
        )
    }) {
        let mut key = CellKey::of(row);
        key.case = String::new();
        key.sessions = None;
        groups.entry(key).or_default().push(row.clone());
    }
    let summary_ids = rows
        .iter()
        .map(|row| {
            let full = CellKey::of(row).id();
            let mut key = CellKey::of(row);
            key.case.clear();
            key.sessions = None;
            (full, key.id())
        })
        .collect::<BTreeMap<_, _>>();
    let mut baseline_mappings = BTreeMap::new();
    let mut summaries = Map::new();
    let mut keys = BTreeMap::new();
    for (key, observations) in groups {
        let independent = observations.iter().all(|row| {
            plans
                .get(CellKey::of(row).id().as_str())
                .is_some_and(|plan| plan.independent_unpaired)
        });
        let id = key.id();
        let mapped = observations
            .iter()
            .map(|row| {
                let plan = plans.get(CellKey::of(row).id().as_str())?;
                summary_ids.get(plan.baseline_key.as_ref()?).cloned()
            })
            .collect::<Option<BTreeSet<_>>>();
        baseline_mappings.insert(id.clone(), mapped);
        let cell = outcome_cell(&key, &observations, independent, None)?;
        keys.insert(id.clone(), key);
        summaries.insert(id, cell);
    }
    let contrasts = summaries
        .iter()
        .filter(|(id, _)| keys.get(*id).is_some_and(|key| key.arm != "baseline"))
        .map(|(id, candidate)| {
            let baseline_id = baseline_mappings.get(id).and_then(Option::as_ref)
                .filter(|ids| ids.len() == 1).and_then(|ids| ids.iter().next()).cloned();
            let contrast = baseline_id
                .as_ref()
                .and_then(|baseline_id| summaries.get(baseline_id))
                .map_or_else(|| unavailable("no consistent pre-registered full-cell baseline mapping"), |baseline| {
                    let first = proportion_parts(candidate)?;
                    let second = proportion_parts(baseline)?;
                    if candidate
                        .get("wilson_ci")
                        .and_then(|value| value.get("status"))
                        .and_then(Value::as_str)
                        != Some("independent_unpaired")
                        || baseline
                            .get("wilson_ci")
                            .and_then(|value| value.get("status"))
                            .and_then(Value::as_str)
                            != Some("independent_unpaired")
                    {
                        return unavailable(
                            "repeated or paired attempts require a registered round-aware contrast",
                        );
                    }
                    let Some((difference, low, high)) = newcombe(first, second, 0.95)? else {
                        return unavailable("one arm has no eligible attempts");
                    };
                    Ok(json!({
                        "status": "independent_unpaired",
                        "estimator": "newcombe_hybrid_score_95",
                        "risk_difference": difference,
                        "low": low,
                        "high": high,
                        "baseline": baseline_id,
                    }))
                },
            )?;
            Ok((id.clone(), contrast))
        })
        .collect::<Result<Vec<_>, BenchError>>()?;
    for (id, contrast) in contrasts {
        if let Some(cell) = summaries.get_mut(&id).and_then(Value::as_object_mut) {
            cell.insert("contrast".to_owned(), contrast);
        }
    }
    Ok(Value::Object(summaries))
}

fn proportion_parts(cell: &Value) -> Result<Proportion, BenchError> {
    let successes = cell
        .get("successes")
        .and_then(Value::as_u64)
        .ok_or_else(|| numeric("outcome contrast", "successes is missing"))?;
    let eligible = cell
        .get("eligible")
        .and_then(Value::as_u64)
        .ok_or_else(|| numeric("outcome contrast", "eligible count is missing"))?;
    Ok(Proportion {
        successes,
        trials: eligible,
    })
}

fn memory_fits(manifest: &Manifest, rows: &[Observation]) -> Result<Value, BenchError> {
    let mut groups = BTreeMap::<CellKey, Vec<(f64, f64)>>::new();
    let mut seen = BTreeSet::new();
    for row in rows.iter().filter(|row| row.metric == "memory_private") {
        if let (Some(sessions), Some(value)) = (row.sessions, row.quantitative()) {
            let identity = CellKey::of(row);
            if !seen.insert((identity.clone(), row.round.clone(), row.cohort.clone())) {
                return Err(BenchError::input(
                    "validate memory cohorts",
                    identity.id(),
                    "keep one phase-aligned point per round/cohort/state/session level",
                ));
            }
            let mut key = identity;
            key.sessions = None;
            groups.entry(key).or_default().push((sessions, value));
        }
    }
    let mut output = Map::new();
    for (key, points) in groups {
        let id = key.id();
        let Some(fitted) = fit(&points) else {
            output.insert(
                id,
                unavailable("fewer than four cohorts or three session levels")?,
            );
            continue;
        };
        let mut stream = stream(manifest, &round_name(rows), &id, "memory_slope")?;
        let interval = fit_interval(
            &points,
            manifest.resamples,
            manifest.confidence,
            &mut stream,
        )?;
        let plan = manifest.cell_plans.iter().find(|plan| plan.key == id);
        let sizing_result = if let Some(plan) = plan {
            if let (Some(s0), Some(h_abs)) = (plan.s0, plan.h_abs) {
                let target = if fitted.slope.abs() <= s0 {
                    h_abs
                } else {
                    0.25 * fitted.slope.abs()
                };
                let half_width = (interval.1 - interval.0) / 2.0;
                let n = u64::try_from(points.len())
                    .map_err(|_| numeric("memory sizing", "too many cohorts"))?;
                let mut result = size_from_width(n, target, Some(half_width), plan)?;
                let object = result
                    .as_object_mut()
                    .ok_or_else(|| numeric("memory sizing", "sizing result is not an object"))?;
                object.insert("achieved_half_width".to_owned(), json!(half_width));
                object.insert("precision_met".to_owned(), json!(half_width <= target));
                object.insert(
                    "estimator".to_owned(),
                    json!("pilot_pairs_bootstrap_square_root_scaling"),
                );
                result
            } else {
                unavailable("register s0 and h_abs for the pooled memory fit")?
            }
        } else {
            unavailable("no pooled memory fit plan")?
        };
        output.insert(
            id,
            json!({
                "sizing": sizing_result,
                "status": "analyzed",
                "estimator": "ordinary_least_squares_one_point_per_cohort",
                "slope": fitted.slope,
                "intercept": fitted.intercept,
                "r_squared": fitted.r_squared,
                "slope_ci": {
                    "status": "pairs_bootstrap_percentile",
                    "low": interval.0,
                    "high": interval.1,
                    "resamples": manifest.resamples,
                "usable_replicates": interval.2,
                },
                "cohorts": points.len(),
            }),
        );
    }
    Ok(Value::Object(output))
}

fn fit_interval(
    points: &[(f64, f64)],
    resamples: u32,
    confidence: f64,
    stream: &mut SeedStream,
) -> Result<(f64, f64, usize), BenchError> {
    let mut slopes = Vec::new();
    for _ in 0..resamples {
        let mut sample = Vec::new();
        for _ in 0..points.len() {
            let index = stream.index(points.len())?;
            sample.push(
                *points
                    .get(index)
                    .ok_or_else(|| numeric("memory bootstrap", "cohort index out of bounds"))?,
            );
        }
        if let Some(line) = fit(&sample) {
            slopes.push(line.slope);
        }
    }
    slopes.sort_by(f64::total_cmp);
    if slopes.len()
        < usize::try_from(resamples)
            .map_err(|_| numeric("memory slope bootstrap", "replicate count conversion"))?
            / 2
    {
        return Err(numeric(
            "memory slope bootstrap",
            "fewer than half the replicates preserve four cohorts and three levels",
        ));
    }
    let alpha = (1.0 - confidence) / 2.0;
    Ok((
        order_quantile(&slopes, alpha)?,
        order_quantile(&slopes, 1.0 - alpha)?,
        slopes.len(),
    ))
}

fn sizing(sorted: &[f64], plan: &CellPlan, cell: &Map<String, Value>) -> Result<Value, BenchError> {
    let headline = match plan.headline {
        Headline::P50 => Quantile::P50,
        Headline::P95 => Quantile::P95,
        Headline::P99 => Quantile::P99,
    };
    let estimate = nearest_rank(sorted, headline).ok_or_else(|| numeric("sizing", "empty cell"))?;
    let target = if estimate > 0.0 {
        estimate * plan.target_half_width
    } else {
        plan.absolute_target.ok_or_else(|| {
            BenchError::input(
                "size cell",
                plan.key.clone(),
                "register absolute_target for a non-positive headline estimate",
            )
        })?
    };
    if cell
        .get(&format!("{}_ci", headline.name()))
        .and_then(|interval| interval.get("status"))
        .and_then(Value::as_str)
        == Some("iid_sensitivity")
    {
        return unavailable(
            "collection order is missing; iid sensitivity cannot size a confirmatory sample",
        );
    }
    let interval = cell
        .get(&format!("{}_ci", headline.name()))
        .filter(|interval| interval.get("status").and_then(Value::as_str) == Some("bca"))
        .and_then(Value::as_object)
        .and_then(|value| value.get("bca"))
        .and_then(Value::as_object);
    let n = u64::try_from(sorted.len()).map_err(|_| numeric("sizing", "sample count too large"))?;
    let width = interval.and_then(|interval| {
        Some((interval.get("high")?.as_f64()? - interval.get("low")?.as_f64()?) / 2.0)
    });
    size_from_width(n, target, width, plan)
}

/// Shared pilot arithmetic for registered quantile and slope interval targets.
fn size_from_width(
    n: u64,
    target: f64,
    width: Option<f64>,
    plan: &CellPlan,
) -> Result<Value, BenchError> {
    let m = plan.observations_per_cohort;
    let Some(h) = width else {
        let next = n
            .checked_add(m)
            .ok_or_else(|| numeric("sizing", "next pilot overflow"))?;
        return Ok(json!({
            "status": if next <= plan.resource_ceiling { "fresh_cohort_required" } else { "sizing_failed" },
            "required_n": if next <= plan.resource_ceiling { json!(next) } else { Value::Null },
            "reason": "BCa width is zero or undefined; collect one fresh complete cohort and retain the failed analysis",
            "estimator": "degenerate_pilot_next_cohort",
            "target_half_width": target,
        }));
    };
    if h == 0.0 {
        let next = n
            .checked_add(m)
            .ok_or_else(|| numeric("sizing", "next pilot overflow"))?;
        return Ok(
            json!({"status": if next <= plan.resource_ceiling { "fresh_cohort_required" } else { "sizing_failed" }, "required_n": if next <= plan.resource_ceiling { json!(next) } else { Value::Null }, "target_half_width": target}),
        );
    }
    let n0 = f64::from(u32::try_from(n).map_err(|_| numeric("sizing", "pilot too large"))?);
    let mf = f64::from(u32::try_from(m).map_err(|_| numeric("sizing", "cohort too large"))?);
    let cohorts = (n0 * (h / target).powi(2) / mf).ceil();
    let required = integer_at_least(cohorts, m, plan.resource_ceiling)?;
    Ok(json!({
        "status": if required <= plan.resource_ceiling { "sized" } else { "sizing_failed" },
        "required_n": required,
        "pilot_n": n,
        "observations_per_cohort": m,
        "pilot_half_width": h,
        "target_half_width": target,
        "achieved_half_width": h,
        "estimator": "pilot_bca_square_root_scaling",
    }))
}

fn integer_at_least(cohorts: f64, m: u64, ceiling: u64) -> Result<u64, BenchError> {
    let mut multiple = 1_u64;
    let mut required = m;
    while f64::from(
        u32::try_from(multiple).map_err(|_| numeric("sizing", "cohort count too large"))?,
    ) < cohorts
    {
        multiple = multiple
            .checked_add(1)
            .ok_or_else(|| numeric("sizing", "cohort count overflow"))?;
        required = required
            .checked_add(m)
            .ok_or_else(|| numeric("sizing", "sample target overflow"))?;
        if required > ceiling
            && f64::from(
                u32::try_from(multiple).map_err(|_| numeric("sizing", "cohort count too large"))?,
            ) < cohorts
        {
            return Ok(required);
        }
    }
    Ok(required)
}

fn segments(rows: &[Observation], ordered: bool) -> Vec<Vec<f64>> {
    if !ordered {
        return vec![rows.iter().filter_map(Observation::quantitative).collect()];
    }
    let mut groups = BTreeMap::<(String, String), Vec<&Observation>>::new();
    for row in rows {
        groups
            .entry((row.round.clone(), row.cohort.clone()))
            .or_default()
            .push(row);
    }
    groups
        .into_values()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.quantitative())
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .collect()
}

fn cohort_range(rows: &[Observation]) -> Result<Option<f64>, BenchError> {
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

fn interval_json(interval: Interval, confidence: f64) -> Result<Value, BenchError> {
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

fn unavailable(reason: &str) -> Result<Value, BenchError> {
    Ok(json!({"status": "unavailable", "reason": reason}))
}

fn stream(
    manifest: &Manifest,
    round: &str,
    cell: &str,
    statistic: &str,
) -> Result<SeedStream, BenchError> {
    SeedStream::new(&[&manifest.seed_string, round, cell, statistic])
}

fn round_name(rows: &[Observation]) -> String {
    let mut rounds = rows
        .iter()
        .map(|row| row.round.as_str())
        .collect::<Vec<_>>();
    rounds.sort_unstable();
    rounds.dedup();
    rounds.join("+")
}

fn number(value: f64) -> Result<Value, BenchError> {
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| numeric("export number", "non-finite estimate"))
}

fn optional_number(value: Option<f64>) -> Result<Value, BenchError> {
    value.map_or(Ok(Value::Null), number)
}

fn numbers(values: &[f64]) -> Result<Value, BenchError> {
    Ok(Value::Array(
        values
            .iter()
            .map(|value| number(*value))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn numeric(action: &str, subject: &str) -> BenchError {
    BenchError::numeric(
        action,
        subject.to_owned(),
        "inspect the cell and its pre-registered analysis",
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::decompositions;
    use crate::observation::{HEADER, parse};

    #[test]
    fn confirmatory_sizing_rejects_iid_and_honors_zero_width_ceiling() {
        let plan: crate::manifest::CellPlan = serde_json::from_value(serde_json::json!({
            "key": "test", "headline": "p50", "target_half_width": 0.25,
            "observations_per_cohort": 4, "resource_ceiling": 8, "independent_unpaired": false
        }))
        .unwrap();
        let fresh = super::size_from_width(4, 1.0, Some(0.0), &plan).unwrap();
        assert_eq!(fresh.get("required_n").unwrap(), 8);
        let failed = super::size_from_width(8, 1.0, Some(0.0), &plan).unwrap();
        assert_eq!(failed.get("status").unwrap(), "sizing_failed");
        assert!(failed.get("required_n").unwrap().is_null());
        let cell = serde_json::json!({"p50_ci": {"status": "iid_sensitivity", "bca": {"low": 1, "high": 2}}});
        let result = super::sizing(&[1.0, 2.0], &plan, cell.as_object().unwrap()).unwrap();
        assert_eq!(result.get("status").unwrap(), "unavailable");
        assert!(result.get("required_n").is_none());
    }

    #[test]
    fn wall_decomposition_unions_overlapping_request_spans() {
        let csv = format!(
            "{}\n1,w,a,,0,1,r,b,arm,agent,task_wall,work,case,c,,,wall,ms,0,1000,none,,0,1,1,,fixture,\n1,n,a,,,,r,b,arm,agent,task_requests,work,case,c,,,requests,requests,0,1,none,,0,1,1,,fixture,\n1,m,a,r,0.2,0.6,r,b,arm,agent,request_latency,work,case,c,,,model_latency,ms,0,400,none,,0,1,1,,fixture,\n1,t,a,t,0.4,0.8,r,b,arm,agent,request_latency,work,case,c,,,tool_latency,ms,0,400,none,,0,1,1,,fixture,\n",
            HEADER.join(",")
        );
        let rows = parse(csv.as_bytes()).unwrap();
        let exports = decompositions(&rows).unwrap();
        let attempt = exports.get("a").unwrap();
        assert!(
            (attempt
                .get("covered")
                .and_then(serde_json::Value::as_f64)
                .unwrap()
                - 0.6)
                .abs()
                < 1e-12
        );
        assert!(
            (attempt
                .get("uncovered")
                .and_then(serde_json::Value::as_f64)
                .unwrap()
                - 0.4)
                .abs()
                < 1e-12
        );
        assert!(
            (attempt
                .get("overlap")
                .and_then(serde_json::Value::as_f64)
                .unwrap()
                - 0.2)
                .abs()
                < 1e-12
        );
    }
}

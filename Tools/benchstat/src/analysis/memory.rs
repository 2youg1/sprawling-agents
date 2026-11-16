// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

use crate::error::BenchError;
use crate::estimators::{Quantile, fit, nearest_rank};
use crate::manifest::{CellPlan, Headline, Manifest};
use crate::observation::{CellKey, Observation};
use crate::resampling::{SeedStream, order_quantile};

use super::support::{numeric, round_name, stream, unavailable};

pub(super) fn memory_fits(manifest: &Manifest, rows: &[Observation]) -> Result<Value, BenchError> {
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

pub(super) fn fit_interval(
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

pub(super) fn sizing(
    sorted: &[f64],
    plan: &CellPlan,
    cell: &Map<String, Value>,
) -> Result<Value, BenchError> {
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
pub(super) fn size_from_width(
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

pub(super) fn integer_at_least(cohorts: f64, m: u64, ceiling: u64) -> Result<u64, BenchError> {
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

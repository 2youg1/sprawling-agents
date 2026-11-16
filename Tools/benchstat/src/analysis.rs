// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whole-round projection from validated observations to canonical export values.

mod memory;
mod outcomes;
mod quantitative;
mod support;

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::error::BenchError;
use crate::estimators::holm;
use crate::manifest::Manifest;
use crate::observation::{CellKey, Observation};

use memory::memory_fits;
use outcomes::{outcome_cell, outcome_summaries};
use quantitative::{QuantitativeContext, decompositions, quantitative_cell};
use support::{number, round_name};

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
            let evidence = rows
                .iter()
                .filter(|row| {
                    row.series == claim.series
                        && row.metric == claim.primary_metric
                        && row.unit == claim.unit
                        && row.quantitative().is_some()
                })
                .map(|row| json!({
                    "observation_id": row.observation_id,
                    "cell": CellKey::of(row).id(),
                    "value": row.value,
                }))
                .collect::<Vec<_>>();
            let reason = if !measured {
                "required coverage is not measured"
            } else if evidence.is_empty() {
                "no eligible primary metric observations in the registered unit"
            } else {
                "claim has no executable cell contrast or falsification rule; free-text conditions require explicit evaluation"
            };
            Ok(json!({
                "id": claim.id,
                "series": claim.series,
                "status": if measured { "unavailable" } else { "deferred" },
                "primary_metric": claim.primary_metric,
                "direction": claim.direction,
                "minimum_worthwhile_change": claim.minimum_worthwhile_change,
                "falsification_condition": claim.falsification_condition,
                "evidence": evidence,
                "reason": reason,
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

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::memory::{size_from_width, sizing};
    use super::quantitative::decompositions;
    use crate::observation::{HEADER, parse};

    #[test]
    fn confirmatory_sizing_rejects_iid_and_honors_zero_width_ceiling() {
        let plan: crate::manifest::CellPlan = serde_json::from_value(serde_json::json!({
            "key": "test", "headline": "p50", "target_half_width": 0.25,
            "observations_per_cohort": 4, "resource_ceiling": 8, "independent_unpaired": false
        }))
        .unwrap();
        let fresh = size_from_width(4, 1.0, Some(0.0), &plan).unwrap();
        assert_eq!(fresh.get("required_n").unwrap(), 8);
        let failed = size_from_width(8, 1.0, Some(0.0), &plan).unwrap();
        assert_eq!(failed.get("status").unwrap(), "sizing_failed");
        assert!(failed.get("required_n").unwrap().is_null());
        let cell = serde_json::json!({"p50_ci": {"status": "iid_sensitivity", "bca": {"low": 1, "high": 2}}});
        let result = sizing(&[1.0, 2.0], &plan, cell.as_object().unwrap()).unwrap();
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

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

use crate::error::BenchError;
use crate::estimators::{
    PowerContract, Proportion, exact_independent_required_n, newcombe, wilson,
};
use crate::manifest::{CellPlan, Manifest};
use crate::observation::{CellKey, FailureClass, Observation};

use super::support::{number, numeric, unavailable};

pub(super) fn outcome_cell(
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

pub(super) fn not_attempted(reason: Option<&str>) -> bool {
    reason.is_some_and(|reason| reason == "not attempted" || reason.starts_with("not attempted:"))
}

pub(super) fn outcome_summaries(
    manifest: &Manifest,
    rows: &[Observation],
) -> Result<Value, BenchError> {
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

pub(super) fn proportion_parts(cell: &Value) -> Result<Proportion, BenchError> {
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

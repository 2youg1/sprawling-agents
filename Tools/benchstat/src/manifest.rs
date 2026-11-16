// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::BenchError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignStatus {
    Required,

    Optional,

    Unsupported,
}

pub fn design_status(series: &str) -> Option<DesignStatus> {
    DESIGN_REGISTRY
        .iter()
        .find_map(|(name, status)| (*name == series).then_some(*status))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub series: String,

    pub status: String,

    pub reason: Option<String>,

    pub reopen_when: Option<String>,

    pub measured_by: Option<String>,

    pub reading: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub id: String,

    pub series: String,

    pub primary_metric: String,

    pub unit: String,

    pub direction: String,

    pub minimum_worthwhile_change: f64,

    pub multiplicity_family: String,

    pub falsification_condition: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Headline {
    P50,

    P95,

    P99,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellPlan {
    pub key: String,

    #[serde(default)]
    pub baseline_key: Option<String>,

    pub headline: Headline,

    pub target_half_width: f64,

    pub observations_per_cohort: u64,

    pub resource_ceiling: u64,

    pub independent_unpaired: bool,

    #[serde(default)]
    pub absolute_target: Option<f64>,

    #[serde(default)]
    pub s0: Option<f64>,

    #[serde(default)]
    pub h_abs: Option<f64>,

    #[serde(default)]
    pub assumed_first: Option<f64>,

    #[serde(default)]
    pub assumed_second: Option<f64>,

    #[serde(default)]
    pub family_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u64,

    pub analysis_revision: String,

    pub method_version: String,

    pub estimator_version: String,

    pub seed_string: String,

    pub resamples: u32,

    pub confidence: f64,

    pub alpha: f64,

    pub power: f64,

    pub bootstrap_unit: String,

    pub input_hashes: BTreeMap<String, String>,

    pub coverage: Vec<Coverage>,

    pub claims: Vec<Claim>,

    pub cell_plans: Vec<CellPlan>,

    #[serde(flatten)]
    pub provenance: BTreeMap<String, serde_json::Value>,
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, BenchError> {
        let manifest: Self = serde_json::from_str(text).map_err(|error| {
            BenchError::input(
                "parse manifest.json",
                error.to_string(),
                "fix the JSON field named by the parser",
            )
        })?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn validate(&self) -> Result<(), BenchError> {
        if self.schema_version != 1 {
            return Err(invalid("schema_version", "write schema_version = 1"));
        }
        if self.analysis_revision.len() != 40
            || !self
                .analysis_revision
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(invalid(
                "analysis_revision",
                "write the analyzer's full 40-character lowercase Git SHA",
            ));
        }
        if self.seed_string.is_empty() {
            return Err(invalid("seed_string", "pre-register a non-empty seed"));
        }
        if self.resamples != 10_000 {
            return Err(invalid("resamples", "write the method's 10000 replicates"));
        }
        if self.confidence != 0.95 || self.alpha != 0.05 || self.power != 0.8 {
            return Err(invalid(
                "confidence/alpha/power",
                "write 0.95, 0.05 and 0.8 respectively",
            ));
        }
        if self.bootstrap_unit != "cohort" {
            return Err(invalid(
                "bootstrap_unit",
                "this analyzer preserves cohort and round units",
            ));
        }
        let digest = self.input_hashes.get("results.csv").ok_or_else(|| {
            invalid(
                "input_hashes.results.csv",
                "pin the published CSV's SHA-256",
            )
        })?;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(invalid(
                "input_hashes.results.csv",
                "write 64 lowercase hexadecimal characters",
            ));
        }
        self.validate_coverage()?;
        self.validate_plans()
    }

    fn validate_coverage(&self) -> Result<(), BenchError> {
        let mut seen = BTreeSet::new();
        for row in &self.coverage {
            let status = design_status(&row.series).ok_or_else(|| {
                invalid(
                    &format!("coverage series {}", row.series),
                    "use a series from the design registry",
                )
            })?;
            if status == DesignStatus::Unsupported {
                return Err(invalid(
                    &format!("coverage series {}", row.series),
                    "unsupported belongs only to the design registry",
                ));
            }
            if !seen.insert(row.series.clone()) {
                return Err(invalid(
                    &format!("duplicate coverage series {}", row.series),
                    "keep one coverage row per series",
                ));
            }
            match row.status.as_str() {
                "measured" => {
                    require_pair(
                        row.measured_by.as_deref(),
                        row.reading.as_deref(),
                        &row.series,
                    )?;
                    if row.reason.is_some() || row.reopen_when.is_some() {
                        return Err(invalid(
                            &format!("measured coverage series {}", row.series),
                            "measured keeps reason and reopen_when null",
                        ));
                    }
                }
                "deferred" | "failed" => {
                    require_pair(
                        row.reason.as_deref(),
                        row.reopen_when.as_deref(),
                        &row.series,
                    )?;
                    if row.measured_by.is_some() {
                        return Err(invalid(
                            &format!("{} coverage series {}", row.status, row.series),
                            "deferred/failed keeps measured_by null; retain a partial reading only",
                        ));
                    }
                }
                _ => {
                    return Err(invalid(
                        &format!("coverage status {}", row.status),
                        "use measured, deferred or failed",
                    ));
                }
            }
        }
        for (series, status) in DESIGN_REGISTRY {
            if status == &DesignStatus::Required && !seen.contains(*series) {
                return Err(invalid(
                    &format!("missing required coverage series {series}"),
                    "add a measured, deferred or failed row with its explanation",
                ));
            }
        }
        for claim in &self.claims {
            if !self.coverage.iter().any(|row| row.series == claim.series) {
                return Err(invalid(
                    &format!("claim {} series {}", claim.id, claim.series),
                    "add the claim's required series to coverage",
                ));
            }
        }
        Ok(())
    }

    fn validate_plans(&self) -> Result<(), BenchError> {
        let mut keys = BTreeSet::new();
        for plan in &self.cell_plans {
            if !keys.insert(plan.key.clone()) {
                return Err(invalid(
                    &format!("duplicate cell plan {}", plan.key),
                    "keep one plan per cell key",
                ));
            }
            if plan.s0.is_some() != plan.h_abs.is_some()
                || plan.s0.is_some_and(|v| !v.is_finite() || v <= 0.0)
                || plan.h_abs.is_some_and(|v| !v.is_finite() || v <= 0.0)
            {
                return Err(invalid(
                    &plan.key,
                    "register positive finite s0 and h_abs together",
                ));
            }
            if plan.target_half_width <= 0.0
                || !plan.target_half_width.is_finite()
                || plan.observations_per_cohort == 0
                || plan.observations_per_cohort > 1_000_000
                || plan.resource_ceiling < plan.observations_per_cohort
                || plan.resource_ceiling > 1_000_000
            {
                return Err(invalid(
                    &format!("cell plan {}", plan.key),
                    "require positive finite width, a non-empty cohort and a sufficient ceiling",
                ));
            }
        }
        let outcome_fields = self
            .cell_plans
            .iter()
            .filter(|plan| {
                plan.assumed_first.is_some()
                    || plan.assumed_second.is_some()
                    || plan.family_size.is_some()
            })
            .collect::<Vec<_>>();
        for plan in outcome_fields {
            let (Some(first), Some(second), Some(family_size)) =
                (plan.assumed_first, plan.assumed_second, plan.family_size)
            else {
                return Err(invalid(
                    &format!("outcome sizing for {}", plan.key),
                    "register assumed_first, assumed_second and family_size together",
                ));
            };
            if !(0.0..=1.0).contains(&first) || !(0.0..=1.0).contains(&second) || family_size == 0 {
                return Err(invalid(
                    &format!("outcome sizing for {}", plan.key),
                    "use proportions in [0,1] and a positive family size",
                ));
            }
        }
        Ok(())
    }
}

const DESIGN_REGISTRY: &[(&str, DesignStatus)] = &[
    ("startup", DesignStatus::Required),
    ("first_pairing", DesignStatus::Required),
    ("cold_boot_gui", DesignStatus::Required),
    ("browser_reload", DesignStatus::Optional),
    ("local_tool", DesignStatus::Required),
    ("request_latency", DesignStatus::Required),
    ("task_wall", DesignStatus::Required),
    ("task_requests", DesignStatus::Required),
    ("task_tokens", DesignStatus::Required),
    ("task_outcomes", DesignStatus::Required),
    ("session_memory", DesignStatus::Required),
    ("sustained_memory", DesignStatus::Required),
    ("real_model_capacity", DesignStatus::Required),
    ("task_io", DesignStatus::Optional),
    ("monitor_beat", DesignStatus::Optional),
    ("view_backlog", DesignStatus::Optional),
];

fn require_pair(first: Option<&str>, second: Option<&str>, series: &str) -> Result<(), BenchError> {
    if first.is_some_and(|text| !text.is_empty()) && second.is_some_and(|text| !text.is_empty()) {
        Ok(())
    } else {
        Err(invalid(
            &format!("coverage fields for {series}"),
            "measured needs measured_by and reading; deferred/failed needs reason and reopen_when",
        ))
    }
}

fn invalid(subject: &str, recovery: &str) -> BenchError {
    BenchError::input("validate manifest.json", subject.to_owned(), recovery)
}

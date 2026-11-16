// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Round configuration and the executable design registry.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::BenchError;

/// One design-registry status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignStatus {
    /// Every round measures or explains the series.
    Required,
    /// A claim decides whether the series is measured.
    Optional,
    /// The instrument cannot observe it in principle.
    Unsupported,
}

/// The complete series boundary, shared with methodology.md's registry.
pub fn design_status(series: &str) -> Option<DesignStatus> {
    DESIGN_REGISTRY
        .iter()
        .find_map(|(name, status)| (*name == series).then_some(*status))
}

/// One round coverage row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    /// Registry name.
    pub series: String,
    /// Round outcome.
    pub status: String,
    /// Why measurement was deferred or failed.
    pub reason: Option<String>,
    /// Observable condition that reopens measurement.
    pub reopen_when: Option<String>,
    /// Instrument used for a measured row.
    pub measured_by: Option<String>,
    /// Raw source used for a measured or partially failed row.
    pub reading: Option<String>,
}

/// One pre-registered claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    /// Stable claim id.
    pub id: String,
    /// Series whose readings the claim needs.
    pub series: String,
    /// Primary raw metric.
    pub primary_metric: String,
    /// Expected unit.
    pub unit: String,
    /// Beneficial direction.
    pub direction: String,
    /// Editorial minimum effect.
    pub minimum_worthwhile_change: f64,
    /// Pre-registered multiplicity family.
    pub multiplicity_family: String,
    /// Observation that would refute the claim.
    pub falsification_condition: String,
}

/// Quantile used by a cell's sizing rule.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Headline {
    /// Median.
    P50,
    /// 95th percentile.
    P95,
    /// 99th percentile with a P95 fallback.
    P99,
}

/// One cell's pre-registered sizing and independence contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellPlan {
    /// The length-prefixed cell identity emitted by `CellKey::id`.
    pub key: String,
    /// Explicit full baseline cell identity for outcome contrasts.
    #[serde(default)]
    pub baseline_key: Option<String>,
    /// Quantile whose interval is sized.
    pub headline: Headline,
    /// Positive relative or absolute width target.
    pub target_half_width: f64,
    /// Observations in one fresh cohort.
    pub observations_per_cohort: u64,
    /// Maximum planned observations.
    pub resource_ceiling: u64,
    /// Whether independent-trial outcome formulas are admissible.
    pub independent_unpaired: bool,
    /// Absolute target for a non-positive estimate.
    #[serde(default)]
    pub absolute_target: Option<f64>,
    /// Near-zero memory-slope threshold.
    #[serde(default)]
    pub s0: Option<f64>,
    /// Absolute memory-slope width target.
    #[serde(default)]
    pub h_abs: Option<f64>,
    /// Assumed first-arm proportion for independent outcome sizing.
    #[serde(default)]
    pub assumed_first: Option<f64>,
    /// Assumed second-arm proportion for independent outcome sizing.
    #[serde(default)]
    pub assumed_second: Option<f64>,
    /// Multiplicity family size for conservative alpha allocation.
    #[serde(default)]
    pub family_size: Option<u32>,
}

/// The complete manifest shape used by the analyzer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Manifest schema version, currently one.
    pub schema_version: u64,
    /// Immutable analyzer commit.
    pub analysis_revision: String,
    /// Method version.
    pub method_version: String,
    /// Estimator implementation version.
    pub estimator_version: String,
    /// Root of every deterministic random stream.
    pub seed_string: String,
    /// Resamples per analysis.
    pub resamples: u32,
    /// Two-sided interval confidence.
    pub confidence: f64,
    /// Two-sided family alpha.
    pub alpha: f64,
    /// Sizing power.
    pub power: f64,
    /// Independent resampling unit.
    pub bootstrap_unit: String,
    /// SHA-256 of each input, keyed by file name.
    pub input_hashes: BTreeMap<String, String>,
    /// Round coverage rows.
    pub coverage: Vec<Coverage>,
    /// Pre-registered claims.
    pub claims: Vec<Claim>,
    /// Per-cell sizing plans.
    pub cell_plans: Vec<CellPlan>,
    /// Toolchain, machine, workload and other provenance not read by estimators.
    #[serde(flatten)]
    pub provenance: BTreeMap<String, serde_json::Value>,
}

impl Manifest {
    /// Decode and validate a manifest without touching the filesystem.
    ///
    /// # Errors
    /// Returns `E_INVALID_INPUT` for every schema or coverage violation.
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

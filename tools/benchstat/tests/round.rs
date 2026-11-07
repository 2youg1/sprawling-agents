// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

const HEADER: &str = "schema_version,observation_id,task_attempt_id,request_id,span_start,span_end,round,build,arm,agent,series,workload,case,cohort,state,sessions,metric,unit,index,value,failure_class,missing_reason,censored,usage_complete,selected,in_flight,counter_source,condition";
const REQUIRED: [&str; 12] = [
    "startup",
    "first_pairing",
    "cold_boot_gui",
    "local_tool",
    "request_latency",
    "task_wall",
    "task_requests",
    "task_tokens",
    "task_outcomes",
    "session_memory",
    "sustained_memory",
    "real_model_capacity",
];

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let mut csv = format!("{HEADER}\n");
    for (index, value) in [1.0, 2.0, 3.0, 100.0].iter().enumerate() {
        writeln!(
            csv,
            "1,o{index},,,,,r1,b1,baseline,sprawling,startup,smoke,help,c1,,,latency,ms,{index},{value},none,,0,1,1,,fixture,"
        )
        .unwrap();
    }
    std::fs::write(dir.path().join("results.csv"), &csv).unwrap();
    let digest = Sha256::digest(csv.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut coverage = String::new();
    for series in REQUIRED {
        if series == "startup" {
            coverage.push_str(&format!(
                "{{\"series\":\"{series}\",\"status\":\"measured\",\"reason\":null,\"reopen_when\":null,\"measured_by\":\"fixture\",\"reading\":\"results.csv\"}},"
            ));
        } else {
            coverage.push_str(&format!(
                "{{\"series\":\"{series}\",\"status\":\"deferred\",\"reason\":\"not exercised by this smoke fixture\",\"reopen_when\":\"a real round collects it\",\"measured_by\":null,\"reading\":null}},"
            ));
        }
    }
    coverage.pop();
    let manifest = format!(
        "{{\"schema_version\":1,\"analysis_revision\":\"0123456789abcdef0123456789abcdef01234567\",\"method_version\":\"1\",\"estimator_version\":\"1\",\"seed_string\":\"fixture-seed\",\"resamples\":10000,\"confidence\":0.95,\"alpha\":0.05,\"power\":0.8,\"bootstrap_unit\":\"cohort\",\"input_hashes\":{{\"results.csv\":\"{digest}\"}},\"coverage\":[{coverage}],\"claims\":[],\"cell_plans\":[]}}"
    );
    std::fs::write(dir.path().join("manifest.json"), manifest).unwrap();
    std::fs::write(
        dir.path().join("README.md"),
        "# Smoke\n\n<!-- benchstat:begin -->\nold\n<!-- benchstat:end -->\n",
    )
    .unwrap();
    dir
}

#[test]
fn a_round_exports_nearest_rank_statistics_and_rewrites_markdown() {
    let dir = fixture();
    let computed = benchstat::export(dir.path()).unwrap();
    let exports: serde_json::Value = serde_json::from_str(&computed.exports).unwrap();
    let cells = exports
        .get("cells")
        .and_then(serde_json::Value::as_object)
        .unwrap();
    let cell = cells.values().next().unwrap();
    assert_eq!(cell.get("n").and_then(serde_json::Value::as_u64), Some(4));
    assert_eq!(
        cell.get("p50").and_then(serde_json::Value::as_f64),
        Some(2.0)
    );
    assert_eq!(
        cell.get("p95").and_then(serde_json::Value::as_f64),
        Some(100.0)
    );
    assert_eq!(
        cell.get("max").and_then(serde_json::Value::as_f64),
        Some(100.0)
    );
    assert!(computed.markdown.contains("deferred"));

    benchstat::export_write(dir.path()).unwrap();
    benchstat::verify(dir.path()).unwrap();
    let readme = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
    assert!(!readme.contains("\nold\n"));

    let tampered = readme.replace("P50 (interval status)", "P50 (interval status) 0");
    std::fs::write(dir.path().join("README.md"), tampered).unwrap();
    let error = benchstat::verify(dir.path()).unwrap_err();
    assert_eq!(error.code, "E_VERIFY_MISMATCH");
}

fn inputs(dir: &std::path::Path, csv: &str, update: impl FnOnce(&mut serde_json::Value)) {
    std::fs::write(dir.join("results.csv"), csv).unwrap();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
    *manifest
        .get_mut("input_hashes")
        .unwrap()
        .get_mut("results.csv")
        .unwrap() = serde_json::json!(
        Sha256::digest(csv.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    update(&mut manifest);
    std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
}

#[test]
fn multi_arm_report_identifies_each_build_and_statistics() {
    let dir = fixture();
    let csv = std::fs::read_to_string(dir.path().join("results.csv")).unwrap();
    let mut combined = csv.clone();
    for line in csv.lines().skip(1) {
        combined.push_str(
            &line
                .replace(",o", ",candidate-o")
                .replace(",b1,baseline,", ",b2,candidate,")
                .replace(",100,none", ",200,none"),
        );
        combined.push('\n');
    }
    inputs(dir.path(), &combined, |_| {});
    let computed = benchstat::export(dir.path()).unwrap();
    let exports: serde_json::Value = serde_json::from_str(&computed.exports).unwrap();
    let cells = exports.get("cells").unwrap().as_object().unwrap();
    assert_eq!(cells.len(), 2);
    assert!(cells.values().any(|cell| {
        cell.get("label")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("baseline")
    }));
    assert!(cells.values().any(|cell| {
        cell.get("label")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("candidate")
    }));
    for cell in cells.values() {
        let label = cell.get("label").unwrap().as_str().unwrap();
        let line = computed
            .markdown
            .lines()
            .find(|line| line.contains(label.replace('|', "\\|").as_str()))
            .unwrap();
        if label.contains("candidate") {
            assert!(line.contains("200"));
        } else {
            assert!(line.contains("100"));
        }
    }
}

#[test]
fn measured_coverage_does_not_evaluate_a_claim_without_observations() {
    let dir = fixture();
    let csv = std::fs::read_to_string(dir.path().join("results.csv")).unwrap();
    inputs(dir.path(), &csv, |manifest| {
        *manifest.get_mut("claims").unwrap() = serde_json::json!([{
            "id": "missing", "series": "startup", "primary_metric": "other", "unit": "ms",
            "direction": "lower", "minimum_worthwhile_change": 1.0,
            "multiplicity_family": "startup", "falsification_condition": "candidate is slower"
        }]);
    });
    let exports: serde_json::Value =
        serde_json::from_str(&benchstat::export(dir.path()).unwrap().exports).unwrap();
    assert_eq!(
        exports
            .get("claims")
            .unwrap()
            .get(0)
            .unwrap()
            .get("status")
            .unwrap(),
        "unavailable"
    );
    assert!(
        exports
            .get("claims")
            .unwrap()
            .get(0)
            .unwrap()
            .get("reason")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("no eligible")
    );
}

#[test]
fn incomplete_usage_excludes_only_token_totals() {
    let csv = format!(
        "{HEADER}\n1,t,a,,,,r,b,arm,agent,task_tokens,work,case,c,,,input_tokens,tokens,0,123,none,,0,0,1,,fixture,\n1,n,a,,,,r,b,arm,agent,task_requests,work,case,c,,,requests,requests,0,2,none,,0,0,1,,fixture,\n"
    );
    let rows = benchstat::observation::parse(csv.as_bytes()).unwrap();
    assert_eq!(rows.first().unwrap().quantitative(), None);
    assert_eq!(
        rows.first().unwrap().effective_missing_reason(),
        Some("token usage is incomplete")
    );
    assert_eq!(rows.get(1).unwrap().quantitative(), Some(2.0));
}

#[test]
fn cross_build_outcome_mapping_preserves_independence_checks() {
    let dir = fixture();
    let mut manifest: benchstat::manifest::Manifest =
        serde_json::from_slice(&std::fs::read(dir.path().join("manifest.json")).unwrap()).unwrap();
    let csv = format!(
        "{HEADER}\n1,b,a,,,,r,old,baseline,agent,task_outcomes,w,c,c,,,functional,ratio,0,0,capability,,0,1,1,,fixture,\n1,c,z,,,,r,new,candidate,agent,task_outcomes,w,c,c,,,functional,ratio,0,1,none,,0,1,1,,fixture,\n"
    );
    let rows = benchstat::observation::parse(csv.as_bytes()).unwrap();
    let base = benchstat::observation::CellKey::of(rows.first().unwrap()).id();
    manifest.cell_plans = rows.iter().map(|row| serde_json::from_value(serde_json::json!({
        "key": benchstat::observation::CellKey::of(row).id(), "headline": "p50", "target_half_width": 0.25,
        "observations_per_cohort": 1, "resource_ceiling": 100, "independent_unpaired": true,
        "baseline_key": base
    })).unwrap()).collect();
    let result = benchstat::analysis::analyze(&manifest, &rows).unwrap();
    let candidate = result
        .get("outcome_summaries")
        .unwrap()
        .as_object()
        .unwrap()
        .values()
        .find(|cell| {
            cell.get("label")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("candidate")
        })
        .unwrap();
    assert_eq!(
        candidate
            .get("contrast")
            .unwrap()
            .get("risk_difference")
            .unwrap(),
        1.0
    );
    manifest
        .cell_plans
        .first_mut()
        .unwrap()
        .independent_unpaired = false;
    let result = benchstat::analysis::analyze(&manifest, &rows).unwrap();
    let candidate = result
        .get("outcome_summaries")
        .unwrap()
        .as_object()
        .unwrap()
        .values()
        .find(|cell| {
            cell.get("label")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("candidate")
        })
        .unwrap();
    assert_eq!(
        candidate.get("contrast").unwrap().get("status").unwrap(),
        "unavailable"
    );
}

#[test]
fn pooled_memory_sizing_exports_precision_misses() {
    let dir = fixture();
    let mut manifest: benchstat::manifest::Manifest =
        serde_json::from_slice(&std::fs::read(dir.path().join("manifest.json")).unwrap()).unwrap();
    let mut csv = format!("{HEADER}\n");
    for (index, (sessions, value)) in [(1, 10), (2, 40), (3, 20), (4, 60), (5, 30), (6, 80)]
        .iter()
        .enumerate()
    {
        writeln!(csv, "1,m{index},,,,,r,b,arm,agent,session_memory,w,c,c{index},idle,{sessions},memory_private,MiB,{index},{value},none,,0,1,1,,fixture,").unwrap();
    }
    let rows = benchstat::observation::parse(csv.as_bytes()).unwrap();
    let mut key = benchstat::observation::CellKey::of(rows.first().unwrap());
    key.sessions = None;
    manifest.cell_plans =
        vec![serde_json::from_value(serde_json::json!({
        "key": key.id(), "headline": "p50", "target_half_width": 0.25,
        "observations_per_cohort": 1, "resource_ceiling": 100, "independent_unpaired": false,
        "s0": 100, "h_abs": 0.001
    })).unwrap()];
    let result = benchstat::analysis::analyze(&manifest, &rows).unwrap();
    let sizing = &result
        .get("memory_fits")
        .unwrap()
        .get(key.id())
        .unwrap()
        .get("sizing")
        .unwrap();
    assert_eq!(sizing.get("target_half_width").unwrap(), 0.001);
    assert_eq!(sizing.get("precision_met").unwrap(), false);
    assert_eq!(sizing.get("status").unwrap(), "sizing_failed");
    assert!(sizing.get("achieved_half_width").unwrap().as_f64().unwrap() > 0.001);
}

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

    let tampered = readme.replace("P50 (95% BCa)", "P50 (95% BCa) 0");
    std::fs::write(dir.path().join("README.md"), tampered).unwrap();
    let error = benchstat::verify(dir.path()).unwrap_err();
    assert_eq!(error.code, "E_VERIFY_MISMATCH");
}

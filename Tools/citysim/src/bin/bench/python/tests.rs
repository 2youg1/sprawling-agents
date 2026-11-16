// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::concurrent::keep_both;
use super::reporting::{CSV_HEADER, csv_cleanup_error, publish_csv};
use super::*;

#[test]
fn stdlib_digest_uses_registered_encoding_and_detects_changes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("a")).unwrap();
    std::fs::write(dir.path().join("z"), b"last").unwrap();
    std::fs::write(dir.path().join("a/b"), b"first").unwrap();
    let mut canonical = b"a/b\0".to_vec();
    canonical.extend_from_slice(&5_u64.to_le_bytes());
    canonical.extend_from_slice(b"firstz\0");
    canonical.extend_from_slice(&4_u64.to_le_bytes());
    canonical.extend_from_slice(b"last");
    let before = stdlib_digest(dir.path()).unwrap();
    assert_eq!(before, B3Hash::digest(&canonical));
    std::fs::write(dir.path().join("a/b"), b"other").unwrap();
    assert_ne!(before, stdlib_digest(dir.path()).unwrap());
    std::fs::write(dir.path().join("a/b"), b"first").unwrap();
    assert_eq!(before, stdlib_digest(dir.path()).unwrap());
    std::fs::rename(dir.path().join("z"), dir.path().join("y")).unwrap();
    assert_ne!(before, stdlib_digest(dir.path()).unwrap());
}

#[test]
fn measured_output_failures_carry_a_code_action_and_recovery() {
    for (exit, stdout, stderr) in [
        (
            SandboxExit::Failure { code: 1 },
            EXPECTED.to_vec(),
            Vec::new(),
        ),
        (SandboxExit::Success, b"wrong output".to_vec(), Vec::new()),
        (SandboxExit::Success, EXPECTED.to_vec(), b"warning".to_vec()),
    ] {
        let outcome = runtime::SandboxOutcome {
            exit,
            stdout,
            stderr,
            ceiling: None,
        };
        let refused = checked_output(&outcome).expect_err("invalid output was accepted");
        assert!(refused.contains("E_INVALID_ARGS"), "{refused}");
        assert!(
            refused.contains("measure the registered CPython-WASI scenario"),
            "{refused}"
        );
        assert!(
            refused.contains("check the selected guest and the fixed script"),
            "{refused}"
        );
        assert!(!refused.contains("E_TOOL_UNAVAILABLE"), "{refused}");
    }
}

#[test]
fn a_missing_guest_is_a_prerequisite_error() {
    let refused = measure_from(None, Arm::Baseline).expect_err("a missing guest produced a report");
    assert!(refused.contains("E_TOOL_UNAVAILABLE"), "{refused}");
    assert!(
        refused.contains("configure a real CPython-WASI guest"),
        "{refused}"
    );
    assert!(!refused.contains("E_INVALID_ARGS"), "{refused}");
}

#[test]
fn an_invalid_guest_produces_no_reading() {
    let dir = tempfile::tempdir().unwrap();
    let wasm = dir.path().join("invalid.wasm");
    std::fs::write(&wasm, b"not WebAssembly").unwrap();
    let refused = measure_at(&wasm).expect_err("an invalid guest produced a report");
    assert!(refused.contains("E_TOOL_UNAVAILABLE"), "{refused}");
    assert!(
        refused.contains("configure a real CPython-WASI guest"),
        "{refused}"
    );
    assert!(!refused.contains("E_INVALID_ARGS"), "{refused}");
}

#[test]
fn a_correctness_module_is_not_cpython_evidence() {
    // `(module (func (export "_start")))`: valid enough to run, but
    // it has no CPython implementation or benchmark output.
    const EMPTY_START: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00, 0x00, 0x03,
        0x02, 0x01, 0x00, 0x07, 0x0a, 0x01, 0x06, b'_', b's', b't', b'a', b'r', b't', 0x00, 0x00,
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
    ];
    let dir = tempfile::tempdir().unwrap();
    let wasm = dir.path().join("correctness.wasm");
    std::fs::write(&wasm, EMPTY_START).unwrap();
    std::fs::create_dir(dir.path().join("lib")).unwrap();
    let refused = measure_at(&wasm).expect_err("a WAT fixture produced a CPython report");
    assert!(refused.contains("E_TOOL_UNAVAILABLE"), "{refused}");
    assert!(!refused.contains("E_INVALID_ARGS"), "{refused}");
    assert!(refused.contains("unexpected stdout"), "{refused}");
    assert!(
        refused.contains("configure a real CPython-WASI guest"),
        "{refused}"
    );
}

#[test]
fn the_csv_report_keeps_raw_observations_in_long_form() {
    let guest = B3Hash::digest(b"guest");
    let script = B3Hash::digest(b"script");
    let build = B3Hash::digest(b"build");
    let identity = Identity {
        build,
        guest,
        script,
        arm: Arm::Candidate,
    };
    let report = Report {
        lines: Vec::new(),
        observations: vec![Observation {
            row: "python_warm_call".to_owned(),
            index: 1,
            round: 2,
            metric: "latency",
            value: 17,
            unit: "us",
            identity,
        }],
        arm: Arm::Candidate,
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.csv");
    report.write_csv_at(&path).unwrap();
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        format!(
            "{CSV_HEADER}1,{build},python_warm_call,1,2,1,candidate,latency,17,us,{guest},{script},general,{},{}\n",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    );
}

#[test]
fn csv_publication_preserves_an_existing_target_and_removes_the_partial_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.csv");
    std::fs::write(&path, b"previous measurement").unwrap();
    let part = path.with_extension("csv.part");
    let refused = Report::new(Arm::Baseline)
        .write_csv_at(&path)
        .expect_err("an existing target was replaced");
    assert!(refused.contains("E_STORAGE_FATAL"), "{refused}");
    assert!(refused.contains(&path.display().to_string()), "{refused}");
    assert!(
        refused.contains("choose a writable target directory"),
        "{refused}"
    );
    assert_eq!(std::fs::read(path).unwrap(), b"previous measurement");
    assert!(!part.exists(), "the partial publication survived");
}

#[test]
fn csv_write_failure_names_the_partial_path_and_keeps_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.csv");
    let part = path.with_extension("csv.part");
    std::fs::write(&part, b"partial").unwrap();
    let published = publish_csv(
        Err(std::io::Error::other("injected write or sync failure")),
        &part,
        &path,
    );
    let cleanup = std::fs::remove_file(&part)
        .map_err(|why| csv_cleanup_error(&path, &part, published.is_ok(), why));
    let refused = keep_both(published, cleanup).unwrap_err();
    assert!(refused.contains("E_STORAGE_FATAL"), "{refused}");
    assert!(
        refused.contains(&format!("{}: injected write", part.display())),
        "{refused}"
    );
    assert!(
        refused.contains("choose a writable target directory"),
        "{refused}"
    );
    assert!(!part.exists());
    assert!(!path.exists());
}

#[test]
fn csv_cleanup_failure_keeps_the_published_and_residual_paths() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("observations.csv");
    let part = path.with_extension("csv.part");
    std::fs::write(&part, b"measurement").unwrap();
    let published = publish_csv(Ok(()), &part, &path);
    assert!(published.is_ok());
    let cleanup = Err::<(), _>(csv_cleanup_error(
        &path,
        &part,
        published.is_ok(),
        std::io::Error::other("injected cleanup failure"),
    ));
    let refused = keep_both(published, cleanup).unwrap_err();
    assert!(refused.contains("E_STORAGE_FATAL"), "{refused}");
    assert!(
        refused.contains(&format!("CSV published at {}", path.display())),
        "{refused}"
    );
    assert!(
        refused.contains(&format!("temporary file {} remains", part.display())),
        "{refused}"
    );
    assert!(
        refused.contains("delete only the residual .part file"),
        "{refused}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"measurement");
    assert_eq!(std::fs::read(&part).unwrap(), b"measurement");
}

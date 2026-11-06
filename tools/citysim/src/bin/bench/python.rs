// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Real CPython-WASI execution readings; WAT correctness fixtures are
//! never performance substitutes (`tools/citysim/spec/Bench.lean` §8-6-1).

use std::path::{Path, PathBuf};

/// Python readings are returned only after the whole family succeeds.
#[derive(Debug)]
pub(crate) struct Report {
    pub(crate) lines: Vec<String>,
}

/// Measures the external guest selected by the production doctor door.
pub(crate) fn measure() -> Result<Report, String> {
    measure_from(sprawling::doctor::usable_python_wasm())
}

/// Runs one cold child through the same guest-selection door.
pub(crate) fn cold_child() -> Result<(), String> {
    let _report = measure()?;
    Ok(())
}

fn measure_from(_wasm: Option<PathBuf>) -> Result<Report, String> {
    Ok(Report { lines: Vec::new() })
}

#[allow(dead_code, reason = "the red acceptance fixture names the implementation boundary")]
fn measure_at(_wasm: &Path) -> Result<Report, String> {
    measure_from(Some(_wasm.to_path_buf()))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_guest_is_a_prerequisite_error() {
        let refused = measure_from(None).expect_err("a missing guest produced a report");
        assert!(refused.contains("E_TOOL_UNAVAILABLE"), "{refused}");
    }

    #[test]
    fn an_invalid_guest_produces_no_reading() {
        let dir = tempfile::tempdir().unwrap();
        let wasm = dir.path().join("invalid.wasm");
        std::fs::write(&wasm, b"not WebAssembly").unwrap();
        let refused = measure_at(&wasm).expect_err("an invalid guest produced a report");
        assert!(refused.contains("E_TOOL_UNAVAILABLE"), "{refused}");
    }

    #[test]
    fn a_correctness_module_is_not_cpython_evidence() {
        // `(module (func (export "_start")))`: valid enough to run, but
        // it has no CPython implementation or benchmark output.
        const EMPTY_START: &[u8] = &[
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x04, 0x01, 0x60, 0x00,
            0x00, 0x03, 0x02, 0x01, 0x00, 0x07, 0x0a, 0x01, 0x06, b'_', b's', b't',
            b'a', b'r', b't', 0x00, 0x00, 0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b,
        ];
        let dir = tempfile::tempdir().unwrap();
        let wasm = dir.path().join("correctness.wasm");
        std::fs::write(&wasm, EMPTY_START).unwrap();
        let refused = measure_at(&wasm).expect_err("a WAT fixture produced a CPython report");
        assert!(refused.contains("CPython"), "{refused}");
    }
}
